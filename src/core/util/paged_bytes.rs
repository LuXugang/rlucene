/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate::core::index::BytesRef;
use crate::core::store::{DataInput, DataOutput, IndexInput};
use crate::core::util::SliceCopyOps;
use crate::core::util::accountable::Accountable;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::group_vint_util::GroupVIntUtil;
use crate::core::util::ram_usage_estimator::size_of_vec;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::mem;
use std::rc::Rc;
use std::sync::Arc;

/// Represents a logical byte buffer as a series of pages. You can write once into
/// the logical buffer (append only), using copy, and then retrieve slices
/// (BytesRef) into it using fill.
#[derive(Default)]
pub struct PagedBytes {
  blocks: Vec<Vec<u8>>,
  frozen_blocks: Option<Vec<Arc<Vec<u8>>>>,
  num_blocks: usize,
  block_size: usize,
  block_bits: usize,
  block_mask: usize,
  did_skip_bytes: bool,
  frozen: bool,
  upto: usize,
  current_block: Option<Vec<u8>>,
  bytes_used_per_block: i64,
}
impl PagedBytes {
  pub fn new(block_bits: usize) -> Self {
    debug_assert!(
      block_bits > 0 && block_bits <= 31,
      "blockBits: {block_bits}"
    );
    let block_size = 1 << block_bits;
    let block_mask = block_size - 1;
    let upto = block_size;
    let bytes_used_per_block = block_size as i64;

    PagedBytes {
      blocks: Vec::with_capacity(16),
      frozen_blocks: None,
      num_blocks: 0,
      block_size,
      block_bits,
      block_mask,
      did_skip_bytes: false,
      frozen: false,
      upto,
      current_block: None,
      bytes_used_per_block,
    }
  }
  fn add_block(&mut self, block: Vec<u8>) -> Result<()> {
    ArrayUtil::grow_with_len(&mut self.blocks, self.num_blocks + 1)?;
    self.blocks[self.num_blocks] = block;
    self.num_blocks += 1;
    Ok(())
  }
  /// Read this many bytes from in
  pub fn copy_with_input(
    &mut self,
    input: &mut impl IndexInput,
    mut byte_count: usize,
  ) -> Result<()> {
    while byte_count > 0 {
      let mut left = self.block_size - self.upto;
      if left == 0 {
        if let Some(block) = self.current_block.take() {
          self.add_block(block)?;
        }
        self.current_block = Some(vec![0u8; self.block_size]);
        self.upto = 0;
        left = self.block_size;
      }
      let current_block = self
        .current_block
        .as_mut()
        .ok_or_else(|| LuceneError::illegal_state("current_block not initialized"))?;
      if left < byte_count {
        input.read_bytes_with_buffer(current_block, self.upto, left, false)?;
        self.upto = self.block_size;
        byte_count -= left;
      } else {
        input.read_bytes_with_buffer(current_block, self.upto, byte_count, false)?;
        self.upto += byte_count;
        break;
      }
    }
    Ok(())
  }
  /// Copy [`BytesRef`] into the pool, setting the output [`BytesRef`] to the
  /// result.
  ///
  /// Do **not** use this method if `freeze(true)` will be called afterward.
  ///
  /// This only supports `bytes.len() <= block_size`.
  #[allow(dead_code)]
  pub fn copy_with_bytes_ref(
    &mut self,
    _bytes: &BytesRef<Vec<u8>>,
    _out: &mut BytesRef<Vec<u8>>,
  ) -> Result<()> {
    Err(LuceneError::not_implemented(
      "PagedBytes::copy_with_bytes_ref is not implemented",
    ))
  }
  /// Commits the final byte buffer, trimming it when requested.
  pub fn freeze(&mut self, trim: bool) -> Result<Reader> {
    if self.frozen {
      return Err(LuceneError::illegal_state("already frozen"));
    }
    if self.did_skip_bytes {
      return Err(LuceneError::illegal_state(
        "cannot freeze when copy(BytesRef, BytesRef) was used",
      ));
    }

    if let Some(mut block) = self.current_block.take() {
      if trim && self.upto < self.block_size {
        block = block[..self.upto].to_vec();
      }
      self.add_block(block)?;
    } else {
      self.add_block(Vec::new())?;
    }

    self.frozen = true;
    self.current_block = None;

    let mut block = Vec::new();
    for i in 0..self.num_blocks {
      block.push(Arc::new(std::mem::take(&mut self.blocks[i])));
    }
    self.frozen_blocks = Some(block);

    Reader::new(self)
  }
  pub fn get_pointer(&self) -> i64 {
    if self.current_block.is_none() {
      0
    } else {
      (self.num_blocks as i64 * self.block_size as i64) + self.upto as i64
    }
  }
  /// Copy bytes in, writing the length as a 1 or 2 byte vInt prefix.
  pub fn copy_using_length_prefix(&mut self, bytes: &BytesRef<Vec<u8>>) -> Result<i64> {
    if bytes.length >= 32768 {
      return Err(LuceneError::illegal_argument(format!(
        "max length is 32767 (got {})",
        bytes.length
      )));
    }

    if self.upto + bytes.length + 2 > self.block_size {
      if bytes.length + 2 > self.block_size {
        return Err(LuceneError::illegal_argument(format!(
          "block size {} is too small to store length {} bytes",
          self.block_size, bytes.length
        )));
      }
      if let Some(block) = self.current_block.take() {
        self.add_block(block)?;
      }
      self.current_block = Some(vec![0u8; self.block_size]);
      self.upto = 0;
    }

    let pointer = self.get_pointer();
    let current_block = self
      .current_block
      .as_mut()
      .ok_or_else(|| LuceneError::illegal_state("current_block not initialized"))?;

    if bytes.length < 128 {
      current_block[self.upto] = bytes.length as u8;
      self.upto += 1;
    } else {
      let length = (bytes.length | 0x8000) as u16;
      current_block[self.upto] = (length >> 8) as u8;
      current_block[self.upto + 1] = length as u8;
      self.upto += 2;
    }
    current_block.copy_from(
      &bytes.bytes[bytes.offset..bytes.offset + bytes.length],
      self.upto,
    );
    self.upto += bytes.length;

    Ok(pointer)
  }
}
impl Accountable for PagedBytes {
  fn ram_bytes_used(&self) -> Result<i64> {
    let mut bytes = size_of_vec(&self.blocks);
    if let Some(frozen_blocks) = &self.frozen_blocks {
      bytes = bytes.saturating_add(size_of_vec(frozen_blocks));
      if let Some(last_block) = frozen_blocks.last() {
        bytes = bytes
          .saturating_add(
            (frozen_blocks.len() as i64).saturating_mul(mem::size_of::<Vec<u8>>() as i64),
          )
          .saturating_add(
            ((frozen_blocks.len() - 1) as i64).saturating_mul(self.bytes_used_per_block),
          )
          .saturating_add(size_of_vec(last_block.as_ref()));
      }
    } else {
      if self.num_blocks > 0 {
        bytes = bytes
          .saturating_add(((self.num_blocks - 1) as i64).saturating_mul(self.bytes_used_per_block))
          .saturating_add(size_of_vec(&self.blocks[self.num_blocks - 1]));
      }
      if let Some(current_block) = &self.current_block {
        bytes = bytes.saturating_add(size_of_vec(current_block));
      }
    }
    Ok(bytes)
  }
}
/// Provides methods to read BytesRefs from a frozen PagedBytes.
#[derive(Clone, Default)]
pub struct Reader {
  blocks: Vec<Arc<Vec<u8>>>,
  block_bits: usize,
  block_mask: usize,
  block_size: usize,
  bytes_used_per_block: i64,
}

impl Reader {
  /// 1<<blockBits must be bigger than biggest single BytesRef slice that will
  /// be pulled
  pub fn new(paged_bytes: &PagedBytes) -> Result<Self> {
    Ok(Reader {
      blocks: paged_bytes
        .frozen_blocks
        .as_ref()
        .ok_or_else(|| LuceneError::illegal_state("blocks_rc not initialized"))?
        .clone(),
      block_bits: paged_bytes.block_bits,
      block_mask: paged_bytes.block_mask,
      block_size: paged_bytes.block_size,
      bytes_used_per_block: paged_bytes.bytes_used_per_block,
    })
  }
  /// Gets a slice out of [`PagedBytes`] starting at `start` with the given
  /// `length`.
  ///
  /// If the slice spans across a block boundary, this method will allocate
  /// sufficient resources and copy the paged data.
  ///
  /// Slices spanning more than two blocks are **not supported**.
  pub fn fill_slice(&self, b: &mut BytesRef<Vec<u8>>, start: usize, length: usize) -> Result<()> {
    debug_assert!(length <= self.block_size + 1, "length={length}");
    b.length = length;

    if length == 0 {
      return Ok(());
    }

    let index = start >> self.block_bits;
    let offset = start & self.block_mask;

    if self.block_size - offset >= length {
      // TODO IMPORTANT: always copy here, could we avoid copying
      // Within block
      b.bytes.clone_from(self.blocks[index].as_ref());
      b.offset = offset;
    } else {
      // Split across two blocks
      ArrayUtil::grow_no_copy(&mut b.bytes, length)?;
      let first_len = self.block_size - offset;
      b.bytes
        .copy_from(&self.blocks[index][offset..offset + first_len], 0);
      b.bytes.copy_from(
        &self.blocks[index + 1][..length - first_len],
        self.block_size - offset,
      );
      b.offset = 0;
    }
    Ok(())
  }
  /// Get the byte at the given offset.
  pub fn get_byte(&self, o: usize) -> u8 {
    let index = o >> self.block_bits;
    let offset = o & self.block_mask;
    self.blocks[index][offset]
  }
  #[allow(dead_code)]
  pub fn fill(_b: &mut BytesRef<Rc<Vec<u8>>>, _start: i64) {
    unimplemented!("not used in Java Lucene");
  }
}
impl Accountable for Reader {
  fn ram_bytes_used(&self) -> Result<i64> {
    let mut bytes = size_of_vec(&self.blocks);
    if let Some(last_block) = self.blocks.last() {
      bytes = bytes
        .saturating_add((self.blocks.len() as i64).saturating_mul(mem::size_of::<Vec<u8>>() as i64))
        .saturating_add(((self.blocks.len() - 1) as i64).saturating_mul(self.bytes_used_per_block))
        .saturating_add(size_of_vec(last_block.as_ref()));
    }
    Ok(bytes)
  }
}
impl Display for PagedBytes {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}(blocksize={})",
      std::any::type_name::<Self>(),
      self.block_size
    )
  }
}
/// Input that transparently iterates over pages
pub struct PagedBytesDataInput {
  current_block_index: usize,
  current_block_upto: usize,
  block_size: usize,
  block_bits: usize,
  block_mask: usize,
  blocks: Vec<Arc<Vec<u8>>>,
}

impl PagedBytesDataInput {
  fn new(blocks: &PagedBytes) -> Result<Self> {
    debug_assert!(blocks.frozen_blocks.is_some());
    Ok(Self {
      current_block_index: 0,
      current_block_upto: 0,
      block_size: blocks.block_size,
      block_bits: blocks.block_bits,
      block_mask: blocks.block_mask,
      blocks: blocks
        .frozen_blocks
        .as_ref()
        .ok_or_else(|| LuceneError::illegal_state("blocks_rc not initialized"))?
        .clone(),
    })
  }
  /// Returns the current byte position.
  pub fn get_position(&self) -> usize {
    (self.current_block_index * self.block_size) + self.current_block_upto
  }
  /// Seek to a position previously obtained from `get_position()`.
  pub fn set_position(&mut self, pos: usize) {
    self.current_block_index = pos >> self.block_bits;
    self.current_block_upto = pos & self.block_mask;
  }
  fn next_block(&mut self) {
    self.current_block_index += 1;
    self.current_block_upto = 0;
  }
}

impl Display for PagedBytesDataInput {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}(blocks={:?}, current_block_index={}, current_block_upto={})",
      std::any::type_name::<Self>(),
      self.blocks,
      self.current_block_index,
      self.current_block_upto
    )
  }
}

impl crate::core::util::close::Closeable for PagedBytesDataInput {}

impl DataInput for PagedBytesDataInput {
  fn read_byte(&mut self) -> Result<u8> {
    if self.current_block_upto == self.block_size {
      self.next_block();
    }

    let byte = self.blocks[self.current_block_index][self.current_block_upto];
    self.current_block_upto += 1;
    Ok(byte)
  }

  fn read_bytes(&mut self, b: &mut [u8], mut offset: usize, len: usize) -> Result<()> {
    debug_assert!(
      b.len() >= (offset + len),
      "b.len()={}, offset={}, len={}",
      b.len(),
      offset,
      len
    );
    let offset_end = offset + len;

    loop {
      let block = &self.blocks[self.current_block_index];
      let block_left = self.block_size - self.current_block_upto;
      let left = offset_end - offset;

      if block_left < left {
        b.copy_from(
          &block[self.current_block_upto..self.current_block_upto + block_left],
          offset,
        );
        self.next_block();
        offset += block_left;
      } else {
        b.copy_from(
          &block[self.current_block_upto..self.current_block_upto + left],
          offset,
        );
        self.current_block_upto += left;
        break;
      }
    }
    Ok(())
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    GroupVIntUtil::read_group_vint_i32(self, dst, offset)
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    if num_bytes < 0 {
      return Err(LuceneError::illegal_argument(format!(
        "num_bytes must be >= 0, got {num_bytes}"
      )));
    }
    let skip_to = self.get_position() + num_bytes as usize;
    self.set_position(skip_to);
    Ok(())
  }
}

pub struct PagedBytesDataOutput {
  pub paged_bytes: PagedBytes,
}
impl PagedBytesDataOutput {
  fn new(paged_bytes: PagedBytes) -> Self {
    PagedBytesDataOutput { paged_bytes }
  }
  /// Return the current byte position.
  pub fn get_position(&self) -> i64 {
    self.paged_bytes.get_pointer()
  }
}
impl DataOutput for PagedBytesDataOutput {
  fn write_byte(&mut self, b: u8) -> Result<()> {
    if self.paged_bytes.upto == self.paged_bytes.block_size {
      if let Some(block) = self.paged_bytes.current_block.take() {
        self.paged_bytes.add_block(block)?;
      }
      self.paged_bytes.current_block = Some(vec![0u8; self.paged_bytes.block_size]);
      self.paged_bytes.upto = 0;
    }

    let block = self
      .paged_bytes
      .current_block
      .as_mut()
      .ok_or_else(|| LuceneError::illegal_state("current_block not initialized"))?;
    block[self.paged_bytes.upto] = b;
    self.paged_bytes.upto += 1;
    Ok(())
  }

  fn write_bytes_range(&mut self, b: &[u8], mut offset: usize, length: usize) -> Result<()> {
    debug_assert!(
      b.len() >= (offset + length),
      "b.len={} offset={} length={}",
      b.len(),
      offset,
      length
    );
    if length == 0 {
      return Ok(());
    }

    if self.paged_bytes.upto == self.paged_bytes.block_size {
      if let Some(block) = self.paged_bytes.current_block.take() {
        self.paged_bytes.add_block(block)?;
      }
      self.paged_bytes.current_block = Some(vec![0u8; self.paged_bytes.block_size]);
      self.paged_bytes.upto = 0;
    }
    let offset_end = offset + length;

    loop {
      let left = offset_end - offset;
      let block_left = self.paged_bytes.block_size - self.paged_bytes.upto;

      let current_block = self
        .paged_bytes
        .current_block
        .as_mut()
        .ok_or_else(|| LuceneError::illegal_state("current_block not initialized"))?;
      if block_left < left {
        current_block.copy_from(&b[offset..offset + block_left], self.paged_bytes.upto);
        let old_block = std::mem::replace(current_block, vec![0u8; self.paged_bytes.block_size]);
        self.paged_bytes.add_block(old_block)?;

        self.paged_bytes.upto = 0;
        offset += block_left;
      } else {
        current_block.copy_from(&b[offset..offset + left], self.paged_bytes.upto);
        self.paged_bytes.upto += left;
        break;
      }
    }
    Ok(())
  }
}

/// Returns a DataInput to read values from this PagedBytes instance.
pub fn get_data_input(paged_bytes: &PagedBytes) -> Result<PagedBytesDataInput> {
  if !paged_bytes.frozen {
    return Err(LuceneError::illegal_state(
      "must call freeze() before get_data_input()",
    ));
  }

  PagedBytesDataInput::new(paged_bytes)
}
/// Returns a DataOutput that you may use to write into this PagedBytes
/// instance. If you do this,  you should not call the other writing methods
/// (eg, copy); results are undefined.
pub fn get_data_output(paged_bytes: PagedBytes) -> Result<PagedBytesDataOutput> {
  if paged_bytes.frozen {
    return Err(LuceneError::illegal_state(
      "cannot get DataOutput after freeze()",
    ));
  }

  Ok(PagedBytesDataOutput::new(paged_bytes))
}

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
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::hint::black_box;
#[cfg(unix)]
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(unix)]
use crate::core::store::native_access::NativeAccess;
#[cfg(unix)]
use crate::core::store::posix_native_access::PosixNativeAccess;
#[cfg(unix)]
use memmap2::Advice;
use memmap2::{Mmap, MmapOptions};

use crate::core::store::index_input::get_full_slice_description;
use crate::core::store::random_access_input::RandomAccessInput;
use crate::core::store::{DataInput, IndexInput, ReadAdvice};
use crate::core::util::bit_util::BitUtil;
use crate::core::util::clone::TryClone;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::group_vint_util::{GroupVIntUtil, IntReader};
use crate::core::util::{CoreHelper, TryIntoInt};

pub struct MemorySegmentIndexInput {
  resource_desc: String,
  shared: Arc<MemorySegmentIndexInputShared>,
  offset: usize,
  length: usize,
  chunk_size_power: u32,
  chunk_size_mask: usize,
  cur_segment_index: usize,
  cur_position: usize,
  closed: AtomicBool,
  owns_shared: bool,
  #[cfg(unix)]
  native_access: PosixNativeAccess,
}

struct MemorySegmentIndexInputShared {
  segments: Vec<Mmap>,
  closed: AtomicBool,
}

impl MemorySegmentIndexInput {
  pub fn new(
    resource_desc: String,
    path: &Path,
    read_advice: ReadAdvice,
    chunk_size_power: u32,
    preload: bool,
  ) -> Result<Self> {
    let file =
      File::open(path).map_err(|e| LuceneError::io_with_path(path.display().to_string(), e))?;
    let file_size_u64 = file
      .metadata()
      .map_err(|e| LuceneError::io_with_path(path.display().to_string(), e))?
      .len();
    let length: usize = file_size_u64.try_convert()?;

    if chunk_size_power >= usize::BITS {
      return Err(LuceneError::illegal_argument(format!(
        "chunkSizePower {chunk_size_power} is too large for this platform"
      )));
    }
    if (file_size_u64 >> chunk_size_power) >= i32::MAX as u64 {
      return Err(LuceneError::illegal_argument(format!(
        "File too big for chunk size: {resource_desc}"
      )));
    }

    let chunk_size = 1usize << chunk_size_power;
    #[cfg(unix)]
    let native_access = PosixNativeAccess::new()?;
    #[cfg(not(unix))]
    let _ = read_advice;
    let mut segments = Vec::new();
    let mut start_offset = 0usize;
    while start_offset < length {
      let seg_size = chunk_size.min(length - start_offset);
      let mmap = unsafe {
        MmapOptions::new()
          .offset(start_offset.try_convert()?)
          .len(seg_size)
          .map(&file)
      }
      .map_err(|e| LuceneError::io_with_path(path.display().to_string(), e))?;

      #[cfg(unix)]
      {
        if preload {
          native_access
            .madvise_will_need(&mmap)
            .map_err(|e| LuceneError::io_with_path(path.display().to_string(), e))?;
        } else {
          native_access
            .madvise(&mmap, &read_advice)
            .map_err(|e| LuceneError::io_with_path(path.display().to_string(), e))?;
        }
      }

      if preload {
        let mut value = 0u8;
        let mut pos = 0usize;
        while pos < mmap.len() {
          value ^= mmap[pos];
          pos += 4096;
        }
        black_box(value);
      }

      segments.push(mmap);
      start_offset += seg_size;
    }

    Ok(Self {
      resource_desc,
      shared: Arc::new(MemorySegmentIndexInputShared {
        segments,
        closed: AtomicBool::new(false),
      }),
      offset: 0,
      length,
      chunk_size_power,
      chunk_size_mask: chunk_size - 1,
      cur_segment_index: 0,
      cur_position: 0,
      closed: AtomicBool::new(false),
      owns_shared: true,
      #[cfg(unix)]
      native_access,
    })
  }

  fn with_slice(&self, slice_description: &str, offset: usize, length: usize) -> Result<Self> {
    match offset.checked_add(length) {
      Some(slice_end) if slice_end <= self.length => {},
      _ => {
        return Err(LuceneError::illegal_argument(format!(
          "slice() {slice_description} out of bounds: offset={offset},length={length},fileLength={}: {}",
          self.length, self
        )));
      },
    }
    self.ensure_open()?;
    let slice_offset = self
      .offset
      .checked_add(offset)
      .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
    let (cur_segment_index, cur_position) = Self::cursor_for_global_position(
      &self.shared.segments,
      self.chunk_size_power,
      self.chunk_size_mask,
      slice_offset,
    )?;
    Ok(Self {
      resource_desc: get_full_slice_description(slice_description),
      shared: self.shared.clone(),
      offset: slice_offset,
      length,
      chunk_size_power: self.chunk_size_power,
      chunk_size_mask: self.chunk_size_mask,
      cur_segment_index,
      cur_position,
      closed: AtomicBool::new(false),
      owns_shared: false,
      #[cfg(unix)]
      native_access: self.native_access,
    })
  }

  fn cursor_for_global_position(
    segments: &[Mmap],
    chunk_size_power: u32,
    chunk_size_mask: usize,
    global_pos: usize,
  ) -> Result<(usize, usize)> {
    if segments.is_empty() {
      if global_pos == 0 {
        return Ok((0, 0));
      }
      return Err(LuceneError::illegal_state(format!(
        "no mmap segment for position {global_pos}"
      )));
    }
    let segment_index = global_pos >> chunk_size_power;
    if segment_index < segments.len() {
      let position = global_pos & chunk_size_mask;
      if position <= segments[segment_index].len() {
        return Ok((segment_index, position));
      }
      return Err(LuceneError::illegal_state(format!(
        "position {position} is outside segment {segment_index} with length {}",
        segments[segment_index].len()
      )));
    }
    if segment_index == segments.len() && (global_pos & chunk_size_mask) == 0 {
      // Java maps an extra 0-byte segment; Rust keeps the equivalent EOF cursor
      // at the end of the last mapped segment.
      let last_segment_index = segments.len() - 1;
      return Ok((last_segment_index, segments[last_segment_index].len()));
    }
    Err(LuceneError::illegal_state(format!(
      "segment index {segment_index} is outside {} segments for position {global_pos}",
      segments.len()
    )))
  }

  fn ensure_open(&self) -> Result<()> {
    if self.closed.load(Ordering::Relaxed) || self.shared.closed.load(Ordering::SeqCst) {
      return Err(LuceneError::already_closed(format!(
        "Already closed: {}",
        self.resource_desc
      )));
    }
    Ok(())
  }

  fn ensure_current_read(&self, len: usize) -> Result<()> {
    let end = IndexInput::get_file_pointer(self)?
      .checked_add(len)
      .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
    if end > self.length {
      return Err(LuceneError::eof(format!("read past EOF: {self}")));
    }
    Ok(())
  }

  fn current_segment_slice(&self, len: usize) -> Result<Option<&[u8]>> {
    self.ensure_current_read(len)?;
    if len == 0 {
      return Ok(Some(&[]));
    }
    let segment = self
      .shared
      .segments
      .get(self.cur_segment_index)
      .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
    let Some(segment_end) = self.cur_position.checked_add(len) else {
      return Ok(None);
    };
    if segment_end <= segment.len() {
      Ok(Some(&segment[self.cur_position..segment_end]))
    } else {
      // cross segment
      Ok(None)
    }
  }

  fn read_current_buffer<R>(&mut self, len: usize, read: impl FnOnce(&[u8]) -> R) -> Result<R> {
    if let Some(bytes) = self.current_segment_slice(len)? {
      let value = read(bytes);
      self.cur_position += len;
      return Ok(value);
    }

    let mut bytes = vec![0u8; len];
    self.read_bytes_boundary_current(&mut bytes, 0, len)?;
    Ok(read(&bytes))
  }

  fn decode_short(bytes: &[u8]) -> Result<i16> {
    let bytes: [u8; BitUtil::SHORT_BYTES] = bytes
      .try_into()
      .map_err(|_| LuceneError::illegal_state("short read returned an invalid byte length"))?;
    Ok(i16::from_le_bytes(bytes))
  }

  fn decode_int(bytes: &[u8]) -> Result<i32> {
    let bytes: [u8; BitUtil::INT_BYTES] = bytes
      .try_into()
      .map_err(|_| LuceneError::illegal_state("int read returned an invalid byte length"))?;
    Ok(i32::from_le_bytes(bytes))
  }

  fn decode_long(bytes: &[u8]) -> Result<i64> {
    let bytes: [u8; BitUtil::LONG_BYTES] = bytes
      .try_into()
      .map_err(|_| LuceneError::illegal_state("long read returned an invalid byte length"))?;
    Ok(i64::from_le_bytes(bytes))
  }

  fn read_bytes_boundary_current(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    self.ensure_current_read(len)?;
    let mut remaining = len;
    let mut output_pos = offset;
    while remaining > 0 {
      let segment = self
        .shared
        .segments
        .get(self.cur_segment_index)
        .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
      if self.cur_position > segment.len() {
        return Err(LuceneError::eof(format!("read past EOF: {self}")));
      }
      let cur_avail = segment.len() - self.cur_position;
      if cur_avail == 0 {
        self.cur_segment_index += 1;
        if self.cur_segment_index >= self.shared.segments.len() {
          return Err(LuceneError::eof(format!("read past EOF: {self}")));
        }
        self.cur_position = 0;
        continue;
      }
      let to_copy = remaining.min(cur_avail);
      b[output_pos..output_pos + to_copy]
        .copy_from_slice(&segment[self.cur_position..self.cur_position + to_copy]);
      self.cur_position += to_copy;
      remaining -= to_copy;
      output_pos += to_copy;
    }
    Ok(())
  }

  fn read_byte_at(&self, pos: usize) -> Result<u8> {
    self.ensure_open()?;
    self.read_buffer(pos, BitUtil::BYTE_BYTES, |bytes| bytes[0])
  }

  fn segment_slice_at(&self, pos: usize, len: usize) -> Result<Option<&[u8]>> {
    let end = pos
      .checked_add(len)
      .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
    if end > self.length {
      return Err(LuceneError::eof(format!("read past EOF: {self}")));
    }
    if len == 0 {
      return Ok(Some(&[]));
    }

    let global_pos = self
      .offset
      .checked_add(pos)
      .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
    let segment_index = global_pos >> self.chunk_size_power;
    let segment_offset = global_pos & self.chunk_size_mask;
    let segment = self
      .shared
      .segments
      .get(segment_index)
      .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
    let Some(segment_end) = segment_offset.checked_add(len) else {
      return Ok(None);
    };
    if segment_end <= segment.len() {
      Ok(Some(&segment[segment_offset..segment_end]))
    } else {
      Ok(None)
    }
  }

  fn read_buffer<R>(&self, pos: usize, len: usize, read: impl FnOnce(&[u8]) -> R) -> Result<R> {
    if let Some(bytes) = self.segment_slice_at(pos, len)? {
      return Ok(read(bytes));
    }

    let mut bytes = vec![0u8; len];
    self.read_bytes_boundary(pos, &mut bytes, 0, len)?;
    Ok(read(&bytes))
  }

  fn read_bytes_boundary(&self, pos: usize, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    let mut remaining = len;
    let mut input_pos = pos;
    let mut output_pos = offset;
    while remaining > 0 {
      let global_pos = self.offset + input_pos;
      let segment_index = global_pos >> self.chunk_size_power;
      let segment_offset = global_pos & self.chunk_size_mask;
      let segment = &self.shared.segments[segment_index];
      let to_copy = remaining.min(segment.len() - segment_offset);
      b[output_pos..output_pos + to_copy]
        .copy_from_slice(&segment[segment_offset..segment_offset + to_copy]);
      remaining -= to_copy;
      input_pos += to_copy;
      output_pos += to_copy;
    }
    Ok(())
  }

  fn read_bytes_at(&self, pos: usize, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    CoreHelper::check_from_index_size(offset, len, b.len())?;
    self.read_buffer(pos, len, |bytes| {
      b[offset..offset + len].copy_from_slice(bytes);
    })
  }

  #[cfg(unix)]
  fn advise(
    &self,
    offset: usize,
    length: usize,
    mut advice: impl FnMut(&Mmap, usize, usize) -> io::Result<()>,
  ) -> Result<()> {
    let end = offset
      .checked_add(length)
      .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
    if end > self.length {
      return Err(LuceneError::eof(format!("read past EOF: {self}")));
    }

    let mut remaining = length;
    let mut input_pos = offset;
    while remaining > 0 {
      let global_pos = self.offset + input_pos;
      let segment_index = global_pos >> self.chunk_size_power;
      let segment_offset = global_pos & self.chunk_size_mask;
      let segment = &self.shared.segments[segment_index];
      let to_advise = remaining.min(segment.len() - segment_offset);
      advice(segment, segment_offset, to_advise).map_err(LuceneError::io)?;
      remaining -= to_advise;
      input_pos += to_advise;
    }
    Ok(())
  }

  #[cfg(unix)]
  fn advise_will_need(&self, pos: usize, len: usize) -> Result<()> {
    self.advise(pos, len, |segment, offset, length| {
      segment.advise_range(Advice::WillNeed, offset, length)
    })
  }
}

impl DataInput for MemorySegmentIndexInput {
  fn read_byte(&mut self) -> Result<u8> {
    self.ensure_open()?;
    self.ensure_current_read(BitUtil::BYTE_BYTES)?;
    if let Some(segment) = self.shared.segments.get(self.cur_segment_index)
      && self.cur_position < segment.len()
    {
      let value = segment[self.cur_position];
      self.cur_position += BitUtil::BYTE_BYTES;
      return Ok(value);
    }

    let mut bytes = [0u8; BitUtil::BYTE_BYTES];
    self.read_bytes_boundary_current(&mut bytes, 0, BitUtil::BYTE_BYTES)?;
    Ok(bytes[0])
  }

  fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    CoreHelper::check_from_index_size(offset, len, b.len())?;
    if let Some(bytes) = self.current_segment_slice(len)? {
      b[offset..offset + len].copy_from_slice(bytes);
      self.cur_position += len;
      return Ok(());
    }
    self.read_bytes_boundary_current(b, offset, len)
  }

  fn read_short(&mut self) -> Result<i16> {
    self.ensure_open()?;
    self.read_current_buffer(BitUtil::SHORT_BYTES, Self::decode_short)?
  }

  fn read_int(&mut self) -> Result<i32> {
    self.ensure_open()?;
    self.read_current_buffer(BitUtil::INT_BYTES, Self::decode_int)?
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    self.ensure_open()?;
    let segment_remaining = self
      .shared
      .segments
      .get(self.cur_segment_index)
      .map_or(0, |segment| segment.len().saturating_sub(self.cur_position));
    let remaining = segment_remaining.min(
      self
        .length
        .saturating_sub(IndexInput::get_file_pointer(self)?),
    );
    let pos = self.cur_position;
    let len =
      GroupVIntUtil::read_group_vint_i32_with_reader(self, remaining as u64, pos, dst, offset)?;
    self.cur_position += len;
    Ok(())
  }

  fn read_long(&mut self) -> Result<i64> {
    self.ensure_open()?;
    self.read_current_buffer(BitUtil::LONG_BYTES, Self::decode_long)?
  }

  fn read_longs(&mut self, dst: &mut [i64], offset: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    CoreHelper::check_from_index_size(offset, len, dst.len())?;
    let byte_len = len
      .checked_mul(BitUtil::LONG_BYTES)
      .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
    self.read_current_buffer(byte_len, |bytes| {
      for (value, chunk) in dst[offset..offset + len]
        .iter_mut()
        .zip(bytes.as_chunks::<{ BitUtil::LONG_BYTES }>().0)
      {
        *value = i64::from_le_bytes(*chunk);
      }
    })?;
    Ok(())
  }

  fn read_ints(&mut self, dst: &mut [i32], offset: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    CoreHelper::check_from_index_size(offset, len, dst.len())?;
    let byte_len = len
      .checked_mul(BitUtil::INT_BYTES)
      .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
    self.read_current_buffer(byte_len, |bytes| {
      for (value, chunk) in dst[offset..offset + len]
        .iter_mut()
        .zip(bytes.as_chunks::<{ BitUtil::INT_BYTES }>().0)
      {
        *value = i32::from_le_bytes(*chunk);
      }
    })?;
    Ok(())
  }

  fn read_floats(&mut self, dst: &mut [f32], offset: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    CoreHelper::check_from_index_size(offset, len, dst.len())?;
    let byte_len = len
      .checked_mul(BitUtil::FLOAT_BYTES)
      .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
    self.read_current_buffer(byte_len, |bytes| {
      for (value, chunk) in dst[offset..offset + len]
        .iter_mut()
        .zip(bytes.as_chunks::<{ BitUtil::FLOAT_BYTES }>().0)
      {
        *value = f32::from_bits(u32::from_le_bytes(*chunk));
      }
    })?;
    Ok(())
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    IndexInput::skip_bytes(self, num_bytes)
  }

  fn is_index_input(&self) -> bool {
    true
  }

  fn seek_in_data_input(&mut self, pos: usize) -> Result<()> {
    IndexInput::seek(self, pos)
  }

  fn get_file_pointer_in_data_input(&self) -> Result<usize> {
    IndexInput::get_file_pointer(self)
  }
}

impl IntReader for MemorySegmentIndexInput {
  fn read(&mut self, pos: usize) -> Result<i32> {
    self.ensure_open()?;
    let segment = self
      .shared
      .segments
      .get(self.cur_segment_index)
      .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
    let end = pos
      .checked_add(BitUtil::INT_BYTES)
      .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
    let bytes = segment
      .get(pos..end)
      .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
    let bytes: [u8; BitUtil::INT_BYTES] = bytes
      .try_into()
      .map_err(|_| LuceneError::illegal_state("int read returned an invalid byte length"))?;
    Ok(i32::from_le_bytes(bytes))
  }
}

impl Display for MemorySegmentIndexInput {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.resource_desc)
  }
}

impl CloseableRef for MemorySegmentIndexInput {
  fn close(&self) -> Result<()> {
    if self.closed.swap(true, Ordering::Relaxed) {
      return Ok(());
    }
    if self.owns_shared {
      self.shared.closed.store(true, Ordering::SeqCst);
    }
    Ok(())
  }
}

impl TryClone for MemorySegmentIndexInput {
  fn try_clone(&self) -> Result<Self>
  where
    Self: Sized,
  {
    self.ensure_open()?;
    Ok(Self {
      resource_desc: self.resource_desc.clone(),
      shared: self.shared.clone(),
      offset: self.offset,
      length: self.length,
      chunk_size_power: self.chunk_size_power,
      chunk_size_mask: self.chunk_size_mask,
      cur_segment_index: self.cur_segment_index,
      cur_position: self.cur_position,
      closed: AtomicBool::new(false),
      owns_shared: false,
      #[cfg(unix)]
      native_access: self.native_access,
    })
  }
}
impl Drop for MemorySegmentIndexInput {
  fn drop(&mut self) {
    let _ = self.close();
  }
}
impl IndexInput for MemorySegmentIndexInput {
  type IndexInput = MemorySegmentIndexInput;

  fn get_file_pointer(&self) -> Result<usize> {
    self.ensure_open()?;
    let global_pos = self
      .cur_segment_index
      .checked_shl(self.chunk_size_power)
      .and_then(|segment_start| segment_start.checked_add(self.cur_position))
      .ok_or_else(|| LuceneError::illegal_state(format!("file pointer overflow: {self}")))?;
    global_pos
      .checked_sub(self.offset)
      .ok_or_else(|| LuceneError::illegal_state(format!("file pointer before offset: {self}")))
  }

  fn seek(&mut self, pos: usize) -> Result<()> {
    self.ensure_open()?;
    if pos > self.length {
      return Err(LuceneError::eof(format!(
        "read past EOF: pos={} vs length={}: {}",
        pos, self.length, self
      )));
    }
    let global_pos = self
      .offset
      .checked_add(pos)
      .ok_or_else(|| LuceneError::eof(format!("read past EOF: {self}")))?;
    let (cur_segment_index, cur_position) = Self::cursor_for_global_position(
      &self.shared.segments,
      self.chunk_size_power,
      self.chunk_size_mask,
      global_pos,
    )?;
    self.cur_segment_index = cur_segment_index;
    self.cur_position = cur_position;
    Ok(())
  }

  fn length(&self) -> Result<usize> {
    Ok(self.length)
  }

  fn slice(
    &self,
    slice_description: &str,
    offset: usize,
    length: usize,
  ) -> Result<Self::IndexInput> {
    self.with_slice(slice_description, offset, length)
  }

  fn slice_with_read_advice(
    &self,
    description: &str,
    offset: usize,
    length: usize,
    read_advice: &ReadAdvice,
  ) -> Result<Self::IndexInput> {
    let slice = self.slice(description, offset, length)?;
    #[cfg(unix)]
    {
      if read_advice != &ReadAdvice::Normal
        && length >= slice.native_access.get_page_size()
        && let Some(advice) = slice.native_access.map_read_advice(read_advice)
      {
        slice.advise(0, slice.length, |segment, offset, length| {
          segment.advise_range(advice, offset, length)
        })?;
      }
    }
    #[cfg(not(unix))]
    {
      let _ = read_advice;
    }
    Ok(slice)
  }

  type RandomAccessSlice = MemorySegmentIndexInput;

  fn random_access_slice(&self, offset: usize, length: usize) -> Result<Self::RandomAccessSlice> {
    self.with_slice("random_access_slice", offset, length)
  }

  fn update_read_advice(&self, read_advice: ReadAdvice) -> Result<()> {
    self.ensure_open()?;
    #[cfg(unix)]
    {
      if let Some(advice) = self.native_access.map_read_advice(&read_advice) {
        self.advise(0, self.length, |segment, offset, length| {
          segment.advise_range(advice, offset, length)
        })?;
      }
    }
    #[cfg(not(unix))]
    {
      let _ = read_advice;
    }
    Ok(())
  }

  fn is_loaded(&self) -> Result<Option<bool>> {
    #[cfg(unix)]
    {
      self.ensure_open()?;
      let mut is_loaded = true;
      self.advise(0, self.length, |segment, offset, length| {
        if is_loaded {
          is_loaded = self.native_access.is_loaded(segment, offset, length)?;
        }
        Ok(())
      })?;
      Ok(Some(is_loaded))
    }
    #[cfg(windows)]
    {
      // On Windows, return no hint until page residency can be queried safely: #14050.
      Ok(None)
    }
    #[cfg(all(not(unix), not(windows)))]
    {
      Ok(None)
    }
  }
}

impl RandomAccessInput for MemorySegmentIndexInput {
  fn length(&self) -> Result<usize> {
    Ok(self.length)
  }

  fn read_byte(&mut self, pos: usize) -> Result<u8> {
    self.ensure_open()?;
    self.read_byte_at(pos)
  }

  fn read_bytes(&mut self, pos: usize, buf: &mut [u8], offset: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    self.read_bytes_at(pos, buf, offset, len)
  }

  fn read_short(&mut self, pos: usize) -> Result<i16> {
    self.ensure_open()?;
    self.read_buffer(pos, BitUtil::SHORT_BYTES, Self::decode_short)?
  }

  fn read_int(&mut self, pos: usize) -> Result<i32> {
    self.ensure_open()?;
    self.read_buffer(pos, BitUtil::INT_BYTES, Self::decode_int)?
  }

  fn read_long(&mut self, pos: usize) -> Result<i64> {
    self.ensure_open()?;
    self.read_buffer(pos, BitUtil::LONG_BYTES, Self::decode_long)?
  }

  fn prefetch(&mut self, pos: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    #[cfg(unix)]
    {
      self.advise_will_need(pos, len)
    }
    #[cfg(not(unix))]
    {
      let _ = (pos, len);
      Ok(())
    }
  }

  fn is_loaded(&self) -> Result<Option<bool>> {
    IndexInput::is_loaded(self)
  }
}

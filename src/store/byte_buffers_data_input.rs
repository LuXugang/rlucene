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
use crate::store::random_access_input::RandomAccessInput;
use crate::store::DataInput;
use crate::util::accountable::Accountable;
use crate::util::bit_util::{FLOAT_BYTES, INT_BYTES, LONG_BYTES, SHORT_BYTES};
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::group_vint_util::GroupVIntUtil;
use byteorder::ReadBytesExt;
use byteorder::{ByteOrder, LE};
use std::io::Cursor;

/**
 * A `DataInput` implementing `RandomAccessInput` and reading data from a list of `Vec<Cursor<&[u8]>>`.
 */
pub struct ByteBuffersDataInput<'a> {
    blocks: Vec<Cursor<&'a [u8]>>,
    block_mask: usize,
    block_bits: usize,
    length: u64,
    offset: u64,
    pos: u64,
}
impl<'a> ByteBuffersDataInput<'a> {
    pub fn new(blocks: Vec<Cursor<&'a [u8]>>) -> Self {
        let (block_bits, block_mask) = if blocks.is_empty() {
            (32, !0)
        } else {
            let block_bytes = blocks[0].get_ref().len() as u64;
            let block_bits = block_bytes.trailing_zeros() as usize;
            (block_bits, (1 << block_bits) - 1)
        };
        // blocks may have empty buffer if was reused, so we can't count length like `ByteBuffersDataOutput#size()`
        let length = blocks.iter().map(|block| block.remain()).sum();
        // The initial "position" of this stream is shifted by the position of the first block.
        let offset = blocks.first().map_or(0, |block| block.position());

        Self {
            blocks,
            block_mask,
            block_bits,
            length,
            offset,
            pos: offset,
        }
    }
    pub fn new_with_blocks(blocks: Vec<Cursor<&'a [u8]>>) -> Self {
        todo!()
    }
    fn block_index(&self, pos: u64) -> usize {
        let value = pos >> self.block_bits;
        debug_assert!(value <= i32::MAX as u64);
        value as usize
    }
    fn block_offset(&self, pos: u64) -> usize {
        let value = pos & (self.block_mask as u64);
        debug_assert!(value <= i32::MAX as u64,);
        value as usize
    }
    fn read_buffer<T, C>(
        &mut self,
        mut pos: u64,
        len: usize,
        output: &mut [T],
        type_size: usize,
        converter: C,
    ) -> Result<(), DataIOError>
    where
        C: Fn(&[u8]) -> T,
        T: Copy,
    {
        let mut len = len * type_size;
        debug_assert!(output.len() == len / type_size);
        let mut bytes = vec![0; len];
        let mut bytes_offset = 0;

        while len > 0 {
            let block_index = self.block_index(pos);
            let block_offset = self.block_offset(pos);

            if block_index >= self.blocks.len() || block_offset + len > (1 << self.block_bits) {
                return Err(DataIOError::eof(format!("{}", pos)));
            }

            let block = self.blocks.get_mut(block_index).unwrap();
            let block_position = block.position();
            let block_vec = block.get_ref();
            let available = block.remain();

            debug_assert!(available <= u32::MAX as u64);

            if available == 0 {
                // move to next block
                continue;
            }

            let chunk = len.min(available as usize);
            bytes[bytes_offset..bytes_offset + chunk]
                .copy_from_slice(&block_vec[block_offset..block_offset + chunk]);
            block.set_position(block_position + (chunk as u64));
            bytes_offset += chunk;
            pos += chunk as u64;
            len -= chunk;
        }

        debug_assert!(bytes.len() % type_size == 0);
        debug_assert!(output.len() == bytes.len() / type_size);
        if type_size == 1 {
            let output_bytes = unsafe {
                std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u8, output.len())
            };
            output_bytes.copy_from_slice(&bytes);
        } else {
            output
                .iter_mut()
                .zip(bytes.chunks_exact(type_size).map(converter))
                .for_each(|(out, value)| *out = value);
        }

        Ok(())
    }
    fn read_longs(&mut self, pos: u64, len: usize, output: &mut [i64]) -> Result<(), DataIOError> {
        self.read_buffer(pos, len, output, LONG_BYTES, LE::read_i64)
    }
    fn read_bytes(&mut self, pos: u64, len: usize, output: &mut [u8]) -> Result<(), DataIOError> {
        self.read_buffer(pos, len, output, 1, |_| unreachable!())
    }
    fn read_ints(&mut self, pos: u64, len: usize, output: &mut [i32]) -> Result<(), DataIOError> {
        self.read_buffer(pos, len, output, INT_BYTES, LE::read_i32)
    }
    fn read_shorts(&mut self, pos: u64, len: usize, output: &mut [i16]) -> Result<(), DataIOError> {
        self.read_buffer(pos, len, output, SHORT_BYTES, LE::read_i16)
    }
    fn read_floats(&mut self, pos: u64, len: usize, output: &mut [f32]) -> Result<(), DataIOError> {
        self.read_buffer(pos, len, output, FLOAT_BYTES, LE::read_f32)
    }

    fn seek(&mut self, position: u64) -> Result<(), DataIOError> {
        self.pos = position + self.offset;
        if position > self.length() {
            self.pos = self.length;
            return Err(DataIOError::eof(format!("{}", self.pos)));
        }
        Ok(())
    }
    fn position(&self) -> u64 {
        self.pos - self.offset
    }
    pub fn slice(&self, offset: u64, length: u64) -> ByteBuffersDataInput<'a> {
        let blocks = slice_buffer_list(&self.blocks, offset, length);
        Self::new(blocks)
    }
}

impl DataInput for ByteBuffersDataInput<'_> {
    fn read_byte(&mut self) -> Result<u8, DataIOError> {
        let block_index = self.block_index(self.pos);
        debug_assert!(block_index < self.blocks.len());
        let block_offset = self.block_offset(self.pos);
        let block = self.blocks.get_mut(block_index).unwrap();
        let block_position = block.position();
        let v = block.get_ref()[block_offset];
        block.set_position(block_position + 1);
        self.pos += 1;
        Ok(v)
    }
    fn read_bytes(&mut self, arr: &mut [u8], off: usize, len: usize) -> Result<(), DataIOError> {
        self.read_bytes(self.pos, len, &mut arr[off..off + len])?;
        self.pos += len as u64;
        Ok(())
    }

    fn read_short(&mut self) -> Result<i16, DataIOError> {
        let mut bytes = [0; SHORT_BYTES];
        self.read_shorts(self.pos, 1, &mut bytes)?;
        self.pos += SHORT_BYTES as u64;
        Ok(bytes[0])
    }

    fn read_int(&mut self) -> Result<i32, DataIOError> {
        let mut bytes = [0; INT_BYTES];
        self.read_ints(self.pos, 1, &mut bytes)?;
        self.pos += INT_BYTES as u64;
        Ok(bytes[0])
    }

    fn read_group_vint(&mut self, dst: &mut [i64], offset: usize) -> Result<(), DataIOError> {
        let block_index = self.block_index(self.pos);
        let block_offset = self.block_offset(self.pos);
        let block = self.blocks.get_mut(block_index).unwrap();
        let remain = block.remain() as usize;
        let len = GroupVIntUtil::read_group_vint_with_reader(
            self,
            remain as u64,
            block_offset as u64,
            dst,
            offset,
        )?;
        self.pos += len as u64;
        Ok(())
    }
    fn read_long(&mut self) -> Result<i64, DataIOError> {
        let mut bytes = [0; LONG_BYTES];
        self.read_longs(self.pos, 1, &mut bytes)?;
        self.pos += LONG_BYTES as u64;
        Ok(bytes[0])
    }

    fn read_longs(
        &mut self,
        dst: &mut [i64],
        offset: usize,
        len: usize,
    ) -> Result<(), DataIOError> {
        self.read_longs(self.pos, len, &mut dst[offset..offset + len])?;
        self.pos += len as u64;
        Ok(())
    }

    fn read_floats(
        &mut self,
        dst: &mut [f32],
        offset: usize,
        len: usize,
    ) -> Result<(), DataIOError> {
        self.read_floats(self.pos, len, &mut dst[offset..offset + len])?;
        self.pos += len as u64;
        Ok(())
    }

    fn skip_bytes(&mut self, num_bytes: u64) -> Result<(), DataIOError> {
        let skip_to = self.position() + num_bytes;
        self.seek(skip_to)
    }
}
impl RandomAccessInput for ByteBuffersDataInput<'_> {
    fn length(&self) -> u64 {
        self.length
    }

    fn read_byte(&mut self, pos: u64) -> Result<u8, DataIOError> {
        let pos = pos + self.offset;
        let mut bytes = [0; 1];
        self.read_bytes(pos, 1, &mut bytes)?;
        self.pos += 1;
        Ok(bytes[0])
    }

    fn read_short(&mut self, pos: u64) -> Result<i16, DataIOError> {
        let pos = pos + self.offset;
        let mut bytes = [0; SHORT_BYTES];
        self.read_shorts(pos, 1, &mut bytes)?;
        self.pos += SHORT_BYTES as u64;
        Ok(bytes[0])
    }

    fn read_int(&mut self, pos: u64) -> Result<i32, DataIOError> {
        let pos = pos + self.offset;
        let mut bytes = [0; INT_BYTES];
        self.read_ints(pos, 1, &mut bytes)?;
        self.pos += INT_BYTES as u64;
        Ok(bytes[0])
    }

    fn read_long(&mut self, pos: u64) -> Result<i64, DataIOError> {
        let pos = pos + self.offset;
        let mut bytes = [0; LONG_BYTES];
        self.read_longs(pos, 1, &mut bytes)?;
        self.pos += LONG_BYTES as u64;
        Ok(bytes[0])
    }

    fn pre_fetch(&mut self, pos: u64, len: u64) {
        todo!()
    }
}

impl Accountable for ByteBuffersDataInput<'_> {
    fn ram_bytes_used(&self) -> i64 {
        unimplemented!()
    }
}

trait CursorExt {
    fn remain(&self) -> u64;
}

impl CursorExt for Cursor<&[u8]> {
    fn remain(&self) -> u64 {
        let position = self.position();
        let total = self.get_ref().len() as u64;
        // set_position seems not check bound
        debug_assert!(
            position <= total,
            "Position ({}) exceeds total ({})",
            position,
            total
        );
        total.saturating_sub(position)
    }
}

pub fn slice_buffer_list<'a>(
    blocks: &[Cursor<&'a [u8]>],
    offset: u64,
    length: u64,
) -> Vec<Cursor<&'a [u8]>> {
    assert!(!blocks.is_empty(), "blocks cannot be empty");

    let abs_start = blocks[0].position() + offset;
    let abs_end = abs_start + length;

    let block_bytes = blocks[0].get_ref().len() as u64;
    let block_bits = block_bytes.trailing_zeros() as u64;
    let block_mask = (1u64 << block_bits) - 1;

    let start_block_idx = (abs_start / block_bytes) as usize;
    let end_block_idx = (abs_end / block_bytes) as usize;

    // Create a new Cursor for each block and adjust the position and underlying data range as needed
    blocks[start_block_idx..=end_block_idx]
        .iter()
        .enumerate()
        .map(|(i, block)| {
            let vec_data = *block.get_ref();

            if i == 0 {
                // first block we need to set position to start_offset to keep al blocks same length except last block
                let start_offset = abs_start & block_mask;
                let mut new_cursor = Cursor::new(vec_data);
                new_cursor.set_position(start_offset);
                new_cursor
            } else if i == (end_block_idx - start_block_idx) {
                // last block  we have to truncate by end_offset and set position to 0
                let end_offset = abs_end & block_mask;
                let limited_data = &vec_data[..end_offset as usize];
                Cursor::new(limited_data)
            } else {
                // other block we can use full block ,so we only need set position to 0
                let mut new_cursor = Cursor::new(vec_data);
                new_cursor.set_position(0);
                new_cursor
            }
        })
        .collect()
}

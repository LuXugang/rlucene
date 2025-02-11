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
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::LuceneError;
use crate::util::long_values::{LongValues, Zeroes};
use std::sync::{Arc, Mutex};
/// Retrieves an instance previously written by `DirectWriter`.
///
/// # See also
/// [`DirectWriter`](crate::util::packed::direct_writer::DirectWriter)
pub struct DirectReader;
impl DirectReader {
    pub(crate) const MERGE_BUFFER_SHIFT: i32 = 7;
    const MERGE_BUFFER_SIZE: i32 = 1 << DirectReader::MERGE_BUFFER_SHIFT;
    const MERGE_BUFFER_MASK: i32 = DirectReader::MERGE_BUFFER_SIZE - 1;

    /// Retrieves an instance from the specified slice, decoding `bits_per_value` for each value.
    pub fn get_instance<R>(slice: Arc<Mutex<R>>, bits_per_value: i32) -> DirectPackedEnum<R>
    where
        R: RandomAccessInput,
    {
        Self::get_instance_with_offset(slice, bits_per_value, 0)
    }
    /// Retrieves an instance from the specified `offset` of the given slice, decoding `bits_per_value` for each value.
    pub fn get_instance_with_offset<R>(
        slice: Arc<Mutex<R>>,
        bits_per_value: i32,
        offset: i64,
    ) -> DirectPackedEnum<R>
    where
        R: RandomAccessInput,
    {
        match bits_per_value {
            1 => DirectPackedEnum::DirectPackedReader1(DirectPackedReader1::new(slice, offset)),
            2 => DirectPackedEnum::DirectPackedReader2(DirectPackedReader2::new(slice, offset)),
            4 => DirectPackedEnum::DirectPackedReader4(DirectPackedReader4::new(slice, offset)),
            8 => DirectPackedEnum::DirectPackedReader8(DirectPackedReader8::new(slice, offset)),
            12 => DirectPackedEnum::DirectPackedReader12(DirectPackedReader12::new(slice, offset)),
            16 => DirectPackedEnum::DirectPackedReader16(DirectPackedReader16::new(slice, offset)),
            20 => DirectPackedEnum::DirectPackedReader20(DirectPackedReader20::new(slice, offset)),
            24 => DirectPackedEnum::DirectPackedReader24(DirectPackedReader24::new(slice, offset)),
            28 => DirectPackedEnum::DirectPackedReader28(DirectPackedReader28::new(slice, offset)),
            32 => DirectPackedEnum::DirectPackedReader32(DirectPackedReader32::new(slice, offset)),
            40 => DirectPackedEnum::DirectPackedReader40(DirectPackedReader40::new(slice, offset)),
            48 => DirectPackedEnum::DirectPackedReader48(DirectPackedReader48::new(slice, offset)),
            56 => DirectPackedEnum::DirectPackedReader56(DirectPackedReader56::new(slice, offset)),
            64 => DirectPackedEnum::DirectPackedReader64(DirectPackedReader64::new(slice, offset)),
            _ => unreachable!(),
        }
    }
    /// Retrieves an instance specialized for merges, typically faster for sequential access but slower for random access.
    pub fn get_merge_instance<R>(
        slice: Arc<Mutex<R>>,
        bits_per_value: i32,
        num_values: i64,
    ) -> DirectPackedEnum<R>
    where
        R: RandomAccessInput,
    {
        Self::get_merge_instance_with_base_offset(slice, bits_per_value, 0, num_values)
    }
    /// Retrieves an instance specialized for merges, typically faster for sequential access.
    pub fn get_merge_instance_with_base_offset<R>(
        slice: Arc<Mutex<R>>,
        bits_per_value: i32,
        base_offset: i64,
        num_values: i64,
    ) -> DirectPackedEnum<R>
    where
        R: RandomAccessInput,
    {
        DirectPackedEnum::LongValuesImpl(LongValuesImpl::new(
            slice,
            bits_per_value,
            num_values,
            base_offset,
        ))
    }
}

struct LongValuesImpl<R>
where
    R: RandomAccessInput,
{
    slice: Arc<Mutex<R>>,
    bits_per_value: i32,
    num_values: i64,
    base_offset: i64,
    buffer: Vec<i64>,
    block_index: i64,
}
impl<R> LongValuesImpl<R>
where
    R: RandomAccessInput,
{
    fn new(
        slice: Arc<Mutex<R>>,
        bits_per_value: i32,
        num_values: i64,
        base_offset: i64,
    ) -> LongValuesImpl<R> {
        LongValuesImpl {
            slice,
            bits_per_value,
            num_values,
            base_offset,
            buffer: vec![0; DirectReader::MERGE_BUFFER_SIZE as usize],
            block_index: -1,
        }
    }

    fn fill_buffer(&mut self, index: i64) -> Result<(), LuceneError> {
        // NOTE: we're not allowed to read more than 3 bytes past the last value
        let mut slice = self
            .slice
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        if index >= self.num_values - DirectReader::MERGE_BUFFER_SIZE as i64 {
            // 128 values left or less
            let mut slow_instance = DirectReader::get_instance_with_offset(
                self.slice.clone(),
                self.bits_per_value,
                self.base_offset,
            );
            drop(slice);
            let num_values_last_block = (self.num_values - index) as usize;
            for i in 0..num_values_last_block {
                self.buffer[i] = slow_instance.get(index + i as i64)?;
            }
        } else if (self.bits_per_value & 0x07) == 0 {
            // bitsPerValue is a multiple of 8
            let bytes_per_value = self.bits_per_value / u8::BITS as i32;
            let mask = if self.bits_per_value == 64 {
                !0i64
            } else {
                (1i64 << self.bits_per_value) - 1
            };
            let mut offset = self.base_offset + (index * self.bits_per_value as i64) / 8;
            for i in 0..DirectReader::MERGE_BUFFER_SIZE as usize {
                if self.bits_per_value > i32::BITS as i32 {
                    self.buffer[i] = slice.read_long(offset)? & mask;
                } else if self.bits_per_value > i16::BITS as i32 {
                    self.buffer[i] = (slice.read_int(offset)? as u32 as i64) & mask;
                } else if self.bits_per_value > i8::BITS as i32 {
                    self.buffer[i] = slice.read_short(offset)? as u16 as i64;
                } else {
                    self.buffer[i] = slice.read_byte(offset)? as i64;
                }
                offset += bytes_per_value as i64;
            }
        } else if self.bits_per_value < 8 {
            // bitsPerValue is 1, 2 or 4
            let values_per_long = u64::BITS as i32 / self.bits_per_value;
            let mask = (1i64 << self.bits_per_value) - 1;
            let mut offset = self.base_offset + (index * self.bits_per_value as i64) / 8;
            let mut i = 0;
            for _ in 0..(2 * self.bits_per_value) {
                let bits = slice.read_long(offset)?;
                for j in 0..values_per_long {
                    self.buffer[i] = (bits as u64 >> (j * self.bits_per_value)) as i64 & mask;
                    i += 1;
                }
                offset += BitUtil::LONG_BYTES as i64;
            }
        } else {
            // bitsPerValue is 12, 20 or 28; read values 2 by 2
            let num_bytes_for_2_values = (self.bits_per_value * 2) / i8::BITS as i32;
            let mask = (1i64 << self.bits_per_value) - 1;
            let mut offset = self.base_offset + (index * self.bits_per_value as i64) / 8;
            for i in (0..DirectReader::MERGE_BUFFER_SIZE as usize).step_by(2) {
                let l = if num_bytes_for_2_values > BitUtil::INT_BYTES as i32 {
                    slice.read_long(offset)?
                } else {
                    slice.read_int(offset)? as i64
                };
                self.buffer[i] = l & mask;
                self.buffer[i + 1] = (l as u64 >> self.bits_per_value) as i64 & mask;
                offset += num_bytes_for_2_values as i64;
            }
        }
        Ok(())
    }
}
impl<R> LongValues for LongValuesImpl<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        debug_assert!(index >= 0);
        debug_assert!(index < self.num_values);
        let block_index = index >> DirectReader::MERGE_BUFFER_SHIFT;
        if self.block_index != block_index {
            self.fill_buffer(block_index << DirectReader::MERGE_BUFFER_SHIFT)?;
            self.block_index = block_index;
        }
        Ok(self.buffer[(index & DirectReader::MERGE_BUFFER_MASK as i64) as usize])
    }
}

struct DirectPackedReader1<R>
where
    R: RandomAccessInput,
{
    input: Arc<Mutex<R>>,
    offset: i64,
}
impl<R> DirectPackedReader1<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: Arc<Mutex<R>>, offset: i64) -> DirectPackedReader1<R> {
        DirectPackedReader1 { input, offset }
    }
}
impl<R> LongValues for DirectPackedReader1<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        let shift = (index & 7) as i32;
        let mut slice = self
            .input
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        let result = (slice.read_byte(self.offset + (index >> 3))? >> shift) & 0x1;
        Ok(result as i64)
    }
}

struct DirectPackedReader2<R>
where
    R: RandomAccessInput,
{
    input: Arc<Mutex<R>>,
    offset: i64,
}

impl<R> DirectPackedReader2<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: Arc<Mutex<R>>, offset: i64) -> Self {
        DirectPackedReader2 { input, offset }
    }
}

impl<R> LongValues for DirectPackedReader2<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        debug_assert!(index >= 0);
        let shift = ((index & 3) as i32) << 1;
        let mut slice = self
            .input
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        let byte = slice.read_byte(self.offset + (index >> 2))?;
        let result = (byte >> shift) & 0x3;
        Ok(result as i64)
    }
}

struct DirectPackedReader4<R>
where
    R: RandomAccessInput,
{
    input: Arc<Mutex<R>>,
    offset: i64,
}

impl<R> DirectPackedReader4<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: Arc<Mutex<R>>, offset: i64) -> Self {
        DirectPackedReader4 { input, offset }
    }
}

impl<R> LongValues for DirectPackedReader4<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        debug_assert!(index >= 0);
        let shift = ((index & 1) as i32) << 2;
        let mut slice = self
            .input
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        let byte = slice.read_byte(self.offset + (index >> 1))?;
        let result = (byte >> shift) & 0xF;
        Ok(result as i64)
    }
}

struct DirectPackedReader8<R>
where
    R: RandomAccessInput,
{
    input: Arc<Mutex<R>>,
    offset: i64,
}

impl<R> DirectPackedReader8<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: Arc<Mutex<R>>, offset: i64) -> Self {
        DirectPackedReader8 { input, offset }
    }
}

impl<R> LongValues for DirectPackedReader8<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        debug_assert!(index >= 0);
        let mut slice = self
            .input
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        let byte = slice.read_byte(self.offset + index)?;
        let result = byte;
        Ok(result as i64)
    }
}

struct DirectPackedReader12<R>
where
    R: RandomAccessInput,
{
    input: Arc<Mutex<R>>,
    offset: i64,
}

impl<R> DirectPackedReader12<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: Arc<Mutex<R>>, offset: i64) -> Self {
        DirectPackedReader12 { input, offset }
    }
}

impl<R> LongValues for DirectPackedReader12<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        debug_assert!(index >= 0);
        let off = (index * 12) >> 3;
        let shift = ((index & 1) as i32) << 2;
        let mut slice = self
            .input
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        let short_val = slice.read_short(self.offset + off)?;
        let result = ((short_val as u16) >> shift) & 0xFFF;
        Ok(result as i64)
    }
}

struct DirectPackedReader16<R>
where
    R: RandomAccessInput,
{
    input: Arc<Mutex<R>>,
    offset: i64,
}

impl<R> DirectPackedReader16<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: Arc<Mutex<R>>, offset: i64) -> Self {
        DirectPackedReader16 { input, offset }
    }
}

impl<R> LongValues for DirectPackedReader16<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        debug_assert!(index >= 0);
        let mut slice = self
            .input
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        let result = slice.read_short(self.offset + (index << 1))? as u16;
        Ok(result as i64)
    }
}
struct DirectPackedReader20<R>
where
    R: RandomAccessInput,
{
    input: Arc<Mutex<R>>,
    offset: i64,
}

impl<R> DirectPackedReader20<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: Arc<Mutex<R>>, offset: i64) -> Self {
        DirectPackedReader20 { input, offset }
    }
}

impl<R> LongValues for DirectPackedReader20<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        debug_assert!(index >= 0);
        let off = (index * 20) >> 3;
        let shift = ((index & 1) as i32) << 2;
        let mut slice = self
            .input
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        let int_val = slice.read_int(self.offset + off)?;
        let result = (int_val >> shift) & 0xFFFFF;
        Ok(result as i64)
    }
}

struct DirectPackedReader24<R>
where
    R: RandomAccessInput,
{
    input: Arc<Mutex<R>>,
    offset: i64,
}

impl<R> DirectPackedReader24<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: Arc<Mutex<R>>, offset: i64) -> Self {
        DirectPackedReader24 { input, offset }
    }
}

impl<R> LongValues for DirectPackedReader24<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        debug_assert!(index >= 0);
        let mut slice = self
            .input
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        let int_val = slice.read_int(self.offset + index * 3)?;
        let result = int_val & 0xFFFFFF;
        Ok(result as i64)
    }
}

struct DirectPackedReader28<R>
where
    R: RandomAccessInput,
{
    input: Arc<Mutex<R>>,
    offset: i64,
}

impl<R> DirectPackedReader28<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: Arc<Mutex<R>>, offset: i64) -> Self {
        DirectPackedReader28 { input, offset }
    }
}

impl<R> LongValues for DirectPackedReader28<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        debug_assert!(index >= 0);
        let off = (index * 28) >> 3;
        let shift = ((index & 1) as i32) << 2;
        let mut slice = self
            .input
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        let int_val = slice.read_int(self.offset + off)?;
        let result = (int_val >> shift) & 0xFFFFFFF;
        Ok(result as i64)
    }
}

struct DirectPackedReader32<R>
where
    R: RandomAccessInput,
{
    input: Arc<Mutex<R>>,
    offset: i64,
}

impl<R> DirectPackedReader32<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: Arc<Mutex<R>>, offset: i64) -> Self {
        DirectPackedReader32 { input, offset }
    }
}

impl<R> LongValues for DirectPackedReader32<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        debug_assert!(index >= 0);
        let mut slice = self
            .input
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        let int_val = slice.read_int(self.offset + (index << 2))?;
        let result = int_val as u32;
        Ok(result as i64)
    }
}

struct DirectPackedReader40<R>
where
    R: RandomAccessInput,
{
    input: Arc<Mutex<R>>,
    offset: i64,
}

impl<R> DirectPackedReader40<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: Arc<Mutex<R>>, offset: i64) -> Self {
        DirectPackedReader40 { input, offset }
    }
}

impl<R> LongValues for DirectPackedReader40<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        debug_assert!(index >= 0);
        let mut slice = self
            .input
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        let long_val = slice.read_long(self.offset + index * 5)?;
        let result = long_val & 0xFFFFFFFFFF;
        Ok(result)
    }
}

struct DirectPackedReader48<R>
where
    R: RandomAccessInput,
{
    input: Arc<Mutex<R>>,
    offset: i64,
}

impl<R> DirectPackedReader48<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: Arc<Mutex<R>>, offset: i64) -> Self {
        DirectPackedReader48 { input, offset }
    }
}

impl<R> LongValues for DirectPackedReader48<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        debug_assert!(index >= 0);
        let mut slice = self
            .input
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        let long_val = slice.read_long(self.offset + index * 6)?;
        let result = long_val & 0xFFFFFFFFFFFF;
        Ok(result)
    }
}

struct DirectPackedReader56<R>
where
    R: RandomAccessInput,
{
    input: Arc<Mutex<R>>,
    offset: i64,
}

impl<R> DirectPackedReader56<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: Arc<Mutex<R>>, offset: i64) -> Self {
        DirectPackedReader56 { input, offset }
    }
}

impl<R> LongValues for DirectPackedReader56<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        debug_assert!(index >= 0);
        let mut slice = self
            .input
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        let long_val = slice.read_long(self.offset + index * 7)?;
        let result = long_val & 0xFFFFFFFFFFFFFF;
        Ok(result)
    }
}

struct DirectPackedReader64<R>
where
    R: RandomAccessInput,
{
    input: Arc<Mutex<R>>,
    offset: i64,
}

impl<R> DirectPackedReader64<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: Arc<Mutex<R>>, offset: i64) -> Self {
        DirectPackedReader64 { input, offset }
    }
}

impl<R> LongValues for DirectPackedReader64<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        debug_assert!(index >= 0);
        let mut slice = self
            .input
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        let result = slice.read_long(self.offset + (index << 3))?;
        Ok(result)
    }
}

pub enum DirectPackedEnum<R>
where
    R: RandomAccessInput,
{
    DirectPackedReader1(DirectPackedReader1<R>),
    DirectPackedReader2(DirectPackedReader2<R>),
    DirectPackedReader4(DirectPackedReader4<R>),
    DirectPackedReader8(DirectPackedReader8<R>),
    DirectPackedReader12(DirectPackedReader12<R>),
    DirectPackedReader16(DirectPackedReader16<R>),
    DirectPackedReader20(DirectPackedReader20<R>),
    DirectPackedReader24(DirectPackedReader24<R>),
    DirectPackedReader28(DirectPackedReader28<R>),
    DirectPackedReader32(DirectPackedReader32<R>),
    DirectPackedReader40(DirectPackedReader40<R>),
    DirectPackedReader48(DirectPackedReader48<R>),
    DirectPackedReader56(DirectPackedReader56<R>),
    DirectPackedReader64(DirectPackedReader64<R>),
    LongValuesImpl(LongValuesImpl<R>),
    Zeroes(Zeroes),
}
impl<R> LongValues for DirectPackedEnum<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        match self {
            DirectPackedEnum::DirectPackedReader1(reader) => reader.get(index),
            DirectPackedEnum::DirectPackedReader2(reader) => reader.get(index),
            DirectPackedEnum::DirectPackedReader4(reader) => reader.get(index),
            DirectPackedEnum::DirectPackedReader8(reader) => reader.get(index),
            DirectPackedEnum::DirectPackedReader12(reader) => reader.get(index),
            DirectPackedEnum::DirectPackedReader16(reader) => reader.get(index),
            DirectPackedEnum::DirectPackedReader20(reader) => reader.get(index),
            DirectPackedEnum::DirectPackedReader24(reader) => reader.get(index),
            DirectPackedEnum::DirectPackedReader28(reader) => reader.get(index),
            DirectPackedEnum::DirectPackedReader32(reader) => reader.get(index),
            DirectPackedEnum::DirectPackedReader40(reader) => reader.get(index),
            DirectPackedEnum::DirectPackedReader48(reader) => reader.get(index),
            DirectPackedEnum::DirectPackedReader56(reader) => reader.get(index),
            DirectPackedEnum::DirectPackedReader64(reader) => reader.get(index),
            DirectPackedEnum::LongValuesImpl(reader) => reader.get(index),
            DirectPackedEnum::Zeroes(reader) => reader.get(index),
        }
    }
}

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
use crate::store::dummy::dummy_random_access_input::DummyRandomAccessInput;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::IndexInput;
use crate::util::error::lucene_error::LuceneError;
use crate::util::long_values::{LongValues, Zeroes};
use crate::util::packed::direct_reader::{DirectPackedEnum, DirectReader};
use std::sync::{Arc, Mutex};

pub struct DirectMonotonicReader<R>
where
    R: RandomAccessInput,
{
    block_shift: i32,
    block_mask: i64,
    readers: Vec<DirectPackedEnum<R>>,
    mins: Vec<i64>,
    avgs: Vec<f32>,
    bpvs: Vec<u8>,
}

impl<R> DirectMonotonicReader<R>
where
    R: RandomAccessInput,
{
    pub fn new(
        block_shift: i32,
        readers: Vec<DirectPackedEnum<R>>,
        mins: Vec<i64>,
        avgs: Vec<f32>,
        bpvs: Vec<u8>,
    ) -> Result<Self, LuceneError> {
        let readers_len = readers.len();
        if readers_len != mins.len() || readers_len != avgs.len() || readers_len != bpvs.len() {
            return Err(LuceneError::illegal_argument(String::from(
                "Mismatched array lengths",
            )));
        }
        let block_mask = (1i64 << block_shift) - 1;
        Ok(DirectMonotonicReader {
            block_shift,
            block_mask,
            readers,
            mins,
            avgs,
            bpvs,
        })
    }

    fn get_bounds(&self, index: i64) -> Result<[i64; 2], LuceneError> {
        match i32::try_from((index as u64) >> self.block_shift) {
            Ok(block) => {
                let block = block as usize;
                let block_index = index & self.block_mask;
                let lower_bound =
                    self.mins[block] + ((self.avgs[block] * (block_index as f32)) as i64);
                let upper_bound = lower_bound + ((1i64 << (self.bpvs[block] as u32)) - 1);
                if self.bpvs[block] == 64 || upper_bound < lower_bound {
                    Ok([i64::MIN, i64::MAX])
                } else {
                    Ok([lower_bound, upper_bound])
                }
            }
            Err(_) => Err(LuceneError::integer_overflow(format!(
                "value: {} is too large",
                index
            ))),
        }
    }

    pub fn binary_search(
        &mut self,
        from_index: i64,
        to_index: i64,
        key: i64,
    ) -> Result<i64, LuceneError> {
        if from_index < 0 || from_index > to_index {
            return Err(LuceneError::illegal_argument(format!(
                "fromIndex={}, toIndex={}",
                from_index, to_index
            )));
        }
        let mut lo = from_index;
        let mut hi = to_index - 1;
        while lo <= hi {
            let mid = (lo + hi) >> 1;
            // Try to run as many iterations of the binary search as possible without
            // hitting the direct readers, since they might hit a page fault.
            let bounds = self.get_bounds(mid)?;
            if bounds[1] < key {
                lo = mid + 1;
            } else if bounds[0] > key {
                hi = mid - 1;
            } else {
                let mid_val = self.get(mid)?;
                if mid_val < key {
                    lo = mid + 1;
                } else if mid_val > key {
                    hi = mid - 1;
                } else {
                    return Ok(mid);
                }
            }
        }
        Ok(-1 - lo)
    }
    pub fn get_instance(meta: &Meta, data: Arc<Mutex<R>>) -> Result<Self, LuceneError> {
        Self::get_instance_with_merging(meta, data, false)
    }

    pub fn get_instance_with_merging(
        meta: &Meta,
        data: Arc<Mutex<R>>,
        merging: bool,
    ) -> Result<Self, LuceneError> {
        let mut readers = Vec::with_capacity(meta.num_blocks);
        for i in 0..meta.num_blocks {
            let bpv = meta.bpvs[i];
            if bpv == 0 {
                readers.push(DirectPackedEnum::Zeroes(Zeroes));
            } else if merging
                && i < meta.num_blocks - 1// we only know the number of values for the last block
                && meta.block_shift >= DirectReader::MERGE_BUFFER_SHIFT
            {
                readers.push(DirectReader::get_merge_instance_with_base_offset(
                    data.clone(),
                    bpv as i32,
                    meta.offsets[i],
                    1i64 << meta.block_shift,
                ));
            } else {
                readers.push(DirectReader::get_instance_with_offset(
                    data.clone(),
                    bpv as i32,
                    meta.offsets[i],
                ));
            }
        }
        DirectMonotonicReader::new(
            meta.block_shift,
            readers,
            meta.mins.clone(),
            meta.avgs.clone(),
            meta.bpvs.clone(),
        )
    }
}
impl<R> LongValues for DirectMonotonicReader<R>
where
    R: RandomAccessInput,
{
    fn get(&mut self, index: i64) -> Result<i64, LuceneError> {
        let block = ((index as u64) >> self.block_shift) as usize;
        let block_index = index & self.block_mask;
        let delta = self.readers[block].get(block_index)?;
        Ok(self.mins[block] + ((self.avgs[block] * (block_index as f32)) as i64) + delta)
    }
}

pub struct Meta {
    pub block_shift: i32,
    pub num_blocks: usize,
    pub mins: Vec<i64>,
    pub avgs: Vec<f32>,
    pub bpvs: Vec<u8>,
    pub offsets: Vec<i64>,
}

impl Meta {
    pub fn new(num_values: i64, block_shift: i32) -> Self {
        let mut num_blocks = (num_values as u64) >> (block_shift as u32);
        if (num_blocks << block_shift) < num_values as u64 {
            num_blocks += 1;
        }
        let num_blocks_usize = num_blocks as usize;
        Meta {
            block_shift,
            num_blocks: num_blocks_usize,
            mins: vec![0; num_blocks_usize],
            avgs: vec![0.0; num_blocks_usize],
            bpvs: vec![0; num_blocks_usize],
            offsets: vec![0; num_blocks_usize],
        }
    }

    pub fn single_zero_block() -> Self {
        Meta::new(1, 63)
    }

    pub fn load_meta<I>(
        meta_in: &mut I,
        num_values: i64,
        block_shift: i32,
    ) -> Result<Self, LuceneError>
    where
        I: IndexInput,
    {
        let mut all_values_zero = true;
        let mut meta = Meta::new(num_values, block_shift);
        for i in 0..meta.num_blocks {
            let min = meta_in.read_long()?;
            meta.mins[i] = min;
            let avg_int = meta_in.read_int()?;
            meta.avgs[i] = f32::from_bits(avg_int as u32);
            meta.offsets[i] = meta_in.read_long()?;
            let bpv = meta_in.read_byte()?;
            meta.bpvs[i] = bpv;
            all_values_zero = all_values_zero && (min == 0) && (avg_int == 0) && (bpv == 0);
        }
        if all_values_zero {
            Ok(Meta::single_zero_block())
        } else {
            Ok(meta)
        }
    }
}

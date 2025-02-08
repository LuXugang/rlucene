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
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::LuceneError;
use once_cell::sync::Lazy;
use std::sync::atomic::AtomicI32;

#[allow(unused)]
pub static ITERATION_SEED: Lazy<AtomicI32> = Lazy::new(|| AtomicI32::new(0));
/// Constants and utility functions for hash containers.
pub struct HashContainers;

#[allow(unused)]
impl HashContainers {
    pub const DEFAULT_EXPECTED_ELEMENTS: i32 = 4;
    pub const DEFAULT_LOAD_FACTOR: f64 = 0.75;
    /// Minimal sane load factor (99 empty slots per 100).
    pub const MIN_LOAD_FACTOR: f64 = 1.0 / 100.0;
    /// Maximum sane load factor (1 empty slot per 100).
    pub const MAX_LOAD_FACTOR: f64 = 99.0 / 100.0;
    /// Minimum hash buffer size.
    pub const MIN_HASH_ARRAY_LENGTH: i32 = 4;
    /// Maximum array size for hash containers
    pub const MAX_HASH_ARRAY_LENGTH: u32 = 0x80000000 >> 1;

    pub fn iteration_increment(seed: i32) -> i32 {
        29 + ((seed & 7) << 1) // Small odd integer.
    }
    pub fn next_buffer_size(
        array_size: i32,
        elements: i32,
        load_factor: f64,
    ) -> Result<i32, LuceneError> {
        debug_assert!(
            Self::check_power_of_two(array_size),
            "Array size must be a power of two."
        );

        if array_size as u32 == Self::MAX_HASH_ARRAY_LENGTH {
            return Err(LuceneError::buffer_allocation(format!(
                "Maximum array size exceeded for this load factor (elements: {}, load factor: {})",
                elements, load_factor
            )));
        }

        Ok(array_size << 1)
    }
    pub fn expand_at_count(array_size: i32, load_factor: f64) -> i32 {
        debug_assert!(Self::check_power_of_two(array_size));
        // Take care of hash container invariant (there has to be at least one empty slot to ensure
        // the lookup loop finds either the element or an empty slot).
        i32::min(
            array_size - 1,
            (array_size as f64 * load_factor).ceil() as i32,
        )
    }

    fn check_power_of_two(array_size: i32) -> bool {
        // These are internals, we can just assert without retrying.
        assert!(array_size > 1);
        assert_eq!(
            BitUtil::next_highest_power_of_two_with_i32(array_size),
            array_size
        );
        true
    }
    /// Computes the minimum buffer size based on the number of elements and load factor.
    /// Ensures the size is a power of two and within allowable limits.
    pub fn min_buffer_size(elements: i32, load_factor: f64) -> Result<i32, LuceneError> {
        if elements < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "Number of elements must be >= 0: {}",
                elements
            )));
        }

        let mut length = ((elements as f64 / load_factor).ceil()) as i64;

        if length == elements as i64 {
            length += 1;
        }

        length = i64::max(
            Self::MIN_HASH_ARRAY_LENGTH as i64,
            BitUtil::next_highest_power_of_two_with_i64(length),
        );

        if length > Self::MAX_HASH_ARRAY_LENGTH as i64 {
            return Err(LuceneError::buffer_allocation(format!(
                "Maximum array size exceeded for this load factor (elements: {}, load factor: {})",
                elements, load_factor
            )));
        }

        Ok(length as i32)
    }
    pub fn check_load_factor(
        load_factor: f64,
        min_allowed_inclusive: f64,
        max_allowed_inclusive: f64,
    ) -> Result<(), LuceneError> {
        if load_factor < min_allowed_inclusive || load_factor > max_allowed_inclusive {
            return Err(LuceneError::buffer_allocation(format!(
                "The load factor should be in range [{:.2}, {:.2}]: {:.2}",
                min_allowed_inclusive, max_allowed_inclusive, load_factor
            )));
        }
        Ok(())
    }
}

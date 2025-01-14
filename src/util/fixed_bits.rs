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
use crate::util::bits::Bits;

pub struct FixedBits<'a> {
    bits: &'a Vec<i64>,
    length: i32,
}
impl<'a> FixedBits<'a> {
    pub fn new(bits: &'a Vec<i64>, length: i32) -> FixedBits<'a> {
        FixedBits { bits, length }
    }
}
impl Bits for FixedBits<'_> {
    fn get(&self, index: i32) -> bool {
        debug_assert!(
            index >= 0 && index < self.length,
            "index = {}, num_bits = {}",
            index,
            self.length
        );
        let i = index >> 6;
        // signed shift will keep a negative index and force an
        // array-index-out-of-bounds-exception, removing the need for an explicit check.
        let bit_mask = 1_u64 << (index % 64);
        debug_assert!(bit_mask <= i64::MAX as u64);
        (bit_mask as i64 & self.bits[i as usize]) != 0
    }

    fn length(&self) -> i32 {
        self.length
    }
}

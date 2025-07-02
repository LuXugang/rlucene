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

/// Interface for `BitSet`-like structures.
///
/// # Note
/// This is an experimental API.
pub trait Bits {
    /// Returns the value of the bit at the specified `index`.
    ///
    /// # Arguments
    /// * `index` - The index should be non-negative and less than the length of
    ///   the bitset. Passing negative or out-of-bounds values results in
    ///   undefined behavior—**just don't do it!**
    ///
    /// # Returns
    /// `true` if the bit is set, `false` otherwise.
    fn get(&self, index: i32) -> bool;

    /// Returns the number of bits in this set
    fn length(&self) -> i32;
}

/// Bits impl of the specified length with all bits set.
pub struct MatchAllBits {
    len: i32,
}
impl MatchAllBits {
    pub fn new(len: i32) -> MatchAllBits {
        MatchAllBits { len }
    }
}
impl Bits for MatchAllBits {
    fn get(&self, _index: i32) -> bool {
        true
    }

    fn length(&self) -> i32 {
        self.len
    }
}

/// Bits impl of the specified length with no bits set.
#[derive(Default)]
pub struct MatchNoBits {
    len: i32,
}
impl Bits for MatchNoBits {
    fn get(&self, _index: i32) -> bool {
        false
    }

    fn length(&self) -> i32 {
        self.len
    }
}

pub enum BitsEnum {}
impl Bits for BitsEnum {
    fn get(&self, _index: i32) -> bool {
        todo!()
    }

    fn length(&self) -> i32 {
        todo!()
    }
}

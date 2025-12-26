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
use crate::core::util::bit_set::BitSet;
use crate::core::util::fixed_bit_set::FixedBitSet;
use std::sync::Arc;
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

    /// Make a copy of the given bits.
    fn copy_of(&self) -> FixedBitSet {
        let length = self.length();
        let mut bit_set = FixedBitSet::new(length);
        bit_set.set_with_range(0, length);
        for i in 0..length {
            if !self.get(i) {
                bit_set.clear_with_index(i);
            }
        }
        bit_set
    }
    fn as_string(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }
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

pub enum BitsEnum2<A, B> {
    A(A),
    B(B),
}
impl<A, B> Bits for BitsEnum2<A, B>
where
    A: Bits,
    B: Bits,
{
    fn get(&self, index: i32) -> bool {
        match self {
            BitsEnum2::A(t) => t.get(index),
            BitsEnum2::B(s) => s.get(index),
        }
    }

    fn length(&self) -> i32 {
        match self {
            BitsEnum2::A(t) => t.length(),
            BitsEnum2::B(s) => s.length(),
        }
    }

    fn copy_of(&self) -> FixedBitSet {
        match self {
            BitsEnum2::A(t) => t.copy_of(),
            BitsEnum2::B(s) => s.copy_of(),
        }
    }

    fn as_string(&self) -> String {
        match self {
            BitsEnum2::A(t) => t.as_string(),
            BitsEnum2::B(s) => s.as_string(),
        }
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

impl<T> Bits for Arc<T>
where
    T: Bits,
{
    fn get(&self, index: i32) -> bool {
        (**self).get(index)
    }

    fn length(&self) -> i32 {
        (**self).length()
    }

    fn copy_of(&self) -> FixedBitSet {
        (**self).copy_of()
    }
    fn as_string(&self) -> String {
        (**self).as_string()
    }
}

impl<T: Bits + ?Sized> Bits for &T {
    fn get(&self, index: i32) -> bool {
        <T as Bits>::get(*self, index)
    }

    fn length(&self) -> i32 {
        <T as Bits>::length(*self)
    }

    fn copy_of(&self) -> FixedBitSet {
        <T as Bits>::copy_of(*self)
    }

    fn as_string(&self) -> String {
        <T as Bits>::as_string(*self)
    }
}

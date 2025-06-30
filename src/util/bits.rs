/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
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

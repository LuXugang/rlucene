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
        // array-index-out-of-bounds-exception, removing the need for an
        // explicit check.
        let bit_mask = 1_u64 << (index % 64);
        debug_assert!(bit_mask <= i64::MAX as u64);
        (bit_mask as i64 & self.bits[i as usize]) != 0
    }

    fn length(&self) -> i32 {
        self.length
    }
}

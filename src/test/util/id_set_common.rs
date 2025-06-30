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
// TODO: should with mask
pub fn flip_bit_range(bitset: &mut bit_set::BitSet, start: usize, end: usize) {
    for i in start..end {
        flip_bit(bitset, i);
    }
}

// TODO: should with mask
pub fn clear_range(bitset: &mut bit_set::BitSet, start: usize, end: usize) {
    for i in start..end {
        bitset.remove(i);
    }
}
// TODO: should with mask
pub fn set_range(bitset: &mut bit_set::BitSet, start: usize, end: usize) {
    for i in start..end {
        bitset.insert(i);
    }
}
pub fn flip_bit(bitset: &mut bit_set::BitSet, index: usize) {
    if bitset.contains(index) {
        bitset.remove(index);
    } else {
        bitset.insert(index);
    }
}

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
use crate::index::point_values::PointTree;
use crate::index::BytesRef;

/// One leaf [PointTree] whose order of points can be changed.
/// This trait is useful for codecs to optimize flush.
pub trait MutablePointTree: PointTree {
    /// Set `packed_value` with a reference to the packed bytes of the i-th
    /// value.
    fn get_value(&self, i: usize, packed_value: &mut BytesRef<Vec<u8>>);

    /// Get the k-th byte of the i-th value.
    fn get_byte_at(&self, i: usize, k: usize) -> u8;

    /// Return the doc ID of the i-th value.
    fn get_doc_id(&self, i: usize) -> i32;

    /// Swap the i-th and j-th values.
    fn swap(&mut self, i: usize, j: usize);

    /// Save the i-th value into the j-th position in temporary storage.
    fn save(&mut self, i: usize, j: usize);

    /// Restore values between i-th and j-th (excluding) in temporary storage
    /// into original storage.
    fn restore(&mut self, i: usize, j: usize);
}

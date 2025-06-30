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
use std::rc::Rc;

pub(crate) trait IntSet {
    /// Returns a slice (`&[i32]`) representation of this int set's values.
    /// Values are valid for indices `[0, size()]`.
    /// If this is a mutable int set, then changes to the set are not guaranteed
    /// to be visible in this slice.
    ///
    /// Returns:
    /// - A slice containing the values for this set, guaranteed to have at
    ///   least [`size()`](Self::size) elements.
    fn get_array(&mut self) -> &Rc<Vec<i32>>;

    /// Returns the number of values in this set.
    /// Guaranteed to be less than or equal to the length of the slice returned
    /// by [`get_array`](Self::get_array).
    ///
    /// Returns:
    /// - The number of values in this set.
    fn size(&self) -> usize;

    /// Computes a long (i64) hash code for this set.
    fn long_hash_code(&mut self) -> i64;
}

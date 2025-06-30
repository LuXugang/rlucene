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
pub(crate) trait SliceCopyOps<T> {
    /// Copies elements from a source slice (`src`) into the current slice
    /// (`self`) starting at the specified offset.
    ///
    /// # Parameters
    /// - `self`: The destination mutable slice where the elements will be
    ///   copied to.
    /// - `src`: The source slice containing the elements to copy.
    /// - `Offset`: The starting position in the destination slice where the
    ///   copy begins.
    ///
    /// # Panics
    /// This function does not panic during runtime in release builds. However,
    /// it includes a `debug_assert!` in debug mode to ensure that `offset +
    /// src.len()` does not exceed the length of the destination slice (`self`).
    /// If the assertion fails, it indicates an out-of-bounds access.
    ///
    /// # Safety
    /// This function uses `unsafe` code to call
    /// `std::ptr::copy_nonoverlapping`, which performs unchecked memory
    /// operations. You must ensure that:
    /// - The destination slice has enough space to accommodate the copied
    ///   elements.
    /// - The `src` and the destination slice (from `offset`) do not overlap.
    fn copy_from(&mut self, src: &[T], offset: usize);
}

impl<T> SliceCopyOps<T> for Vec<T> {
    #[inline]
    fn copy_from(&mut self, src: &[T], offset: usize) {
        self.as_mut_slice().copy_from(src, offset)
    }
}
impl<T> SliceCopyOps<T> for [T] {
    fn copy_from(&mut self, src: &[T], offset: usize) {
        debug_assert!(
            offset + src.len() <= self.len(),
            "Copy out of bounds: offset={}, src_len={}, buffer_len={}",
            offset,
            src.len(),
            self.len()
        );

        unsafe {
            std::ptr::copy_nonoverlapping(src.as_ptr(), self.as_mut_ptr().add(offset), src.len());
        }
    }
}

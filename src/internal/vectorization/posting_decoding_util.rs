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
use std::cell::RefCell;
use std::rc::Rc;

use crate::store::IndexInput;
use crate::util::error::lucene_error::Result;
/// Utility struct to decode postings.
pub struct PostingDecodingUtil<I: IndexInput> {
    /// The wrapper {@link IndexInput}.
    pub input: Rc<RefCell<I>>,
}
#[allow(unused)]
impl<I: IndexInput> PostingDecodingUtil<I> {
    /// Sole constructor, called by sub-classes.
    pub fn new(input: Rc<RefCell<I>>) -> Self {
        PostingDecodingUtil { input }
    }

    /// Core method for decoding blocks of docs / freqs / positions / offsets:
    ///
    /// - Read `count` longs into `c[c_index..]`
    /// - For all `i >= 0` such that `b_shift - i * dec > 0`:
    ///   - Apply shift `b_shift - i * dec` to each value in `c`
    ///   - Store the result in `b` at offset `count * i`
    /// - Apply mask `c_mask` to each value in `c` starting at `c_index`
    #[allow(clippy::too_many_arguments)]
    pub fn split_ints_same(
        &mut self,
        count: i32,
        b_and_c: &mut [i32],
        b_shift: i32,
        dec: i32,
        b_mask: i32,
        c_index: i32,
        c_mask: i32,
    ) -> Result<()> {
        self.input.borrow_mut().read_ints(b_and_c, c_index, count)?;

        let count = count as usize;
        let c_index = c_index as usize;
        let max_iter = (b_shift - 1) / dec;
        for i in 0..count {
            for j in 0..=max_iter {
                let shift = b_shift - j * dec;
                if shift > 0 {
                    b_and_c[count * j as usize + i] =
                        ((b_and_c[c_index + i] as u64) >> shift) as i32 & b_mask;
                }
            }
            b_and_c[c_index + i] &= c_mask;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn split_ints_diff(
        &mut self,
        count: i32,
        b: &mut [i32],
        b_shift: i32,
        dec: i32,
        b_mask: i32,
        c: &mut [i32],
        c_index: i32,
        c_mask: i32,
    ) -> Result<()> {
        self.input.borrow_mut().read_ints(c, c_index, count)?;
        let count = count as usize;
        let c_index = c_index as usize;
        let max_iter = (b_shift - 1) / dec;
        for i in 0..count {
            for j in 0..=max_iter {
                let shift = b_shift - j * dec;
                if shift > 0 {
                    b[count * j as usize + i] = ((c[c_index + i] as u64) >> shift) as i32 & b_mask;
                }
            }
            c[c_index + i] &= c_mask;
        }

        Ok(())
    }
}

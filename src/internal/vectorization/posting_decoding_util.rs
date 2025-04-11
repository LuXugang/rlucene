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
use crate::store::IndexInput;
use crate::util::error::lucene_error::Result;
use std::cell::RefCell;
use std::rc::Rc;
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
    pub fn split_longs_same(
        &mut self,
        count: i32,
        b_and_c: &mut [i64],
        b_shift: i32,
        dec: i32,
        b_mask: i64,
        c_index: i32,
        c_mask: i64,
    ) -> Result<()> {
        self.input
            .borrow_mut()
            .read_longs(b_and_c, c_index, count)?;

        let count = count as usize;
        let c_index = c_index as usize;
        let max_iter = (b_shift - 1) / dec;
        for i in 0..count {
            for j in 0..=max_iter {
                let shift = b_shift - j * dec;
                if shift > 0 {
                    b_and_c[count * j as usize + i] =
                        ((b_and_c[c_index + i] as u64) >> shift) as i64 & b_mask;
                }
            }
            b_and_c[c_index + i] &= c_mask;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn split_longs_diff(
        &mut self,
        count: i32,
        b: &mut [i64],
        b_shift: i32,
        dec: i32,
        b_mask: i64,
        c: &mut [i64],
        c_index: i32,
        c_mask: i64,
    ) -> Result<()> {
        self.input.borrow_mut().read_longs(c, c_index, count)?;
        let count = count as usize;
        let c_index = c_index as usize;
        let max_iter = (b_shift - 1) / dec;
        for i in 0..count {
            for j in 0..=max_iter {
                let shift = b_shift - j * dec;
                if shift > 0 {
                    b[count * j as usize + i] = ((c[c_index + i] as u64) >> shift) as i64 & b_mask;
                }
            }
            c[c_index + i] &= c_mask;
        }

        Ok(())
    }
}

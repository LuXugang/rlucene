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
use crate::index::impact::Impact;
use crate::util::error::lucene_error::Result;
use std::borrow::Cow;

/// Information about upcoming impacts, i.e., (freq, norm) pairs.
pub trait Impacts {
    /// Return the number of levels on which we have impacts.
    ///
    /// The returned value is always greater than 0 and may not always be the
    /// same, even on a single postings list, depending on the current doc
    /// ID.
    fn num_levels(&self) -> i32;

    /// Return the maximum inclusive doc ID until which the list of impacts
    /// returned by `get_impacts(level)` is valid.
    ///
    /// This is a non-decreasing function of `level`.
    fn get_doc_id_up_to(&self, level: i32) -> i32;

    /// Return impacts on the given level.
    ///
    /// These impacts are sorted by increasing frequency and increasing unsigned
    /// norm, and only valid until the doc ID returned by
    /// `get_doc_id_up_to(level)` (inclusive).
    ///
    /// The returned list is never empty and should behave like `RandomAccess`
    /// if it contains more than one element.
    ///
    /// NOTE: There is no guarantee that these impacts actually appear in
    /// postings, only that they trigger scores that are greater than or
    /// equal to the impacts that actually appear in postings.
    fn get_impacts(&mut self, level: i32) -> Result<Cow<[Impact]>>;
}

pub enum ImpactsEnums {}
impl Impacts for ImpactsEnums {
    fn num_levels(&self) -> i32 {
        todo!()
    }

    fn get_doc_id_up_to(&self, _level: i32) -> i32 {
        todo!()
    }

    fn get_impacts(&mut self, _level: i32) -> Result<Cow<[Impact]>> {
        todo!()
    }
}

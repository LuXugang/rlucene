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
use crate::index::impacts::Impacts;
use crate::util::error::lucene_error::Result;

/// Source of `Impacts`.
///
/// NOTE: Advancing the iterator may invalidate the returned impacts,
/// so they should not be used after the iterator has been advanced.
pub trait ImpactsSource {
    /// Shallow-advance to `target`.
    ///
    /// This is cheaper than calling
    /// [`advance(target)`](crate::search::doc_id_set_iterator::DocIdSetIterator::advance)
    /// and allows further calls
    /// to [`get_impacts()`](ImpactsSource::get_impacts) to ignore doc IDs that
    /// are less than `target` in order to get more precise information
    /// about impacts.
    ///
    /// This method may not be called on targets that are less than the current
    /// [`doc_id()`](crate::search::doc_id_set_iterator::DocIdSetIterator::doc_id).
    /// After this method has been called,
    /// [`next_doc()`](crate::search::doc_id_set_iterator::DocIdSetIterator::next_doc)
    /// may not be called if the current doc ID is less than `target - 1`,
    /// and [`advance(target)`](crate::search::doc_id_set_iterator::DocIdSetIterator::advance)
    /// may not be called on targets that are less than `target`.
    fn advance_shallow(&mut self, target: i32) -> Result<()>;

    type Impacts: Impacts;
    /// Get information about upcoming impacts for doc IDs greater than or equal
    /// to the max of the current
    /// [`doc_id()`](crate::search::doc_id_set_iterator::DocIdSetIterator::doc_id)
    /// and the last target passed to
    /// [`advance_shallow()`](ImpactsSource::advance_shallow).
    ///
    /// This method may not be called on an unpositioned iterator where
    /// [`advance_shallow()`](ImpactsSource::advance_shallow) has never been
    /// called. #Note :
    ///  advancing this iterator may
    ///   invalidate the returned impacts, so they should not be used after the
    /// iterator has been advanced.
    fn get_impacts(&mut self) -> Result<Self::Impacts>;
}

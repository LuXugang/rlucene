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
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::util::error::lucene_error::{LuceneError, Result};

/// This abstract struct defines methods to iterate over a set of non-decreasing
/// document IDs. Note that this struct assumes it iterates on document IDs, and
/// therefore [`NO_MORE_DOCS`] is set to its constant value to be used as a
/// sentinel object.
///
/// Implementations of this struct are expected to treat `i32::MAX` as an
/// invalid value.
pub trait DocIdSetIterator {
    /// Returns the following:
    ///
    /// - `-1` if [`next_doc`](DocIdSetIterator::next_doc) or
    ///   [`advance`](DocIdSetIterator::advance) has not been called yet.
    /// - [`NO_MORE_DOCS`]if the iterator has been exhausted.
    /// - Otherwise, it returns the document ID it is currently on.
    fn doc_id(&self) -> i32;
    /// Advances to the next document in the set and returns the document ID it
    /// is currently on, or [`NO_MORE_DOCS`] if there are no more documents
    /// in the set.
    ///
    /// # Note
    /// After the iterator has been exhausted, you should not call this method,
    /// as it may result in undefined behavior.
    fn next_doc(&mut self) -> Result<i32>;
    /// Advances to the first document beyond the current one whose document
    /// number is greater than or equal to the `target`, and returns the
    /// document number itself. If `target` is greater than the
    /// highest document number in the set, the iterator is exhausted, and
    /// [`NO_MORE_DOCS`] is returned.
    ///
    /// # Undefined Behavior
    /// The behavior of this method is **undefined** when called with `target <=
    /// current`, or after the iterator has been exhausted. Both cases may
    /// result in unpredictable behavior.
    ///
    /// # Behavior for `target > current`
    /// When `target > current`, it behaves similarly to:
    ///
    /// ```text
    /// fn advance(target: i32) -> i32 {
    ///     let mut doc;
    ///     while {
    ///         doc = next_doc();
    ///         doc < target
    ///     } {}
    ///     doc
    /// }
    /// ```
    ///
    /// Some implementations may be significantly more efficient than this.
    ///
    /// # Note
    /// This method may be called with [`NO_MORE_DOCS`] for efficiency
    /// by some Scorers. If your implementation cannot efficiently determine
    /// that it should exhaust, it is recommended to check for this value in
    /// each call to this method.
    fn advance(&mut self, _target: i32) -> Result<i32> {
        unimplemented!("advance() must be implemented if it need to be used")
    }
    /// A slow (linear) implementation of [`advance`](DocIdSetIterator::advance)
    /// that relies on [`next_doc`](DocIdSetIterator::next_doc) to move
    /// beyond the target position.
    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        debug_assert!(self.doc_id() < target);
        let mut doc;
        loop {
            doc = self.next_doc()?;
            if doc >= target {
                break;
            }
        }
        Ok(doc)
    }
    /// Returns the estimated cost of this [`DocIdSetIterator`].
    /// This is generally an upper bound on the number of documents this
    /// iterator might match, but it may also be a rough heuristic, a
    /// hardcoded value, or otherwise completely inaccurate.
    fn cost(&self) -> Result<i64> {
        unimplemented!("cost() must be implemented if it need to be used")
    }
}

///An empty [`DocIdSetIterator`]
pub struct EmptyDISI {
    exhausted: bool,
}
impl Default for EmptyDISI {
    fn default() -> Self {
        Self::new()
    }
}

impl EmptyDISI {
    pub fn new() -> Self {
        Self { exhausted: false }
    }
}
impl DocIdSetIterator for EmptyDISI {
    fn doc_id(&self) -> i32 {
        if self.exhausted {
            NO_MORE_DOCS
        } else {
            -1
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        debug_assert!(!self.exhausted);
        self.exhausted = true;
        Ok(NO_MORE_DOCS)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        debug_assert!(!self.exhausted);
        debug_assert!(_target >= 0);
        self.exhausted = true;
        Ok(NO_MORE_DOCS)
    }

    fn cost(&self) -> Result<i64> {
        Ok(0)
    }
}

/// A [`DocIdSetIterator`] that matches all documents up to `maxDoc - 1`.  */
pub struct AllDocIdSetIterator {
    doc: i32,
    max_doc: i32,
}
impl AllDocIdSetIterator {
    pub fn new(max_doc: i32) -> Self {
        AllDocIdSetIterator { doc: -1, max_doc }
    }
}
impl DocIdSetIterator for AllDocIdSetIterator {
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc + 1)?;
        Ok(self.doc)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.doc = target;
        if self.doc >= self.max_doc {
            self.doc = NO_MORE_DOCS
        }
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.max_doc as i64)
    }
}

/// A [`DocIdSetIterator`] that matches a range of documents from `min_doc_id`
/// (inclusive) to `max_doc_id` (exclusive).
///
/// # Parameters
/// - `min_doc_id`: The minimum document ID to match (inclusive).
/// - `max_doc_id`: The maximum document ID to match (exclusive).
///
/// # See Also
/// - [`DocIdSetIterator`]
pub struct Range {
    doc: i32,
    min_doc: i32,
    max_doc: i32,
}
impl Range {
    pub fn new(min_doc: i32, max_doc: i32) -> Result<Range> {
        if min_doc >= max_doc {
            return Err(LuceneError::illegal_argument(format!(
                "minDoc must be < maxDoc but got minDoc= {min_doc} maxDoc= {max_doc}"
            )));
        }
        if min_doc < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "minDoc must be >= 0 but got minDoc= {min_doc}"
            )));
        }
        Ok(Range {
            doc: -1,
            min_doc,
            max_doc,
        })
    }
}
impl DocIdSetIterator for Range {
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc + 1)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        if target < self.min_doc {
            self.doc = self.min_doc;
        } else if target >= self.max_doc {
            self.doc = NO_MORE_DOCS
        } else {
            self.doc = target
        }
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok((self.max_doc - self.min_doc) as i64)
    }
}
pub mod disi_const {
    /// When returned by
    /// [`next_doc`](crate::search::doc_id_set_iterator::DocIdSetIterator::next_doc),
    /// [`advance`](crate::search::doc_id_set_iterator::DocIdSetIterator::advance),
    /// and [`doc_id`](crate::search::doc_id_set_iterator::DocIdSetIterator::doc_id),
    /// it means there are no more documents in the iterator.
    pub const NO_MORE_DOCS: i32 = i32::MAX;
}

#[cfg(test)]
mod tests {
    use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::search::doc_id_set_iterator::{DocIdSetIterator, Range};
    use crate::util::error::lucene_error::Result;

    #[allow(dead_code)] // for quick search
    struct TestDocIdSetIterator {}
    #[test]
    fn test_range_basic() -> Result<()> {
        let result = Range::new(5, 8);
        assert!(result.is_ok());
        let mut disi = result?;
        assert_eq!(-1, disi.doc_id());
        assert_eq!(5, disi.next_doc()?);
        assert_eq!(6, disi.next_doc()?);
        assert_eq!(7, disi.next_doc()?);
        assert_eq!(NO_MORE_DOCS, disi.next_doc()?);
        Ok(())
    }

    #[test]
    fn test_invalid_range() {
        let disi_result = Range::new(5, 4);
        assert!(disi_result.is_err());
    }

    #[test]
    fn test_invalid_min() {
        let disi_result = Range::new(-1, 4);
        assert!(disi_result.is_err());
    }

    #[test]
    fn test_empty() {
        let disi_result = Range::new(7, 7);
        assert!(disi_result.is_err());
    }

    #[test]
    fn test_advance() -> Result<()> {
        let disi_result = Range::new(5, 20);
        assert!(disi_result.is_ok());
        let mut disi = disi_result?;
        assert_eq!(-1, disi.doc_id());
        assert_eq!(5, disi.next_doc()?);
        assert_eq!(17, disi.advance(17)?);
        assert_eq!(18, disi.next_doc()?);
        assert_eq!(19, disi.next_doc()?);
        assert_eq!(NO_MORE_DOCS, disi.next_doc()?);
        Ok(())
    }
}

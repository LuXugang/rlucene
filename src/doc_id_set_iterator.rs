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

/**
 * This abstract class defines methods to iterate over a set of non-decreasing doc ids. Note that
 * this class assumes it iterates on doc Ids, and therefore `NO_MORE_DOCS` is set to `
 * NO_MORE_DOCS` in order to be used as a sentinel object. Implementations of this class are
 * expected to consider `i32::MAX` as an invalid value.
 */

pub trait DocIdSetIterator {
    /**
     * Returns the following:
     *
     * `-1` if `nextDoc()` or `advance` were not called yet.
     * `NO_MORE_DOCS` if the iterator has exhausted.
     * Otherwise, it should return the doc ID it is currently on.
     */
    fn doc_id(&self) -> i32;
    /**
     * Advances to the next document in the set and returns the doc it is currently on, or `NO_MORE_DOCS` if there are no more docs in the set.
     *
     * <b>NOTE:</b>after the iterator has exhausted you should not call this method, as it may result in unpredicted behavior.
     */
    fn next_doc(&mut self) -> i32;
    /**
     * Advances to the first beyond the current whose document number is greater than or equal to
     * <i>target</i>, and returns the document number itself. Exhausts the iterator and returns `NO_MORE_DOCS` if `target` is greater than the highest document number in the set.
     *
     * The behavior of this method is <b>undefined</b> when called with <code> target &le; current</code>, or after the iterator has exhausted. Both cases may result in unpredicted behavior.
     *
     * <b>NOTE:</b> this method may be called with `NO_MORE_DOCS` for efficiency by some
     * Scorers. If your implementation cannot efficiently determine that it should exhaust, it is
     * recommended that you check for that value in each call to this method.
     */
    fn advance(&mut self, target: i32) -> i32;
    /**
     * Slow (linear) implementation of `advance` relying on `nextDoc()` to advance
     * beyond the target position.
     */
    fn slow_advance(&mut self, target: i32) -> i32 {
        assert!(self.doc_id() < target);
        let mut doc;
        loop {
            doc = self.next_doc();
            if doc >= target {
                break;
            }
        }
        doc
    }
    /**
     * Returns the estimated cost of this `DocIdSetIterator`.
     * This is generally an upper bound of the number of documents this iterator might match, but
     * may be a rough heuristic, hardcoded value, or otherwise completely inaccurate.
     */
    fn cost(&self) -> i64;
}

/** An empty `DocIdSetIterator` */
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
        
        if self.exhausted { NO_MORE_DOCS } else { -1 }
    }

    fn next_doc(&mut self) -> i32 {
        assert!(!self.exhausted);
        self.exhausted = true;
        NO_MORE_DOCS
    }

    fn advance(&mut self, target: i32) -> i32 {
        assert!(!self.exhausted);
        assert!(target >= 0);
        self.exhausted = true;
        NO_MORE_DOCS
    }

    fn cost(&self) -> i64 {
        0
    }
}

/** A `DocIdSetIterator` that matches all documents up to `maxDoc - 1`. */
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

    fn next_doc(&mut self) -> i32 {
        self.advance(self.doc + 1)
    }

    fn advance(&mut self, target: i32) -> i32 {
        self.doc = target;
        if self.doc >= self.max_doc {
            self.doc = NO_MORE_DOCS
        }
        self.doc
    }

    fn cost(&self) -> i64 {
        self.max_doc as i64
    }
}

/**
 * A `DocIdSetIterator` that matches a range documents from minDocID (inclusive) to maxDocID
 * (exclusive).
 */
pub struct Range {
    doc: i32,
    min_doc: i32,
    max_doc: i32,
}
impl Range {
    pub fn new(min_doc: i32, max_doc: i32) -> Result<Range, String> {
        if min_doc >= max_doc {
            return Err(format!(
                "minDoc must be < maxDoc but got minDoc= {} maxDoc= {}",
                min_doc, max_doc
            ));
        }
        if min_doc < 0 {
            return Err(format!("minDoc must be >= 0 but got minDoc= {}", min_doc));
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

    fn next_doc(&mut self) -> i32 {
        self.advance(self.doc + 1)
    }

    fn advance(&mut self, target: i32) -> i32 {
        if target < self.min_doc {
            self.doc = self.min_doc;
        } else if target >= self.max_doc {
            self.doc = NO_MORE_DOCS
        } else {
            self.doc = target
        }
        self.doc
    }

    fn cost(&self) -> i64 {
        (self.max_doc - self.min_doc) as i64
    }
}

/**
 * When returned by `nextDoc()`, `advance(i32)` and `docID` it means there
 * are no more docs in the iterator.
 */
pub const NO_MORE_DOCS: i32 = i32::MAX;

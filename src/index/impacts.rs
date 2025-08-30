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
    fn get_doc_id_upto(&self, level: i32) -> i32;

    /// Return impacts on the given level.
    ///
    /// These impacts are sorted by increasing frequency and increasing unsigned
    /// norm, and only valid until the doc ID returned by
    /// `get_doc_id_upto(level)` (inclusive).
    ///
    /// The returned list is never empty and should behave like `RandomAccess`
    /// if it contains more than one element.
    ///
    /// NOTE: There is no guarantee that these impacts actually appear in
    /// postings, only that they trigger scores that are greater than or
    /// equal to the impacts that actually appear in postings.
    fn get_impacts(&'_ mut self, level: i32) -> Result<Cow<'_, [Impact]>>;
}

// Impacts
pub enum Either2Impacts<A, B>
where
    A: Impacts,
    B: Impacts,
{
    A(A),
    B(B),
}

impl<A, B> Impacts for Either2Impacts<A, B>
where
    A: Impacts,
    B: Impacts,
{
    fn num_levels(&self) -> i32 {
        match self {
            Either2Impacts::A(t) => t.num_levels(),
            Either2Impacts::B(s) => s.num_levels(),
        }
    }

    fn get_doc_id_upto(&self, level: i32) -> i32 {
        match self {
            Either2Impacts::A(t) => t.get_doc_id_upto(level),
            Either2Impacts::B(s) => s.get_doc_id_upto(level),
        }
    }

    fn get_impacts(&'_ mut self, level: i32) -> Result<Cow<'_, [Impact]>> {
        match self {
            Either2Impacts::A(t) => t.get_impacts(level),
            Either2Impacts::B(s) => s.get_impacts(level),
        }
    }
}

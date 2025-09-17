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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error::Result;

pub trait TwoPhaseIterator {
    type DocIdSetIterator: DocIdSetIterator;

    /// Return the approximation [`DocIdSetIterator`].
    ///
    /// The returned iterator must advance synchronously with this
    /// `TwoPhaseIterator`.
    fn approximation(&mut self) -> &mut Self::DocIdSetIterator;

    /// Return whether the current doc ID that `approximation()` is on matches.
    ///
    /// This method should only be called when the iterator is positioned
    /// (i.e. not when `doc_id()` is `-1` or `NO_MORE_DOCS`) and at most once.
    ///
    /// # Errors
    /// Returns an error if an I/O error occurs.
    fn matches(&mut self) -> Result<bool>;

    /// An estimate of the expected cost to determine that a single
    /// document matches.
    ///
    /// This can be called before iterating the documents of
    /// `approximation()`. Returns an expected cost in number of simple
    /// operations (add, multiply, compare, array index). Must be positive.
    fn match_cost(&self) -> f32;
}

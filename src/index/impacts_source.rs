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

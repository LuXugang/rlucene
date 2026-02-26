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
use crate::core::index::dummy::dummy_leaf_reader::DummyLeafReader;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::dummy::dummy_matches::DummyMatches;
use crate::core::search::dummy::dummy_scorer_supplier::DummyScorerSupplier;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::Query;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

pub struct DummyWeight<IRC>
where
    IRC: IndexReaderContext,
{
    leaf_reader: IRCLeafReader<IRC>,
}

impl<IRC> DummyWeight<IRC>
where
    IRC: IndexReaderContext,
{
    pub fn new(lr: IRCLeafReader<IRC>) -> Self {
        Self { leaf_reader: lr }
    }
}
impl Default for DummyWeight<LeafReaderContext<DummyLeafReader>> {
    fn default() -> Self {
        Self::new(DummyLeafReader)
    }
}

impl<IRC> SegmentCacheable<IRC> for DummyWeight<IRC>
where
    IRC: IndexReaderContext,
{
    fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl<IRC> Weight<IRC> for DummyWeight<IRC>
where
    IRC: IndexReaderContext,
{
    type Matches = DummyMatches;

    fn matches(
        &self,
        _context: &LeafReaderContext<IRCLeafReader<IRC>>,
        _doc: i32,
        _searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::Matches>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn default_matches(
        &self,
        _context: &LeafReaderContext<IRCLeafReader<IRC>>,
        _doc: i32,
        _searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<MatchWithNoTerms>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn explain(
        &self,
        _context: &LeafReaderContext<IRCLeafReader<IRC>>,
        _doc: i32,
        _searcher: &IndexSearcher<IRC>,
    ) -> Result<Explanation> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_query(&self) -> Arc<Query> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn scorer(
        &self,
        _context: &LeafReaderContext<IRCLeafReader<IRC>>,
        _searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<IRC>>::Scorer>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type ScorerSupplier = DummyScorerSupplier<IRC>;

    fn scorer_supplier(
        &self,
        _context: &LeafReaderContext<IRCLeafReader<IRC>>,
        _searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn bulk_scorer(
        &self,
        _context: &LeafReaderContext<IRCLeafReader<IRC>>,
        _searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<IRC>>::BulkScorer>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn count(&self, _context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn default_count(&self, __context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

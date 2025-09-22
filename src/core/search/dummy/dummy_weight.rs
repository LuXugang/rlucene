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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::dummy::dummy_matches::DummyMatches;
use crate::core::search::dummy::dummy_query::DummyQuery;
use crate::core::search::dummy::dummy_scorer_supplier::DummyScorerSupplier;
use crate::core::search::explanation::Explanation;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;

pub struct DummyWeight;

impl SegmentCacheable for DummyWeight {
    fn is_cacheable<LR>(&self, ctx: &LeafReaderContext<LR>) -> bool
    where
        LR: LeafReader,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl Weight for DummyWeight {
    type Matches = DummyMatches;

    fn matches<LR>(
        &mut self,
        _context: &LeafReaderContext<LR>,
        _doc: i32,
    ) -> Result<Option<Self::Matches>>
    where
        LR: LeafReader,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn default_matches<LR>(
        &mut self,
        _context: &LeafReaderContext<LR>,
        _doc: i32,
    ) -> Result<Option<MatchWithNoTerms>>
    where
        LR: LeafReader,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn explain<LR>(&mut self, _context: &LeafReaderContext<LR>, _doc: i32) -> Result<Explanation>
    where
        LR: LeafReader,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type Query = DummyQuery;

    fn get_query(&self) -> &Self::Query {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn scorer<LR>(
        &mut self,
        _context: &LeafReaderContext<LR>,
    ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier>::Scorer>>
    where
        LR: LeafReader,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type ScorerSupplier = DummyScorerSupplier;

    fn scorer_supplier<LR>(
        &mut self,
        _context: &LeafReaderContext<LR>,
    ) -> Result<Option<Self::ScorerSupplier>>
    where
        LR: LeafReader,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn bulk_scorer<LR>(
        &mut self,
        _context: &LeafReaderContext<LR>,
    ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier>::BulkScorer>>
    where
        LR: LeafReader,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn count<LR>(&self, _context: &LeafReaderContext<LR>) -> Result<i32>
    where
        LR: LeafReader,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn default_count<LR>(&self, _context: &LeafReaderContext<LR>) -> Result<i32>
    where
        LR: LeafReader,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

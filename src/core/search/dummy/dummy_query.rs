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
use crate::core::index::dummy::dummy_index_reader_context::DummyIndexReaderContext;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::search::dummy::dummy_weight::DummyWeight;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::Query;
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::similarities_impl::similarities::Similarity;
use std::fmt::Display;

#[derive(Eq, Hash, PartialEq, Debug)]
pub struct DummyQuery {}
impl Query for DummyQuery {
    fn as_string(&self, _field: &str) -> String {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type Weight<S>
        = DummyWeight
    where
        S: Similarity;
    type IndexReaderContext = DummyIndexReaderContext;

    fn create_weight<S>(
        self,
        _search: &IndexSearcher<Self::IndexReaderContext, S>,
        _score_mod: &ScoreMode,
        _boost: f32,
    ) -> crate::core::util::error::lucene_error::Result<Self::Weight<S>>
    where
        S: Similarity,
        Self: Sized,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type Query = DummyQuery;

    fn rewrite<IRC, S>(
        &self,
        _searcher: &IndexSearcher<IRC, S>,
    ) -> crate::core::util::error::lucene_error::Result<Option<Self::Query>>
    where
        IRC: IndexReaderContext,
        S: Similarity,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
impl Display for DummyQuery {
    fn fmt(&self, _f: &mut std::fmt::Formatter) -> std::fmt::Result {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

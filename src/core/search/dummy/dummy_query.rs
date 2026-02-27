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
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::Result;

#[derive(Debug, Clone)]
pub struct DummyQuery {
    id: Identity,
}
impl DummyQuery {
    pub fn new() -> Self {
        Self {
            id: Identity::new(),
        }
    }
}
impl Default for DummyQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for DummyQuery {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for DummyQuery {}

impl std::hash::Hash for DummyQuery {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        0usize.hash(state);
    }
}

impl HasIdentity for DummyQuery {
    fn identity(&self) -> &Identity {
        &self.id
    }
}
impl QueryBase for DummyQuery {
    fn as_string(&self, _field: &str) -> Result<String> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn create_weight<IRC>(
        self,
        _searcher: &IndexSearcher<IRC>,
        _score_mode: &ScoreMode,
        _boost: f32,
    ) -> crate::core::util::error::lucene_error::Result<QueryWeight<IRC>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
        IRCLeafReader<IRC>: 'static,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn rewrite<IRC>(
        self,
        _searcher: &IndexSearcher<IRC>,
    ) -> crate::core::util::error::lucene_error::Result<Query>
    where
        IRC: IndexReaderContext,
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

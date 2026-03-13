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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{IntoBoxQuery, Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::Result;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

/// Query wrapper that reduces the size of max-score blocks to more easily detect problems with the max-score logic.
#[derive(Clone, Default, Debug)]
pub struct BlockScoreQueryWrapper {
    query: Box<Query>,
    block_length: usize,
    id: Identity,
}
impl BlockScoreQueryWrapper {
    pub(crate) fn new<T>(query: T, block_length: usize) -> Self
    where
        T: IntoBoxQuery,
    {
        Self {
            query: query.into_box_query(),
            block_length,
            id: Identity::new(),
        }
    }
}

impl HasIdentity for BlockScoreQueryWrapper {
    fn identity(&self) -> &Identity {
        &self.id
    }
}
impl PartialEq for BlockScoreQueryWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.query == other.query && self.block_length == other.block_length
    }
}
impl Eq for BlockScoreQueryWrapper {}
impl Hash for BlockScoreQueryWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.query.hash(state);
        self.block_length.hash(state);
    }
}

impl QueryBase for BlockScoreQueryWrapper {
    fn as_string(&self, field: &str) -> Result<String> {
        self.query.as_string(field)
    }

    fn create_weight<IRC>(
        self,
        _searcher: &IndexSearcher<IRC>,
        _score_mode: &ScoreMode,
        _boost: f32,
    ) -> Result<QueryWeight<IRC>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        todo!()
    }

    fn rewrite<IRC>(mut self, searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        let query_id = self.query.identity().clone();
        let rewritten = self.query.rewrite(searcher)?;
        if rewritten.identity() != &query_id {
            return Ok(BlockScoreQueryWrapper::new(rewritten, self.block_length).into());
        }
        self.query = Box::new(rewritten);
        Ok(self.into())
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

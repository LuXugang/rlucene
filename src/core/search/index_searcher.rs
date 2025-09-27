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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::term::Term;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::term_statistics::TermStatistics;
use crate::core::util::error::lucene_error::Result;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) const MAX_CLAUSE_COUNT: i32 = 1024;
pub struct IndexSearcher<IRC, S>
where
    IRC: IndexReaderContext,
    S: Similarity,
{
    reader_context: IRC,
    similarity: Rc<S>,
}

impl<IRC, S> IndexSearcher<IRC, S>
where
    IRC: IndexReaderContext,
    S: Similarity,
{
    pub fn stored_fields(&self) {}

    pub fn get_top_reader_context(&self) -> &IRC {
        &self.reader_context
    }
    pub fn get_similarity(&self) -> Rc<S> {
        self.similarity.clone()
    }
    pub fn collection_statistics(&self, _field: &str) -> CollectionStatistics {
        todo!()
    }
    pub fn term_statistics(
        &self,
        term: Arc<Term>,
        doc_freq: i32,
        total_term_freq: i64,
    ) -> Result<TermStatistics> {
        TermStatistics::new(term, doc_freq as i64, total_term_freq)
    }
}
pub fn get_max_clause_count() -> i32 {
    MAX_CLAUSE_COUNT
}

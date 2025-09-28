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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::term::Term;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::term_statistics::TermStatistics;
use crate::core::util::error::lucene_error::{LuceneError, Result};
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
/// Holds information about a specific leaf context and the corresponding range of doc ids to
/// search within. Used to optionally search across partitions of the same segment concurrently.
///
/// A partition instance can be created via [`LeafReaderContextPartition::create_for_entire_segment`],
/// in which case it will target the entire provided [`LeafReaderContext`].
/// A true partition of a segment can be created via
/// [`LeafReaderContextPartition::create_from_and_to`] providing the minimum doc id (inclusive) to
/// search as well as the max doc id (exclusive).
pub struct LeafReaderContextPartition {
    pub min_doc_id: i32,
    pub max_doc_id: i32,
    pub ctx_ord: usize,
    // we keep track of maxDocs separately because we use NO_MORE_DOCS as upper bound when targeting
    // the entire segment. We use this only in tests.
    max_docs: i32,
}
impl LeafReaderContextPartition {
    pub fn new<LR>(
        leaf_reader_context: &LeafReaderContext<LR>,
        min_doc_id: i32,
        max_doc_id: i32,
        max_docs: i32,
    ) -> Result<Self>
    where
        LR: LeafReader,
    {
        let ctx_max_doc = leaf_reader_context.reader().max_doc()?;
        if min_doc_id >= max_doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "minDocId is greater than or equal to maxDocId: [{}] >= [{}]",
                min_doc_id, max_doc_id
            )));
        }
        if min_doc_id < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "minDocId is lower than 0: [{}]",
                min_doc_id
            )));
        }
        if min_doc_id >= ctx_max_doc {
            return Err(LuceneError::illegal_argument(format!(
                "minDocId is greater than maxDoc: [{}] >= [{}]",
                min_doc_id, ctx_max_doc
            )));
        }

        Ok(Self {
            min_doc_id,
            max_doc_id,
            ctx_ord: leaf_reader_context.ord,
            max_docs,
        })
    }
    /// Creates a partition of the provided leaf context that targets the entire segment
    pub fn create_for_entire_segment<LR>(ctx: &LeafReaderContext<LR>) -> Result<Self>
    where
        LR: LeafReader,
    {
        Self::new(ctx, 0, NO_MORE_DOCS, ctx.reader().max_doc()?)
    }

    /// Creates a partition of the provided leaf context that targets a subset of the entire segment,
    /// starting from and including the min doc id provided, until and not including the provided max doc id
    pub fn create_from_and_to<LR>(
        ctx: &LeafReaderContext<LR>,
        min_doc_id: i32,
        max_doc_id: i32,
    ) -> Result<Self>
    where
        LR: LeafReader,
    {
        debug_assert!(max_doc_id != NO_MORE_DOCS);
        Self::new(ctx, min_doc_id, max_doc_id, max_doc_id - min_doc_id)
    }
}

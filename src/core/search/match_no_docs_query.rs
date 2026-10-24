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
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::QueryBase;
use crate::core::search::query::QueryRef;
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::weight::Weight;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};

/// A query that matches no documents.
#[derive(Clone, Debug)]
pub struct MatchNoDocsQuery {
  id: Identity,
  reason: Cow<'static, str>,
}

impl Default for MatchNoDocsQuery {
  fn default() -> Self {
    Self::new()
  }
}

impl MatchNoDocsQuery {
  /// Creates an instance with default settings.
  pub fn new() -> Self {
    Self {
      id: Identity::new(),
      reason: Cow::Borrowed(""),
    }
  }
  /// Provides a reason explaining why this query was used
  pub fn with_reason<T>(reason: T) -> Self
  where
    T: Into<Cow<'static, str>>,
  {
    let reason = reason.into();
    Self {
      id: Identity::new(),
      reason,
    }
  }
}

impl PartialEq for MatchNoDocsQuery {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

impl Eq for MatchNoDocsQuery {}

impl Hash for MatchNoDocsQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.reason.hash(state);
  }
}

impl HasIdentity for MatchNoDocsQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for MatchNoDocsQuery {
  type Weight<IRC, SIM>
    = MatchNoDocsWeight
  where
    SIM: Similarity,
    IRC: IndexReaderContext + 'static;

  fn to_string(&self, _field: &str) -> Result<String> {
    Ok(format!("MatchNoDocsQuery(\"{}\")", self.reason))
  }

  fn create_weight<IRC, SIM>(
    self,
    _searcher: &IndexSearcher<IRC, SIM>,
    _score_mode: &ScoreMode,
    _boost: f32,
  ) -> Result<Self::Weight<IRC, SIM>>
  where
    SIM: Similarity,
    IRC: IndexReaderContext + 'static,
    Self: Sized,
    IndexSearcher<IRC, SIM>: Sync,
  {
    Ok(MatchNoDocsWeight::new(self))
  }

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    visitor.visit_leaf(self.into())
  }
}

pub struct MatchNoDocsWeight {
  parent_query: MatchNoDocsQuery,
}

impl MatchNoDocsWeight {
  pub fn new(query: MatchNoDocsQuery) -> Self {
    Self {
      parent_query: query,
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for MatchNoDocsWeight
where
  IRC: IndexReaderContext + 'static,
{
  fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    Ok(true)
  }
}

impl<IRC, SIM> Weight<IRC, SIM> for MatchNoDocsWeight
where
  SIM: Similarity,
  IRC: IndexReaderContext + 'static,
{
  fn explain(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _doc: i32,
    _searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Explanation>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    let parent_query = &self.parent_query;
    Ok(Explanation::no_match_no_details(
      parent_query.reason.clone(),
    ))
  }

  fn get_query(&self) -> QueryRef<'_> {
    QueryRef::MatchNoDocs(&self.parent_query)
  }

  type ScorerSupplier = crate::core::search::weight::DefaultScorerSupplier<
    crate::core::search::constant_score_scorer::ConstantScoreDisiScorer<
      crate::core::search::doc_id_set_iterator::EmptyDISI,
    >,
  >;

  fn scorer_supplier(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<Self::ScorerSupplier>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    Ok(None)
  }

  fn count(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<i32>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    Ok(0)
  }
}

impl std::fmt::Debug for MatchNoDocsWeight {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "weight({:?})", QueryRef::MatchNoDocs(&self.parent_query))
  }
}

impl crate::core::util::accountable::Accountable for MatchNoDocsQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}

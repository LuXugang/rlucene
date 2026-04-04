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
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::Weight;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// A query that matches no documents.
#[derive(Clone, Debug)]
pub struct MatchNoDocsQuery {
  id: Identity,
  reason: String,
}

impl Default for MatchNoDocsQuery {
  fn default() -> Self {
    Self::new()
  }
}

impl MatchNoDocsQuery {
  /// Default constructor
  pub fn new() -> Self {
    Self {
      id: Identity::new(),
      reason: "".to_string(),
    }
  }
  /// Provides a reason explaining why this query was used
  pub fn with_reason<T>(reason: T) -> Self
  where
    T: Into<String>,
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
  fn as_string(&self, _field: &str) -> Result<String> {
    Ok(format!("MatchNoDocsQuery(\"{}\")", self.reason))
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
    Ok(Box::new(MatchNoDocsWeight::new(self)))
  }

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Ok(self.into())
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
    todo!()
  }
}

pub struct MatchNoDocsWeight {
  parent_query: Arc<Query>,
}

impl MatchNoDocsWeight {
  pub fn new(query: MatchNoDocsQuery) -> Self {
    Self {
      parent_query: Arc::new(query.into()),
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for MatchNoDocsWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    Ok(true)
  }
}

impl<IRC> Weight<IRC> for MatchNoDocsWeight
where
  IRC: IndexReaderContext,
{
  type Matches = MatchWithNoTerms;

  fn matches(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _doc: i32,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::Matches>> {
    Ok(None)
  }

  fn explain(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _doc: i32,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    let parent_query = if let Query::MatchNoDocs(v) = self.parent_query.as_ref() {
      v
    } else {
      return Err(LuceneError::illegal_state(""));
    };
    Ok(Explanation::no_match_no_details(
      parent_query.reason.clone(),
    ))
  }

  fn get_query(&self) -> Arc<Query> {
    self.parent_query.clone()
  }

  type ScorerSupplier = QueryWeightSs<IRC>;

  fn scorer_supplier(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    Ok(None)
  }

  fn count(&self, _context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
    Ok(0)
  }
}

impl std::fmt::Debug for MatchNoDocsWeight {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "weight({:?})", self.parent_query)
  }
}
#[cfg(test)]
mod tests {
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::document::field_type::FieldType;
  use crate::core::index::directory_reader::directory_reader_util;
  use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
  use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
  use crate::core::index::term::Term;
  use crate::core::search::boolean_clause::Occur;
  use crate::core::search::boolean_query::Builder;
  use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
  use crate::core::search::query::{Query, QueryBase};
  use crate::core::search::term_query::TermQuery;
  use crate::core::store::directory::Directory;
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
  use crate::test::core::search::query_utils::QueryUtils;
  use crate::test::core::util::dummy_index_searcher;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_index_writer_config_with_analyzer, new_log_merge_policy,
    new_searcher_with_reader, new_text_field, random,
  };
  use rand::Rng;
  use std::collections::HashMap;

  #[allow(dead_code)] // for quick search
  struct TestMatchNoDocsQuery;
  fn set_up<R>(random: &mut R) -> MockAnalyzer
  where
    R: Rng + ?Sized,
  {
    MockAnalyzer::new(random)
  }

  #[test]
  fn test_simple() -> Result<()> {
    {
      let mut query = MatchNoDocsQuery::new();
      assert_eq!(query.as_string("")?, "MatchNoDocsQuery(\"\")");

      query = MatchNoDocsQuery::with_reason("field 'title' not found");
      assert_eq!(
        query.as_string("")?,
        "MatchNoDocsQuery(\"field 'title' not found\")"
      );
      let dummy_searcher = dummy_index_searcher()?;
      let rewrite = query.rewrite(&dummy_searcher)?;
      assert!(matches!(rewrite, Query::MatchNoDocs(_)));
      assert_eq!(
        rewrite.as_string("")?,
        "MatchNoDocsQuery(\"field 'title' not found\")"
      );
    }

    Ok(())
  }

  #[test]
  fn test_query() -> Result<()> {
    let mut random = random();

    let dir = new_directory_shared(&mut random)?;
    let analyzer = MockAnalyzer::new(&mut random);

    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    iwc.set_max_buffered_docs(2);
    iwc.set_merge_policy(new_log_merge_policy(&mut random)?);

    let mut iw = IndexWriter::new(dir.clone(), iwc)?;
    let mut field_to_type = HashMap::new();
    add_doc("one", &mut iw, &mut random, &mut field_to_type)?;
    add_doc("two", &mut iw, &mut random, &mut field_to_type)?;
    add_doc("three", &mut iw, &mut random, &mut field_to_type)?;

    let reader = directory_reader_util::open_from_writer(&iw)?;
    let searcher = new_searcher_with_reader(reader)?;

    let mut query: Query = MatchNoDocsQuery::with_reason("field not found").into();
    assert_eq!(searcher.count(query.clone())?, 0);

    let hits = searcher.search(MatchNoDocsQuery::new(), 1000)?.score_docs;
    assert_eq!(hits.len(), 0);
    assert_eq!(
      query.as_string("")?,
      "MatchNoDocsQuery(\"field not found\")"
    );

    let mut bq = Builder::new();
    bq.add(
      TermQuery::new(Term::from_text("key", "five")),
      Occur::Should,
    )?;
    bq.add(
      MatchNoDocsQuery::with_reason("field not found"),
      Occur::Must,
    )?;
    query = bq.build().into();

    assert_eq!(searcher.count(query.clone())?, 0);

    let hits = searcher.search(MatchNoDocsQuery::new(), 1000)?.score_docs;
    assert_eq!(hits.len(), 0);
    assert_eq!(
      query.as_string("")?,
      "key:five +MatchNoDocsQuery(\"field not found\")"
    );

    let mut bq = Builder::new();
    bq.add(TermQuery::new(Term::from_text("key", "one")), Occur::Should)?;
    bq.add(
      MatchNoDocsQuery::with_reason("field not found"),
      Occur::Should,
    )?;
    query = bq.build().into();

    assert_eq!(
      query.as_string("")?,
      "key:one MatchNoDocsQuery(\"field not found\")"
    );
    assert_eq!(searcher.count(query.clone())?, 1);

    let hits = searcher.search(query.clone(), 1000)?.score_docs;
    let rewrite = searcher.rewrite(query.clone())?;

    assert_eq!(hits.len(), 1);
    assert_eq!(rewrite.as_string("")?, "key:one");

    iw.close()?;

    Ok(())
  }

  #[test]
  fn test_equals() -> Result<()> {
    let q1: Query = MatchNoDocsQuery::new().into();
    let q2: Query = MatchNoDocsQuery::new().into();

    assert_eq!(q1, q2);
    QueryUtils::check_from_query(&q1);

    Ok(())
  }

  fn add_doc<R, D, L, B>(
    text: &str,
    iw: &mut IndexWriter<D, L, B>,
    random: &mut R,
    field_to_type: &mut HashMap<String, FieldType>,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    D: Directory,
    L: LiveIndexWriterConfig,
    B: IndexWriterBase,
  {
    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      "key",
      text,
      Store::Yes,
      field_to_type,
    )?);

    iw.add_document(doc)?;

    Ok(())
  }
}

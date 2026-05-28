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
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::string_field::StringField;
use crate::core::index::composite_reader::get_context;
use crate::core::index::field_invert_state::FieldInvertState;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_reader::MultiReader;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::term::Term;
use crate::core::index::term_states::build;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::similarities_impl::similarities::{
  BoxSimScorer, SimScorer, Similarity, SimilarityEnum,
};
use crate::core::search::term_query::TermQuery;
use crate::core::search::term_statistics::TermStatistics;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::query_utils::QueryUtils;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config, new_searcher_with_reader, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[allow(dead_code)] // for quick search
struct TestTermQuery;

#[test]
fn test_equals() -> Result<()> {
  QueryUtils::check_equal::<Query>(
    &TermQuery::new(Term::from_text("foo", "bar")).into(),
    &TermQuery::new(Term::from_text("foo", "bar")).into(),
  );

  QueryUtils::check_unequal::<Query>(
    &TermQuery::new(Term::from_text("foo", "bar")).into(),
    &TermQuery::new(Term::from_text("foo", "baz")).into(),
  );

  let multi_reader = MultiReader::empty()?;
  let context = get_context(multi_reader)?;
  let searcher = IndexSearcher::new(context)?;

  QueryUtils::check_equal::<Query>(
    &TermQuery::new(Term::from_text("foo", "bar")).into(),
    &TermQuery::with_term_state(
      Term::from_text("foo", "bar"),
      Some(build(&searcher, Term::from_text("foo", "bar"), true)?),
    )
    .into(),
  );

  Ok(())
}
#[test]
fn test_create_weight_does_not_seek_if_scores_are_not_needed() -> Result<()> {
  // FilterDirectoryReader未实现
  Ok(())
}
#[test]
fn test_query_matches_count() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone());

  let random_num_docs = TestUtil::next_int(&mut random, 10, 100);
  let mut num_matching_docs = 0;

  for _ in 0..random_num_docs {
    let mut doc = Document::new();
    if random.random_bool(0.5) {
      doc.add(StringField::from_string("foo", "bar", Store::No)?);
      num_matching_docs += 1;
    }
    writer.add_document(doc)?;
  }

  writer.force_merge(1)?;

  let reader = writer.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;

  let test_query: Query = TermQuery::new(Term::from_text("foo", "bar")).into();
  assert_eq!(num_matching_docs, searcher.count(test_query.clone())?);

  let weight = searcher.create_weight(test_query, ScoreMode::Complete, 1.0)?;
  let leaves = searcher.reader_context.leaves()?;
  assert_eq!(num_matching_docs, weight.count(&leaves[0])?);

  writer.close()?;
  Ok(())
}
#[test]
fn test_get_term_states() -> Result<()> {
  assert!(
    TermQuery::new(Term::from_text("foo", "bar"))
      .get_term_states()
      .is_none()
  );

  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_merge_policy(NoMergePolicy::default());

  let writer = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let mut doc = Document::new();
  doc.add(StringField::from_string("foo", "bar", Store::No)?);
  writer.add_document(doc)?;
  writer.get_reader()?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("foo", "baz", Store::No)?);
  writer.add_document(doc)?;
  writer.get_reader()?;

  writer.add_document(Document::new())?;

  let reader = writer.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;

  let query_with_context = TermQuery::with_term_state(
    Term::from_text("foo", "bar"),
    Some(build(&searcher, Term::from_text("foo", "bar"), true)?),
  );
  assert!(query_with_context.get_term_states().is_some());

  writer.close()?;
  Ok(())
}

#[test]
fn test_with_different_score_modes() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_merge_policy(NoMergePolicy::default());
  let writer = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let mut doc = Document::new();
  doc.add(StringField::from_string("foo", "bar", Store::No)?);
  writer.add_document(doc)?;
  writer.get_reader()?;

  let reader = writer.get_reader()?;
  let mut searcher = new_searcher_with_reader(reader)?;
  let existing_similarity = searcher.get_similarity().clone();

  for score_mode in ScoreMode::values() {
    let scorer_called = Arc::new(AtomicBool::new(false));
    let s = SimilarityEnum::custom(SimilarityImpl::new(
      existing_similarity.clone(),
      scorer_called.clone(),
    ));
    searcher.set_similarity(s);
    let term_query = TermQuery::new(Term::from_text("foo", "bar"));
    term_query.create_weight(&searcher, score_mode, 1f32)?;
    assert_eq!(
      score_mode.needs_scores(),
      scorer_called.load(Ordering::Relaxed)
    );
  }

  writer.close()?;
  Ok(())
}

pub struct SimilarityImpl<S> {
  existing_similarity: S,
  scorer_called: Arc<AtomicBool>,
}
impl<S> SimilarityImpl<S>
where
  S: Similarity,
{
  fn new(existing_similarity: S, scorer_called: Arc<AtomicBool>) -> Self {
    Self {
      existing_similarity,
      scorer_called,
    }
  }
}

impl<S> Display for SimilarityImpl<S>
where
  S: Similarity,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "SimilarityImpl")
  }
}

impl<S> Similarity for SimilarityImpl<S>
where
  S: Similarity,
  S::SimScorer: SimScorer + Send + Sync + 'static,
{
  fn compute_norm(&self, state: &FieldInvertState) -> Result<i64> {
    self.existing_similarity.compute_norm(state)
  }

  type SimScorer = BoxSimScorer;

  fn scorer(
    &self,
    boost: f32,
    collection_stats: &CollectionStatistics,
    term_stats: &[TermStatistics],
  ) -> Result<Self::SimScorer> {
    self
      .scorer_called
      .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(Box::new(self.existing_similarity.scorer(
      boost,
      collection_stats,
      term_stats,
    )?))
  }
}

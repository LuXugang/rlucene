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
use crate::core::index::field_invert_state::FieldInvertState;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::multi_doc_values::MultiDocValues;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::term::Term;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::similarities_impl::per_field_similarity_wrapper::PerFieldSimilarityWrapper;
use crate::core::search::similarities_impl::similarities::{
  BoxSimScorer, SimScorer, Similarity, SimilarityEnum,
};
use crate::core::search::term_query::TermQuery;
use crate::core::search::term_statistics::TermStatistics;
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
use crate::test::support::core::util::DefaultIndexSearchCR;
use crate::test::support::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_searcher_with_reader,
  new_text_field, random,
};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

#[allow(dead_code)] // for quick search
pub struct TestSimilarityProvider;

fn set_up() -> Result<DefaultIndexSearchCR> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;
  let sim = SimilarityEnum::custom(ExampleSimilarityProvider::new());
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwc.set_similarity(sim);

  let iw = RandomIndexWriter::with_config(&mut random, directory.clone(), iwc);

  let mut doc = Document::new();
  let mut field_to_type = HashMap::new();
  let mut field = new_text_field(
    &mut random,
    "foo",
    "quick brown fox",
    Store::No,
    &mut field_to_type,
  )?;
  let mut field2 = new_text_field(
    &mut random,
    "bar",
    "quick brown fox",
    Store::No,
    &mut field_to_type,
  )?;
  doc.add(field.clone());
  doc.add(field2.clone());
  iw.add_document(&mut random, doc)?;

  doc = Document::new();
  field.set_string_value("jumps over lazy brown dog")?;
  field2.set_string_value("jumps over lazy brown dog")?;
  doc.add(field);
  doc.add(field2);
  iw.add_document(&mut random, doc)?;

  let reader = iw.get_reader(&mut random)?;
  iw.close(&mut random)?;

  let mut searcher = new_searcher_with_reader(reader)?;
  searcher.set_similarity(SimilarityEnum::custom(ExampleSimilarityProvider::new()));

  Ok(searcher)
}
#[test]
fn test_basics() -> Result<()> {
  let searcher = set_up()?;
  let reader = searcher.get_index_reader();

  let mut foo_norms = MultiDocValues::get_norm_values(reader, "foo")?.unwrap();
  let mut bar_norms = MultiDocValues::get_norm_values(reader, "bar")?.unwrap();

  for i in 0..reader.max_doc()? {
    assert_eq!(i, foo_norms.next_doc()?);
    assert_eq!(i, bar_norms.next_doc()?);
    assert_ne!(foo_norms.long_value()?, bar_norms.long_value()?);
  }

  // sanity check of searching
  let foodocs = searcher.search(TermQuery::new(Term::from_text("foo", "brown")), 10)?;
  assert!(foodocs.total_hits.value() > 0);

  let bardocs = searcher.search(TermQuery::new(Term::from_text("bar", "brown")), 10)?;
  assert!(bardocs.total_hits.value() > 0);

  assert!(foodocs.score_docs[0].score < bardocs.score_docs[0].score);
  Ok(())
}

struct ExampleSimilarityProvider {
  sim1: Sim1,
  sim2: Sim2,
}
impl ExampleSimilarityProvider {
  fn new() -> Self {
    Self {
      sim1: Sim1,
      sim2: Sim2,
    }
  }
}
impl Display for ExampleSimilarityProvider {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl Similarity for ExampleSimilarityProvider {
  fn compute_norm(&self, state: &FieldInvertState) -> Result<i64> {
    PerFieldSimilarityWrapper::compute_norm(self, state)
  }

  type SimScorer = BoxSimScorer;

  fn scorer(
    &self,
    boost: f32,
    collection_stats: &CollectionStatistics,
    term_stats: &[TermStatistics],
  ) -> Result<Self::SimScorer> {
    PerFieldSimilarityWrapper::scorer(self, boost, collection_stats, term_stats)
  }
}

impl PerFieldSimilarityWrapper for ExampleSimilarityProvider {
  type Similarity = SimilarityTestEnum;

  fn get(&self, field: &str) -> Self::Similarity {
    if field == "foo" {
      SimilarityTestEnum::Sim1(self.sim1.clone())
    } else {
      SimilarityTestEnum::Sim2(self.sim2.clone())
    }
  }
}
enum SimilarityTestEnum {
  Sim1(Sim1),
  Sim2(Sim2),
}

impl Display for SimilarityTestEnum {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      SimilarityTestEnum::Sim1(v) => write!(f, "{}", v),
      SimilarityTestEnum::Sim2(v) => write!(f, "{}", v),
    }
  }
}

impl Similarity for SimilarityTestEnum {
  fn get_discount_overlaps(&self) -> bool {
    match self {
      SimilarityTestEnum::Sim1(_) => true,
      SimilarityTestEnum::Sim2(_) => false,
    }
  }

  fn compute_norm(&self, state: &FieldInvertState) -> Result<i64> {
    match self {
      SimilarityTestEnum::Sim1(v) => v.compute_norm(state),
      SimilarityTestEnum::Sim2(v) => v.compute_norm(state),
    }
  }

  type SimScorer = BoxSimScorer;

  fn scorer(
    &self,
    boost: f32,
    collection_stats: &CollectionStatistics,
    term_stats: &[TermStatistics],
  ) -> Result<Self::SimScorer> {
    match self {
      SimilarityTestEnum::Sim1(v) => Ok(Box::new(v.scorer(boost, collection_stats, term_stats)?)),
      SimilarityTestEnum::Sim2(v) => Ok(Box::new(v.scorer(boost, collection_stats, term_stats)?)),
    }
  }
}

#[derive(Default, Clone)]
struct Sim1;

impl Display for Sim1 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl Similarity for Sim1 {
  fn compute_norm(&self, _state: &FieldInvertState) -> Result<i64> {
    Ok(1i64)
  }

  type SimScorer = SimScorerImpl;

  fn scorer(
    &self,
    _boost: f32,
    _collection_stats: &CollectionStatistics,
    _term_stats: &[TermStatistics],
  ) -> Result<Self::SimScorer> {
    Ok(SimScorerImpl)
  }
}
#[derive(Default)]
struct SimScorerImpl;
impl SimScorer for SimScorerImpl {
  fn score(&self, _freq: f32, _norm: i64) -> f32 {
    1f32
  }
}
#[derive(Default, Clone)]
struct Sim2;

impl Display for Sim2 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl Similarity for Sim2 {
  fn compute_norm(&self, _state: &FieldInvertState) -> Result<i64> {
    Ok(10i64)
  }

  type SimScorer = SimScorerImpl2;

  fn scorer(
    &self,
    _boost: f32,
    _collection_stats: &CollectionStatistics,
    _term_stats: &[TermStatistics],
  ) -> Result<Self::SimScorer> {
    Ok(SimScorerImpl2)
  }
}

#[derive(Default)]
struct SimScorerImpl2;

impl SimScorer for SimScorerImpl2 {
  fn score(&self, _freq: f32, _norm: i64) -> f32 {
    10f32
  }
}

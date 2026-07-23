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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::{BooleanQuery, Builder};
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::Query;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::{TopDocs, TopDocsLike};
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::boolean_query::{Callback, rand_bool_query};
use crate::test_framework::core::search::query_utils::QueryUtils;
use crate::test_framework::core::util::DefaultIndexSearchCR;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_searcher_with_reader, new_string_field, new_text_field,
  random, random_from_seed,
};
use crate::test_framework::ulp_f32;
use rand::{Rng, RngExt};
use std::collections::HashMap;
use std::sync::LazyLock;

#[allow(dead_code)] // for quick search
pub struct TestBooleanMinShouldMatch;
static CONTEXT: LazyLock<DefaultIndexSearchCR> = LazyLock::new(|| {
  let mut random = random();
  set_up(&mut random).expect("failed to initialize TestBooleanMinShouldMatch")
});
fn set_up<R>(random: &mut R) -> Result<DefaultIndexSearchCR>
where
  R: Rng + ?Sized,
{
  let data = vec![
    Some("A 1 2 3 4 5 6"),
    Some("Z       4 5 6"),
    None,
    Some("B   2   4 5 6"),
    Some("Y     3   5 6"),
    None,
    Some("C     3     6"),
    Some("X       4 5 6"),
  ];

  let index = new_directory_shared(random)?;
  let writer = RandomIndexWriter::new(random, index.clone())?;

  let mut field_types = HashMap::new();

  for (i, value) in data.into_iter().enumerate() {
    let mut doc = Document::new();
    doc.add(new_string_field(
      random,
      "id",
      i.to_string(),
      Store::Yes,
      &mut field_types,
    )?);
    doc.add(new_string_field(
      random,
      "all",
      "all",
      Store::Yes,
      &mut field_types,
    )?);
    if let Some(text) = value {
      doc.add(new_text_field(
        random,
        "data",
        text,
        Store::Yes,
        &mut field_types,
      )?);
    }
    writer.add_document(random, doc)?;
  }

  let reader = writer.get_reader(random)?;
  let searcher = new_searcher_with_reader(reader)?;
  writer.close(random)?;
  Ok(searcher)
}
fn verify_nr_hits<IRC, R, T>(
  random: &mut R,
  s: &IndexSearcher<IRC>,
  q: T,
  expected: usize,
) -> Result<()>
where
  IRC: IndexReaderContext + Sync,
  R: Rng + ?Sized,
  T: Into<Query>,
  <IRC as IndexReaderContext>::LeafReader: Clone,
{
  let q = q.into();
  let h = s.search(q.clone(), 1000)?.score_docs;
  if expected != h.len() {
    print_hits(&h, s)?;
  }
  assert_eq!(expected, h.len(), "result count");

  let collector_manager = TopScoreDocCollectorManager::new(1000, i32::MAX as usize)?;
  let h2 = s
    .search_with_collector_manager(q.clone(), &collector_manager)?
    .score_docs;
  if expected != h2.len() {
    print_hits(&h2, s)?;
  }
  assert_eq!(expected, h2.len(), "result count (bs2)");
  QueryUtils::check_from_searcher(random, q, s)
}
#[test]
fn test_all_optional() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let mut q = Builder::new();
  for i in 1..=4 {
    q.add(
      TermQuery::new(Term::from_text("data", i.to_string())),
      Occur::Should,
    )?;
  }
  q.set_minimum_number_should_match(2);
  verify_nr_hits(&mut random, s, q.build(), 2)?;

  Ok(())
}
#[test]
fn test_one_req_and_some_optional() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let mut q = Builder::new();
  q.add(TermQuery::new(Term::from_text("all", "all")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "5")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "4")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "3")), Occur::Should)?;

  q.set_minimum_number_should_match(2);

  verify_nr_hits(&mut random, s, q.build(), 5)?;

  Ok(())
}
#[test]
fn test_some_req_and_some_optional() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let mut q = Builder::new();
  q.add(TermQuery::new(Term::from_text("all", "all")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "6")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "5")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "4")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "3")), Occur::Should)?;

  q.set_minimum_number_should_match(2);

  verify_nr_hits(&mut random, s, q.build(), 5)?;

  Ok(())
}

#[test]
fn test_one_prohib_and_some_optional() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let mut q = Builder::new();
  q.add(TermQuery::new(Term::from_text("data", "1")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "2")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "3")), Occur::MustNot)?;
  q.add(TermQuery::new(Term::from_text("data", "4")), Occur::Should)?;

  q.set_minimum_number_should_match(2);

  verify_nr_hits(&mut random, s, q.build(), 1)?;

  Ok(())
}

#[test]
fn test_some_prohib_and_some_optional() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let mut q = Builder::new();
  q.add(TermQuery::new(Term::from_text("data", "1")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "2")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "3")), Occur::MustNot)?;
  q.add(TermQuery::new(Term::from_text("data", "4")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "C")), Occur::MustNot)?;

  q.set_minimum_number_should_match(2);

  verify_nr_hits(&mut random, s, q.build(), 1)?;

  Ok(())
}

#[test]
fn test_one_req_one_prohib_and_some_optional() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let mut q = Builder::new();
  q.add(TermQuery::new(Term::from_text("data", "6")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "5")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "4")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "3")), Occur::MustNot)?;
  q.add(TermQuery::new(Term::from_text("data", "2")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "1")), Occur::Should)?;

  q.set_minimum_number_should_match(3);

  verify_nr_hits(&mut random, s, q.build(), 1)?;

  Ok(())
}

#[test]
fn test_some_req_one_prohib_and_some_optional() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let mut q = Builder::new();
  q.add(TermQuery::new(Term::from_text("all", "all")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "6")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "5")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "4")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "3")), Occur::MustNot)?;
  q.add(TermQuery::new(Term::from_text("data", "2")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "1")), Occur::Should)?;

  q.set_minimum_number_should_match(3);

  verify_nr_hits(&mut random, s, q.build(), 1)?;

  Ok(())
}

#[test]
fn test_one_req_some_prohib_and_some_optional() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let mut q = Builder::new();
  q.add(TermQuery::new(Term::from_text("data", "6")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "5")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "4")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "3")), Occur::MustNot)?;
  q.add(TermQuery::new(Term::from_text("data", "2")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "1")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "C")), Occur::MustNot)?;

  q.set_minimum_number_should_match(3);

  verify_nr_hits(&mut random, s, q.build(), 1)?;

  Ok(())
}
#[test]
fn test_some_req_some_prohib_and_some_optional() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let mut q = Builder::new();
  q.add(TermQuery::new(Term::from_text("all", "all")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "6")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "5")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "4")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "3")), Occur::MustNot)?;
  q.add(TermQuery::new(Term::from_text("data", "2")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "1")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "C")), Occur::MustNot)?;

  q.set_minimum_number_should_match(3);

  verify_nr_hits(&mut random, s, q.build(), 1)?;

  Ok(())
}

#[test]
fn test_min_higher_then_num_optional() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let mut q = Builder::new();
  q.add(TermQuery::new(Term::from_text("all", "all")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "6")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "5")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "4")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "3")), Occur::MustNot)?;
  q.add(TermQuery::new(Term::from_text("data", "2")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "1")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "C")), Occur::MustNot)?;

  q.set_minimum_number_should_match(90);

  verify_nr_hits(&mut random, s, q.build(), 0)?;

  Ok(())
}

#[test]
fn test_min_equal_to_num_optional() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let mut q = Builder::new();
  q.add(TermQuery::new(Term::from_text("all", "all")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "6")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "3")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "2")), Occur::Should)?;

  q.set_minimum_number_should_match(2);

  verify_nr_hits(&mut random, s, q.build(), 1)?;

  Ok(())
}

#[test]
fn test_one_optional_equal_to_min() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let mut q = Builder::new();
  q.add(TermQuery::new(Term::from_text("all", "all")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "3")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text("data", "2")), Occur::Must)?;

  q.set_minimum_number_should_match(1);

  verify_nr_hits(&mut random, s, q.build(), 1)?;

  Ok(())
}

#[test]
fn test_no_optional_but_min() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let mut q = Builder::new();
  q.add(TermQuery::new(Term::from_text("all", "all")), Occur::Must)?;
  q.add(TermQuery::new(Term::from_text("data", "2")), Occur::Must)?;

  q.set_minimum_number_should_match(1);

  verify_nr_hits(&mut random, s, q.build(), 0)?;

  Ok(())
}

#[test]
fn test_no_optional_but_min_2() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let mut q = Builder::new();
  q.add(TermQuery::new(Term::from_text("all", "all")), Occur::Must)?;

  q.set_minimum_number_should_match(1);

  verify_nr_hits(&mut random, s, q.build(), 0)?;

  Ok(())
}

#[test]
fn test_random_queries() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;

  let field = "data".to_string();
  let vals = vec![
    "1".to_string(),
    "2".to_string(),
    "3".to_string(),
    "4".to_string(),
    "5".to_string(),
    "6".to_string(),
    "A".to_string(),
    "Z".to_string(),
    "B".to_string(),
    "Y".to_string(),
    "Z".to_string(),
    "X".to_string(),
    "foo".to_string(),
  ];
  let max_lev = 4;

  let min_nr_cb = CallbackImpl::new(field.clone(), &vals);

  let num = at_least(&mut random, 20);
  for i in 0..num {
    let lev = random.random_range(0..max_lev);
    let seed: u64 = random.random();

    let mut q1_random = random_from_seed(seed);
    let q1 = rand_bool_query(
      &mut q1_random,
      true,
      lev,
      &field,
      &vals,
      None::<&CallbackImpl>,
    )?;

    let mut q2_random = random_from_seed(seed);
    let mut q2 = rand_bool_query(
      &mut q2_random,
      true,
      lev,
      &field,
      &vals,
      None::<&CallbackImpl>,
    )?;

    min_nr_cb.post_create(&mut random, &mut q2)?;

    let q1 = q1.build();
    let q2 = q2.build();

    let top1 = s.search(q1.clone(), 100)?;
    let top2 = s.search(q2.clone(), 100)?;
    if i < 100 {
      QueryUtils::check_from_searcher(&mut random, q1.clone(), s)?;
      QueryUtils::check_from_searcher(&mut random, q2.clone(), s)?;
    }
    assert_subset_of_same_scores(&q2, top1, top2)?;
  }
  Ok(())
}
fn assert_subset_of_same_scores(
  query: &BooleanQuery,
  top1: TopDocs<ScoreDoc>,
  top2: TopDocs<ScoreDoc>,
) -> Result<()> {
  assert!(top2.total_hits().value() <= top1.total_hits().value());
  let num_scoring_clauses =
    query.get_clauses_idx(Occur::Should).len() + query.get_clauses_idx(Occur::Must).len();

  for hit in 0..top2.total_hits().value() {
    let id = top2.score_docs[hit].doc;
    let score = top2.score_docs[hit].score;
    let mut found = false;

    for other in 0..top1.total_hits().value() {
      if top1.score_docs[other].doc == id {
        found = true;
        let other_score = top1.score_docs[other].score;

        // BooleanQuery sums scores into doubles where possible, but rewriting duplicate clauses
        // into a boosted clause and ReqOptSumScorer both introduce intermediate float rounding.
        // Allow losing one ulp of accuracy per scoring clause, as in Lucene's #14715 fix.
        let tolerance = ulp_f32(score) * num_scoring_clauses as f32;
        assert!(
          (score - other_score).abs() <= tolerance,
          "doc {id} scores don't match for query {query:?}: score={score}, other_score={other_score}, diff={}, tolerance={tolerance}",
          (score - other_score).abs(),
        );
      }
    }

    assert!(found);
  }

  Ok(())
}

#[test]
fn test_rewrite_msm1() -> Result<()> {
  let s = &*CONTEXT;
  let mut q1 = Builder::new();
  q1.add(TermQuery::new(Term::from_text("data", "1")), Occur::Should)?;

  let mut q2 = Builder::new();
  q2.add(TermQuery::new(Term::from_text("data", "1")), Occur::Should)?;
  q2.set_minimum_number_should_match(1);

  let q1 = q1.build();
  let q2 = q2.build();
  let top1 = s.search(q1, 100)?;
  let top2 = s.search(q2.clone(), 100)?;
  assert_subset_of_same_scores(&q2, top1, top2)?;
  Ok(())
}

#[test]
fn test_rewrite_negate() -> Result<()> {
  let s = &*CONTEXT;
  let mut q1 = Builder::new();
  q1.add(TermQuery::new(Term::from_text("data", "1")), Occur::Should)?;

  let mut q2 = Builder::new();
  q2.add(TermQuery::new(Term::from_text("data", "1")), Occur::Should)?;
  q2.add(TermQuery::new(Term::from_text("data", "Z")), Occur::MustNot)?;

  let q1 = q1.build();
  let q2 = q2.build();

  let top1 = s.search(q1.clone(), 100)?;
  let top2 = s.search(q2.clone(), 100)?;
  assert_subset_of_same_scores(&q2, top1, top2)?;
  Ok(())
}

#[test]
fn test_flatten_inner_disjunctions() -> Result<()> {
  let mut random = random();
  let s = &*CONTEXT;
  let mut builder = Builder::new();
  builder.set_minimum_number_should_match(2);
  builder.add(TermQuery::new(Term::from_text("all", "all")), Occur::Should)?;
  builder.add(TermQuery::new(Term::from_text("data", "1")), Occur::Should)?;
  builder.add(TermQuery::new(Term::from_text("data", "2")), Occur::Must)?;
  let q: Query = builder.build().into();
  verify_nr_hits(&mut random, s, q, 1)?;

  let mut inner_builder = Builder::new();
  inner_builder.add(TermQuery::new(Term::from_text("all", "all")), Occur::Should)?;
  inner_builder.add(TermQuery::new(Term::from_text("data", "1")), Occur::Should)?;
  let inner: Query = inner_builder.build().into();

  let mut builder = Builder::new();
  builder.set_minimum_number_should_match(2);
  builder.add(inner, Occur::Should)?;
  builder.add(TermQuery::new(Term::from_text("data", "2")), Occur::Must)?;
  let q: Query = builder.build().into();

  verify_nr_hits(&mut random, s, q, 0)?;
  Ok(())
}
fn print_hits<IRC>(_hits: &[ScoreDoc], _searcher: &IndexSearcher<IRC>) -> Result<()>
where
  IRC: IndexReaderContext,
{
  // not required in Rust Lucene
  Ok(())
}
pub(crate) struct CallbackImpl<'a> {
  field: String,
  vals: &'a [String],
}
impl<'a> CallbackImpl<'a> {
  fn new(field: String, vals: &'a [String]) -> Self {
    Self { field, vals }
  }
}
impl Callback for CallbackImpl<'_> {
  fn post_create<R>(&self, random: &mut R, q: &mut Builder) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut opt = 0;
    for clause in &q.clauses {
      if *clause.occur() == Occur::Should {
        opt += 1;
      }
    }

    q.set_minimum_number_should_match(random.random_range(0..=(opt + 1)));

    if random.random_bool(0.5) {
      let random_term = Term::from_text(
        &self.field,
        &self.vals[random.random_range(0..self.vals.len())],
      );
      q.add(TermQuery::new(random_term), Occur::MustNot)?;
    }

    Ok(())
  }
}

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
use crate::core::index::directory_reader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::query::Query;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::search::asserting_query::AssertingQuery;
use crate::test_framework::core::search::block_score_query_wrapper::BlockScoreQueryWrapper;
use crate::test_framework::core::search::check_hits::CheckHits;
use crate::test_framework::core::search::random_approximation_query::RandomApproximationQuery;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_index_writer_config, new_searcher_with_threads, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};

#[allow(dead_code)] // for quick search
pub struct TestBlockMaxConjunction;

fn maybe_wrap<R>(random: &mut R, mut query: Query) -> Result<Query>
where
  R: Rng + ?Sized,
{
  if random.random_bool(0.5) {
    query = BlockScoreQueryWrapper::new(query, TestUtil::next_usize(random, 2, 8)).into();
    query = AssertingQuery::new(random, query).into()
  }
  Ok(query)
}
fn maybe_wrap_two_phase<R>(random: &mut R, mut query: Query) -> Result<Query>
where
  R: Rng + ?Sized,
{
  if random.random_bool(0.5) {
    query = RandomApproximationQuery::new(query, random).into();
    query = AssertingQuery::new(random, query).into()
  }
  Ok(query)
}
#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;
  let num_docs = at_least(&mut random, 1000);

  for _ in 0..num_docs {
    let mut doc = Document::new();
    let upper = 1 << random.random_range(0..5);
    let num_values = random.random_range(0..upper);
    let start = random.random_range(0..10);
    for j in 0..num_values {
      doc.add(StringField::from_string(
        "foo",
        (start + j).to_string(),
        Store::No,
      )?);
    }
    w.add_document(doc)?;
  }

  let reader = directory_reader::open_from_writer(&w)?;
  w.close()?;

  let may_be_wrap = random.random_bool(0.5);
  let wrap_with_assertions = random.random_bool(0.5);
  let searcher = new_searcher_with_threads(
    &mut random,
    reader,
    may_be_wrap,
    wrap_with_assertions,
    false,
  )?;

  for _ in 0..100 {
    let start = random.random_range(0..10);
    let upper = 1 << random.random_range(0..5);
    let num_clauses = random.random_range(0..upper);

    let mut builder = Builder::new();
    for i in 0..num_clauses {
      builder.add(
        maybe_wrap(
          &mut random,
          TermQuery::new(Term::from_text("foo", (start + i).to_string())).into(),
        )?,
        Occur::Must,
      )?;
    }
    let query: Query = builder.build().into();

    CheckHits::check_top_scores(&mut random, &query, &searcher)?;

    let filter_term = random.random_range(0..30);
    let mut filtered_builder = Builder::new();
    filtered_builder.add(query.clone(), Occur::Must)?;
    filtered_builder.add(
      TermQuery::new(Term::from_text("foo", filter_term.to_string())),
      Occur::Filter,
    )?;
    let filtered_query: Query = filtered_builder.build().into();

    CheckHits::check_top_scores(&mut random, &filtered_query, &searcher)?;

    builder = Builder::new();
    for i in 0..num_clauses {
      builder.add(
        maybe_wrap_two_phase(
          &mut random,
          TermQuery::new(Term::from_text("foo", (start + i).to_string())).into(),
        )?,
        Occur::Must,
      )?;
    }

    let _two_phase_inner: Query = builder.build().into();

    let mut two_phase_builder = Builder::new();
    two_phase_builder.add(query, Occur::Must)?;
    two_phase_builder.add(
      TermQuery::new(Term::from_text("foo", filter_term.to_string())),
      Occur::Filter,
    )?;
    let two_phase_query: Query = two_phase_builder.build().into();

    CheckHits::check_top_scores(&mut random, &two_phase_query, &searcher)?;
  }

  Ok(())
}

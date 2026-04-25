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
use std::collections::HashMap;
use std::sync::Arc;

use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::field_type::FieldType;
use crate::core::document::text_field::text_field_type::TYPE_NOT_STORED;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::similarities_impl::bm25_similarity::BM25Similarity;
use crate::core::search::similarities_impl::classic_similarity::ClassicSimilarity;
use crate::core::search::similarities_impl::similarities::SimilarityEnum;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_field, new_index_writer_config, new_log_merge_policy,
  new_searcher_with_reader, new_text_field, random,
};
use rand::RngExt;
#[allow(dead_code)] // for quick search
pub struct TestSimilarity2;

fn set_up() -> Result<Vec<SimilarityEnum>> {
  Ok(vec![
    ClassicSimilarity::new().into(),
    BM25Similarity::new()?.into(),
  ])
}

fn sims() -> Result<Vec<Arc<SimilarityEnum>>> {
  Ok(set_up()?.into_iter().map(Arc::new).collect())
}

/**
 * because of stupid things like querynorm, it's possible we computeStats on a field that doesnt
 * exist at all test this against a totally empty index, to make sure sims handle it
 */
#[test]
fn test_empty_index() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone());
  let ir = iw.get_reader()?;
  iw.close()?;
  let mut searcher = new_searcher_with_reader(ir)?;

  for sim in sims()? {
    searcher.set_similarity(sim);
    assert_eq!(
      0,
      searcher
        .search(TermQuery::new(Term::from_text("foo", "bar")), 10)?
        .total_hits
        .value()
    );
  }
  Ok(())
}

/** similar to the above, but ORs the query with a real field */
#[test]
fn test_empty_field() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone());
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "foo",
    "bar",
    Store::No,
    &mut field_to_type,
  )?);
  iw.add_document(doc)?;
  let ir = iw.get_reader()?;
  iw.close()?;
  let mut searcher = new_searcher_with_reader(ir)?;

  for sim in sims()? {
    searcher.set_similarity(sim);
    let mut query = Builder::new();
    query.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
    query.add(TermQuery::new(Term::from_text("bar", "baz")), Occur::Should)?;
    assert_eq!(1, searcher.search(query.build(), 10)?.total_hits.value());
  }
  Ok(())
}

/**
 * similar to the above, however the field exists, but we query with a term that doesnt exist too
 */
#[test]
fn test_empty_term() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone());
  let mut doc = Document::new();
  let mut field_to_type = HashMap::new();
  doc.add(new_text_field(
    &mut random,
    "foo",
    "bar",
    Store::No,
    &mut field_to_type,
  )?);
  iw.add_document(doc)?;
  let ir = iw.get_reader()?;
  iw.close()?;
  let mut searcher = new_searcher_with_reader(ir)?;

  for sim in sims()? {
    searcher.set_similarity(sim);
    let mut query = Builder::new();
    query.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
    query.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
    assert_eq!(1, searcher.search(query.build(), 10)?.total_hits.value());
  }
  Ok(())
}

/** make sure we can retrieve when norms are disabled */
#[test]
fn test_no_norms() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone());
  let mut doc = Document::new();
  let mut field_to_type = HashMap::new();
  let mut ft = FieldType::from_ref(&*TYPE_NOT_STORED)?;
  ft.set_omit_norms(true)?;
  ft.freeze();
  doc.add(new_field(
    &mut random,
    "foo",
    "bar",
    &ft,
    &mut field_to_type,
  )?);
  iw.add_document(doc)?;
  let ir = iw.get_reader()?;
  iw.close()?;
  let mut searcher = new_searcher_with_reader(ir)?;

  for sim in sims()? {
    searcher.set_similarity(sim);
    let mut query = Builder::new();
    query.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
    assert_eq!(1, searcher.search(query.build(), 10)?.total_hits.value());
  }
  Ok(())
}

/** make sure scores are not skewed by docs not containing the field */
#[test]
fn test_no_field_skew() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut iw_config = new_index_writer_config(&mut random);
  iw_config.set_merge_policy(new_log_merge_policy(&mut random)?);
  let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), iw_config);
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "foo",
    "bar baz somethingelse",
    Store::No,
    &mut field_to_type,
  )?);
  iw.add_document(doc)?;

  let mut query_builder = Builder::new();
  query_builder.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
  query_builder.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
  let query = query_builder.build();

  let sims = sims()?;
  let mut scores = Vec::new();
  {
    let ir = iw.get_reader()?;
    let mut searcher = new_searcher_with_reader(ir)?;
    for sim in &sims {
      searcher.set_similarity(sim.clone());
      scores.push(searcher.explain(query.clone(), 0)?);
    }
  }

  let num_extra_docs = random.random_range(1..=1000);
  for _ in 0..num_extra_docs {
    iw.add_document(Document::new())?;
  }

  {
    let ir = iw.get_reader()?;
    let mut searcher = new_searcher_with_reader(ir)?;
    for (i, sim) in sims.iter().enumerate() {
      searcher.set_similarity(sim.clone());
      let expected = &scores[i];
      let actual = searcher.explain(query.clone(), 0)?;
      assert_eq!(
        expected.get_value().to_f32(),
        actual.get_value().to_f32(),
        "{}: actual={},expected={}",
        sim,
        actual,
        expected
      );
    }
  }

  iw.close()?;
  Ok(())
}

/** make sure all sims work if TF is omitted */
#[test]
fn test_omit_tf() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone());
  let mut doc = Document::new();
  let mut field_to_type = HashMap::new();
  let mut ft = FieldType::from_ref(&*TYPE_NOT_STORED)?;
  ft.set_index_options(IndexOptions::Docs)?;
  ft.freeze();
  doc.add(new_field(
    &mut random,
    "foo",
    "bar",
    &ft,
    &mut field_to_type,
  )?);
  iw.add_document(doc)?;
  let ir = iw.get_reader()?;
  iw.close()?;
  let mut searcher = new_searcher_with_reader(ir)?;

  for sim in sims()? {
    searcher.set_similarity(sim);
    let mut query = Builder::new();
    query.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
    assert_eq!(1, searcher.search(query.build(), 10)?.total_hits.value());
  }
  Ok(())
}

/** make sure all sims work if TF and norms is omitted */
#[test]
fn test_omit_tf_and_norms() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone());
  let mut doc = Document::new();
  let mut field_to_type = HashMap::new();
  let mut ft = FieldType::from_ref(&*TYPE_NOT_STORED)?;
  ft.set_index_options(IndexOptions::Docs)?;
  ft.set_omit_norms(true)?;
  ft.freeze();
  doc.add(new_field(
    &mut random,
    "foo",
    "bar",
    &ft,
    &mut field_to_type,
  )?);
  iw.add_document(doc)?;
  let ir = iw.get_reader()?;
  iw.close()?;
  let mut searcher = new_searcher_with_reader(ir)?;

  for sim in sims()? {
    searcher.set_similarity(sim);
    let mut query = Builder::new();
    query.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
    assert_eq!(1, searcher.search(query.build(), 10)?.total_hits.value());
  }
  Ok(())
}

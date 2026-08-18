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
use crate::core::document::field_type::FieldType;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::standard_directory_reader::StandardDirectoryReader;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::query::Query;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::Result;
use crate::sandbox::search::large_num_hits_top_docs_collector::LargeNumHitsTopDocsCollector;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::check_hits::CheckHits;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_reader, new_string_field, random,
};
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestLargeNumHitsTopDocsCollector;

fn set_up<R>(random: &mut R) -> Result<(Arc<StandardDirectoryReader<DirEnum>>, Query)>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let writer = RandomIndexWriter::new(random, dir)?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
  for _ in 0..1_000 {
    let mut doc = Document::new();
    doc.add(new_string_field(
      random,
      "field",
      "5",
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(random, doc)?;
    writer.add_document(random, Document::new())?;
  }
  let reader = Arc::new(writer.get_reader(random)?);
  writer.close(random)?;

  let mut builder = BooleanQueryBuilder::new();
  builder.add(TermQuery::new(Term::from_text("field", "5")), Occur::Should)?;
  builder.add(MatchAllDocsQuery::new(), Occur::Should)?;
  let test_query: Query = builder.build().into();

  Ok((reader, test_query))
}

#[test]
fn test_request_more_hits_than_collected() -> Result<()> {
  run_num_hits(150)
}

#[test]
fn test_single_num_hit() -> Result<()> {
  run_num_hits(1)
}

#[test]
fn test_request_less_hits_than_collected() -> Result<()> {
  run_num_hits(25)
}

#[test]
fn test_illegal_arguments() -> Result<()> {
  let mut random = random();
  let (reader, test_query) = set_up(&mut random)?;
  let searcher = new_searcher_with_reader(reader.clone())?;
  let mut large_collector = LargeNumHitsTopDocsCollector::new(15);
  let regular_collector_manager = TopScoreDocCollectorManager::new(15, i32::MAX as usize)?;

  searcher.search_with_collector(test_query.clone(), &mut large_collector)?;
  let top_docs =
    searcher.search_with_collector_manager(test_query.clone(), &regular_collector_manager)?;

  assert_eq!(large_collector.total_hits, top_docs.total_hits.value());

  match large_collector.top_docs_with_how_many(350_000) {
    Ok(_) => unreachable!("top_docs_with_how_many should reject oversized requests"),
    Err(expected) => {
      assert!(
        expected
          .to_string()
          .contains("Incorrect number of hits requested")
      );
    },
  }
  Ok(())
}

#[test]
fn test_no_pq_build() -> Result<()> {
  let mut random = random();
  let (reader, test_query) = set_up(&mut random)?;
  let searcher = new_searcher_with_reader(reader.clone())?;
  let mut large_collector = LargeNumHitsTopDocsCollector::new(250_000);
  let regular_collector_manager =
    TopScoreDocCollectorManager::new(reader.num_docs()? as usize, i32::MAX as usize)?;

  searcher.search_with_collector(test_query.clone(), &mut large_collector)?;
  let top_docs =
    searcher.search_with_collector_manager(test_query.clone(), &regular_collector_manager)?;

  assert_eq!(large_collector.total_hits, top_docs.total_hits.value());

  assert!(large_collector.pq.is_none());
  assert!(
    large_collector
      .pq
      .as_ref()
      .and_then(|pq| pq.top())
      .is_none()
  );
  Ok(())
}

#[test]
fn test_pq_build() -> Result<()> {
  let mut random = random();
  let (reader, test_query) = set_up(&mut random)?;
  let searcher = new_searcher_with_reader(reader.clone())?;
  let mut large_collector = LargeNumHitsTopDocsCollector::new(50);
  let regular_collector_manager = TopScoreDocCollectorManager::new(50, i32::MAX as usize)?;

  searcher.search_with_collector(test_query.clone(), &mut large_collector)?;
  let top_docs =
    searcher.search_with_collector_manager(test_query.clone(), &regular_collector_manager)?;

  assert_eq!(large_collector.total_hits, top_docs.total_hits.value());

  assert!(large_collector.pq.is_some());
  assert!(
    large_collector
      .pq
      .as_ref()
      .and_then(|pq| pq.top())
      .is_some()
  );
  Ok(())
}

#[test]
fn test_no_pq_hits_order() -> Result<()> {
  let mut random = random();
  let (reader, test_query) = set_up(&mut random)?;
  let searcher = new_searcher_with_reader(reader.clone())?;
  let mut large_collector = LargeNumHitsTopDocsCollector::new(250_000);
  let regular_collector_manager =
    TopScoreDocCollectorManager::new(reader.num_docs()? as usize, i32::MAX as usize)?;

  searcher.search_with_collector(test_query.clone(), &mut large_collector)?;
  let mut top_docs =
    searcher.search_with_collector_manager(test_query.clone(), &regular_collector_manager)?;

  assert_eq!(large_collector.total_hits, top_docs.total_hits.value());

  assert!(large_collector.pq.is_none());
  assert!(
    large_collector
      .pq
      .as_ref()
      .and_then(|pq| pq.top())
      .is_none()
  );

  top_docs = large_collector.top_docs()?;

  if !top_docs.score_docs.is_empty() {
    let mut pre_score = top_docs.score_docs[0].score;
    for score_doc in top_docs.score_docs {
      assert!(score_doc.score <= pre_score);
      pre_score = score_doc.score;
    }
  }
  Ok(())
}

fn run_num_hits(num_hits: usize) -> Result<()> {
  let mut random = random();
  let (reader, test_query) = set_up(&mut random)?;
  let searcher = new_searcher_with_reader(reader.clone())?;
  let mut large_collector = LargeNumHitsTopDocsCollector::new(num_hits);
  let regular_collector_manager = TopScoreDocCollectorManager::new(num_hits, i32::MAX as usize)?;

  searcher.search_with_collector(test_query.clone(), &mut large_collector)?;

  let first_top_docs = large_collector.top_docs()?;
  let second_top_docs =
    searcher.search_with_collector_manager(test_query.clone(), &regular_collector_manager)?;

  assert_eq!(
    large_collector.total_hits,
    second_top_docs.total_hits.value()
  );
  assert_eq!(
    first_top_docs.score_docs.len(),
    second_top_docs.score_docs.len()
  );
  CheckHits::check_equal(
    &test_query,
    &first_top_docs.score_docs,
    &second_top_docs.score_docs,
  )
}

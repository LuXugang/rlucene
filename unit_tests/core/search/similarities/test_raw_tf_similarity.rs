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
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::test::support::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, new_searcher_with_reader, new_text_field, random,
};
use std::collections::HashMap;

use crate::core::index::directory_reader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::term::Term;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::similarities_impl::raw_tf_similarity::RawTFSimilarity;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use crate::search_tests::similarities::base_similarity_test_case::BaseSimilarityTestCase;
use crate::test::support::core::util::DefaultIndexSearchCR;
use rand::Rng;

#[allow(dead_code)]
struct TestRawTFSimilarity;
fn set_up<R>(random: &mut R) -> Result<DefaultIndexSearchCR>
where
  R: Rng + ?Sized,
{
  let directory = new_directory_shared(random)?;

  {
    let index_writer = IndexWriter::new(directory.clone(), new_index_writer_config(random)?)?;
    let mut field_types = HashMap::new();
    let mut document1 = Document::new();
    let mut document2 = Document::new();
    let mut document3 = Document::new();

    document1.add(new_text_field(
      random,
      "test",
      "one",
      Store::Yes,
      &mut field_types,
    )?);
    document2.add(new_text_field(
      random,
      "test",
      "two two",
      Store::Yes,
      &mut field_types,
    )?);
    document3.add(new_text_field(
      random,
      "test",
      "three three three",
      Store::Yes,
      &mut field_types,
    )?);

    index_writer.add_document(document1)?;
    index_writer.add_document(document2)?;
    index_writer.add_document(document3)?;
    index_writer.commit()?;
  }

  let index_reader = directory_reader::open(directory)?;
  let mut index_searcher = new_searcher_with_reader(index_reader)?;
  index_searcher.set_similarity(RawTFSimilarity::default());

  Ok(index_searcher)
}
#[test]
fn test_one() -> Result<()> {
  let mut random = random();
  let index_searcher = set_up(&mut random)?;
  impl_test(&index_searcher, "one", 1.0)?;
  Ok(())
}

#[test]
fn test_two() -> Result<()> {
  let mut random = random();
  let index_searcher = set_up(&mut random)?;
  impl_test(&index_searcher, "two", 2.0)?;
  Ok(())
}

#[test]
fn test_three() -> Result<()> {
  let mut random = random();
  let index_searcher = set_up(&mut random)?;
  impl_test(&index_searcher, "three", 3.0)?;
  Ok(())
}

fn impl_test(index_searcher: &DefaultIndexSearchCR, text: &str, expected_score: f32) -> Result<()> {
  let query = TermQuery::new(Term::from_text("test", text));
  let top_docs = index_searcher.search(query, 1)?;

  assert_eq!(1, top_docs.total_hits.value());
  assert_eq!(1, top_docs.score_docs.len());
  assert_eq!(expected_score, top_docs.score_docs[0].score);

  Ok(())
}
#[test]
fn test_boost_query() -> Result<()> {
  let mut random = random();
  let index_searcher = set_up(&mut random)?;

  let query = TermQuery::new(Term::from_text("test", "three"));
  let boost = 14.0f32;

  let top_docs = index_searcher.search(BoostQuery::new(query, boost)?, 1)?;

  assert_eq!(1, top_docs.total_hits.value());
  assert_eq!(1, top_docs.score_docs.len());
  assert_eq!(42.0f32, top_docs.score_docs[0].score);

  Ok(())
}

impl BaseSimilarityTestCase for TestRawTFSimilarity {
  type Similarity = RawTFSimilarity;

  fn get_similarity<R>(&self, _random: &mut R) -> Result<Self::Similarity>
  where
    R: Rng + ?Sized,
  {
    Ok(RawTFSimilarity::default())
  }
}
#[test]
fn test_random_scoring() -> Result<()> {
  let mut random = random();
  let case = TestRawTFSimilarity;
  case.test_random_scoring(&mut random)
}

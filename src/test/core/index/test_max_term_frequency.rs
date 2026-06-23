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
use crate::core::document::text_field::TextField;
use crate::core::index::field_invert_state::FieldInvertState;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_doc_values::MultiDocValues;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::similarities_impl::similarities::BoxSimScorer;
use crate::core::search::similarities_impl::similarities::{SimScorer, Similarity, SimilarityEnum};
use crate::core::search::term_statistics::TermStatistics;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::analysis::mock_tokenizer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_log_merge_policy, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use rand::prelude::SliceRandom;
use std::fmt::{Display, Formatter};

/// Tests the maxTermFrequency statistic in FieldInvertState
#[allow(dead_code)] // for quick search
pub struct TestMaxTermFrequency;

/// Simple similarity that encodes maxTermFrequency directly as a byte
#[derive(Default, Clone)]
struct TestSimilarity;

impl Display for TestSimilarity {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl Similarity for TestSimilarity {
  type SimScorer = BoxSimScorer;

  fn compute_norm(&self, state: &FieldInvertState) -> Result<i64> {
    Ok(state.get_max_term_frequency() as i64)
  }

  fn scorer(
    &self,
    _boost: f32,
    _collection_stats: &CollectionStatistics,
    _term_stats: &[TermStatistics],
  ) -> Result<Self::SimScorer> {
    Ok(Box::new(TestSimScorer))
  }
}

struct TestSimScorer;

impl SimScorer for TestSimScorer {
  fn score(&self, _freq: f32, _norm: i64) -> f32 {
    0f32
  }
}

/// Makes a bunch of single-char tokens (the max freq will at most be 255).
/// Shuffles them around, and returns the whole list with debug formatting.
/// This works fine because we use lettertokenizer. Puts the max-frequency term
/// into expected, to be checked against the norm.
fn add_value<R: Rng + ?Sized>(random: &mut R, expected: &mut Vec<i32>) -> String {
  let mut terms = Vec::new();
  let max_ceiling = TestUtil::next_int(random, 0, 255);
  let mut max = 0;

  for ch in 'a'..='z' {
    let num = TestUtil::next_int(random, 0, max_ceiling);
    for _ in 0..num {
      terms.push(ch.to_string());
    }
    max = max.max(num);
  }

  expected.push(max);
  terms.shuffle(random);
  format!("{terms:?}")
}

#[test]
fn test() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::with_automaton(&mut random, mock_tokenizer::SIMPLE.clone(), true);
  let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  config.set_merge_policy(new_log_merge_policy(&mut random)?);
  config.set_similarity(SimilarityEnum::custom(TestSimilarity));

  let writer = RandomIndexWriter::with_config(&mut random, dir.clone(), config);

  let mut expected = Vec::new();
  for _ in 0..100 {
    let value = add_value(&mut random, &mut expected);
    let mut doc = Document::new();
    doc.add(TextField::from_string("foo", value, Store::No)?);
    writer.add_document(&mut random, doc)?;
  }

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let mut foo_norms = MultiDocValues::get_norm_values(&reader, "foo")?
    .ok_or_else(|| LuceneError::illegal_state("norms missing for field foo"))?;

  for i in 0..reader.max_doc()? {
    assert_eq!(i, foo_norms.next_doc()?);
    assert_eq!(expected[i as usize] as i64, foo_norms.long_value()? & 0xff);
  }
  reader.close()?;

  Ok(())
}

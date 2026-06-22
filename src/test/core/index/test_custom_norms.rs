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
use crate::core::document::field::Store;
use crate::core::document::text_field::TextField;
use crate::core::index::directory_reader;
use crate::core::index::field_invert_state::FieldInvertState;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::MAX_TERM_LENGTH;
use crate::core::index::multi_doc_values::MultiDocValues;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::stored_fields::StoredFields;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::similarities_impl::classic_similarity;
use crate::core::search::similarities_impl::per_field_similarity_wrapper::PerFieldSimilarityWrapper;
use crate::core::search::similarities_impl::similarities::{
  BoxSimScorer, Similarity, SimilarityEnum,
};
use crate::core::search::similarities_impl::tf_idf_similarity::TFIDFSimilarity;
use crate::core::search::term_statistics::TermStatistics;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::line_file_docs::LineFileDocs;
use crate::test::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, random, rarely,
};
use crate::test::core::util::test_util::TestUtil;
use std::fmt::{Display, Formatter};

#[allow(dead_code)] // for quick search
pub struct TestCustomNorms;

const FLOAT_TEST_FIELD: &str = "normsTestFloat";

#[test]
fn test_float_norms() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let mut analyzer = MockAnalyzer::new(&mut random);
  analyzer.set_max_token_length(TestUtil::next_int(&mut random, 2, MAX_TERM_LENGTH));

  let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let provider = MySimProvider;
  config.set_similarity(SimilarityEnum::custom(provider));
  let writer = RandomIndexWriter::with_config(&mut random, dir.clone(), config);
  let mut docs = LineFileDocs::new(&mut random)?;
  let num = at_least(&mut random, 100);

  for _i in 0..num {
    let mut doc = docs.next_doc()?;
    let boost = TestUtil::next_int(&mut random, 1, 10);
    let value = (0..boost)
      .map(|_| boost.to_string())
      .collect::<Vec<_>>()
      .join(" ");
    let f = TextField::from_string(FLOAT_TEST_FIELD, value, Store::Yes)?;

    doc.add(f);
    writer.add_document(doc.clone())?;
    doc.remove_field(FLOAT_TEST_FIELD);
    if rarely(&mut random) {
      writer.commit()?;
    }
  }
  writer.commit()?;
  writer.close()?;

  let open = directory_reader::open(dir)?;
  let mut norms = MultiDocValues::get_norm_values(&open, FLOAT_TEST_FIELD)?
    .ok_or_else(|| LuceneError::illegal_state("norms is None"))?;
  let mut stored_fields = open.stored_fields()?;
  for i in 0..open.max_doc()? {
    let document = stored_fields.document(i)?;
    let expected: i64 = document
      .get(FLOAT_TEST_FIELD)?
      .ok_or_else(|| LuceneError::illegal_state("missing custom norms field"))?
      .split(' ')
      .next()
      .ok_or_else(|| LuceneError::illegal_state("missing first token"))?
      .parse()?;
    assert_eq!(i, norms.next_doc()?);
    assert_eq!(expected, norms.long_value()?);
  }
  open.close()?;
  docs.close();
  Ok(())
}

#[derive(Default, Clone)]
pub struct FloatEncodingBoostSimilarity;

impl Display for FloatEncodingBoostSimilarity {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl Similarity for FloatEncodingBoostSimilarity {
  fn compute_norm(&self, state: &FieldInvertState) -> Result<i64> {
    Ok(state.get_length() as i64)
  }

  type SimScorer = BoxSimScorer;

  fn scorer(
    &self,
    _boost: f32,
    _collection_stats: &CollectionStatistics,
    _term_stats: &[TermStatistics],
  ) -> Result<Self::SimScorer> {
    Err(LuceneError::unsupported_operation(""))
  }
}

#[derive(Default, Clone)]
struct MySimProvider;

impl Similarity for MySimProvider {
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
    Ok(Box::new(PerFieldSimilarityWrapper::scorer(
      self,
      boost,
      collection_stats,
      term_stats,
    )?))
  }
}

impl Display for MySimProvider {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl PerFieldSimilarityWrapper for MySimProvider {
  type Similarity = TestSimilarityEnum;

  fn get(&self, field: &str) -> Self::Similarity {
    if FLOAT_TEST_FIELD == field {
      TestSimilarityEnum::FloatEncodingBoost(FloatEncodingBoostSimilarity)
    } else {
      TestSimilarityEnum::Classic(classic_similarity::new())
    }
  }
}

pub enum TestSimilarityEnum {
  FloatEncodingBoost(FloatEncodingBoostSimilarity),
  Classic(TFIDFSimilarity),
}

impl Display for TestSimilarityEnum {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      TestSimilarityEnum::FloatEncodingBoost(v) => write!(f, "{}", v),
      TestSimilarityEnum::Classic(v) => write!(f, "{}", v),
    }
  }
}

impl Similarity for TestSimilarityEnum {
  fn get_discount_overlaps(&self) -> bool {
    match self {
      TestSimilarityEnum::FloatEncodingBoost(_) => true,
      TestSimilarityEnum::Classic(_) => false,
    }
  }

  fn compute_norm(&self, state: &FieldInvertState) -> Result<i64> {
    match self {
      TestSimilarityEnum::FloatEncodingBoost(v) => v.compute_norm(state),
      TestSimilarityEnum::Classic(v) => v.compute_norm(state),
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
      TestSimilarityEnum::FloatEncodingBoost(v) => v.scorer(boost, collection_stats, term_stats),
      TestSimilarityEnum::Classic(v) => {
        let v = v.scorer(boost, collection_stats, term_stats)?;
        Ok(Box::new(v))
      },
    }
  }
}

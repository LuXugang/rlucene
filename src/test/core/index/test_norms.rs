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
use crate::core::index::directory_reader;
use crate::core::index::field_invert_state::FieldInvertState;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::{IndexWriter, MAX_TERM_LENGTH};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
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
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::support::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
use crate::test::support::core::util::lucene_test_case::{
  at_least, create_temp_dir_with_prefix, get_only_leaf_reader, new_directory_shared,
  new_fs_directory, new_index_writer_config, new_index_writer_config_with_analyzer,
  new_log_merge_policy, new_text_field, random,
};
use crate::test::support::core::util::test_util::TestUtil;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
/// Test that norms info is preserved during index life - including separate norms, addDocument, addIndexes, forceMerge.
#[allow(dead_code)] // for quick search
pub struct TestNorms;

const BYTE_TEST_FIELD: &str = "byte_norms";
#[test]
fn test_max_byte_norms() -> Result<()> {
  let mut random = random();
  let dir = new_fs_directory(
    &mut random,
    create_temp_dir_with_prefix("TestNorms.testMaxByteNorms")?,
  )?;
  build_index(dir.clone())?;

  let open = directory_reader::open(dir.clone())?;
  let mut norm_values = MultiDocValues::get_norm_values(&open, BYTE_TEST_FIELD)?
    .ok_or_else(|| LuceneError::illegal_state("norm_values is None"))?;
  let mut stored_fields = open.stored_fields()?;

  for i in 0..open.max_doc()? {
    let document = stored_fields.document(i)?;
    let field_value = document
      .get(BYTE_TEST_FIELD)?
      .ok_or_else(|| LuceneError::illegal_state("field value is None"))?;
    let expected: i64 = field_value
      .split(' ')
      .next()
      .ok_or_else(|| LuceneError::illegal_state("missing first token"))?
      .parse()?;

    assert_eq!(i, norm_values.next_doc()?);
    assert_eq!(expected, norm_values.long_value()?);
  }
  open.close()?;
  Ok(())
}
pub fn build_index(dir: Arc<DirEnum>) -> Result<()> {
  let mut random = random();
  let mut analyzer = MockAnalyzer::new(&mut random);
  // we need at least 3 for maxTokenLength otherwise norms are messed up
  analyzer.set_max_token_length(TestUtil::next_int(&mut random, 3, MAX_TERM_LENGTH));

  let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  let provider = MySimProvider;
  config.set_similarity(SimilarityEnum::custom(provider));

  let writer = RandomIndexWriter::with_config(&mut random, dir, config);
  let num = at_least(&mut random, 100);

  for _ in 0..num {
    let mut doc = Document::new();
    let boost = TestUtil::next_int(&mut random, 1, 255);
    let value = (0..boost)
      .map(|_| boost.to_string())
      .collect::<Vec<_>>()
      .join(" ");

    let field = TextField::from_string(BYTE_TEST_FIELD, value, Store::Yes)?;
    doc.add(field);
    writer.add_document(&mut random, doc)?;
  }

  writer.commit(&mut random)?;
  writer.close(&mut random)?;
  Ok(())
}
#[test]
fn test_empty_value_vs_no_value() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);

  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let mut doc = Document::new();
  writer.add_document(doc.clone())?;
  let mut field_to_type = HashMap::new();
  doc.add(new_text_field(
    &mut random,
    "foo",
    "",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(doc)?;

  writer.force_merge(1)?;

  let reader = directory_reader::open_from_writer(&writer)?;
  writer.close()?;

  let leaf_reader = get_only_leaf_reader(&reader)?;
  let mut norm_values = leaf_reader.get_norm_values("foo")?.unwrap();

  assert_eq!(1, norm_values.next_doc()?); // doc 0 does not have norms
  assert_eq!(0, norm_values.long_value()?);

  reader.close()?;
  Ok(())
}

#[derive(Default, Clone)]
pub struct ByteEncodingBoostSimilarity;

impl Display for ByteEncodingBoostSimilarity {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl Similarity for ByteEncodingBoostSimilarity {
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
    if BYTE_TEST_FIELD == field {
      TestSimilarityEnum::ByteEncodingBoost(ByteEncodingBoostSimilarity)
    } else {
      TestSimilarityEnum::Classic(classic_similarity::new())
    }
  }
}
pub enum TestSimilarityEnum {
  ByteEncodingBoost(ByteEncodingBoostSimilarity),
  Classic(TFIDFSimilarity),
}

impl Display for TestSimilarityEnum {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      TestSimilarityEnum::ByteEncodingBoost(v) => write!(f, "{}", v),
      TestSimilarityEnum::Classic(v) => write!(f, "{}", v),
    }
  }
}

impl Similarity for TestSimilarityEnum {
  fn get_discount_overlaps(&self) -> bool {
    match self {
      TestSimilarityEnum::ByteEncodingBoost(_) => true,
      TestSimilarityEnum::Classic(_) => false,
    }
  }

  fn compute_norm(&self, state: &FieldInvertState) -> Result<i64> {
    match self {
      TestSimilarityEnum::ByteEncodingBoost(v) => v.compute_norm(state),
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
      TestSimilarityEnum::ByteEncodingBoost(v) => v.scorer(boost, collection_stats, term_stats),
      TestSimilarityEnum::Classic(v) => {
        let v = v.scorer(boost, collection_stats, term_stats)?;
        Ok(Box::new(v))
      },
    }
  }
}

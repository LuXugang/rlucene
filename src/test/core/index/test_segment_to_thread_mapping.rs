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
use crate::core::codecs::dummy::dummy_binary_doc_values::DummyBinaryDocValues;
use crate::core::codecs::dummy::dummy_doc_values_skipper::DummyDocValuesSkipper;
use crate::core::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_doc_values::DummySortedDocValues;
use crate::core::codecs::dummy::dummy_sorted_numeric_doc_values::DummySortedNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_set_doc_values::DummySortedSetDocValues;
use crate::core::document::document::Document;
use crate::core::index::composite_reader::get_context;
use crate::core::index::dummy::dummy_byte_vector_values::DummyByteVectorValues;
use crate::core::index::dummy::dummy_cache_helper::DummyCacheHelper;
use crate::core::index::dummy::dummy_float_vector_values::DummyFloatVectorValues;
use crate::core::index::dummy::dummy_point_value_base::DummyPointValues;
use crate::core::index::dummy::dummy_stored_fields::DummyStoredFields;
use crate::core::index::dummy::dummy_term_vectors::DummyTermVectors;
use crate::core::index::dummy::dummy_terms::DummyTerms;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::index_reader::{IndexReader, IndexReaderBase};
use crate::core::index::leaf_metadata::LeafMetaData;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::{LeafReaderContext, TopParentMeta};
use crate::core::index::term::Term;
use crate::core::search::index_searcher::{IndexSearcher, do_slices};
use crate::core::search::knn_collector::KnnCollector;
use crate::core::util::bits::Bits;
use crate::core::util::dummy::dummy_bits::DummyBits;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
use rand::seq::SliceRandom;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
#[allow(dead_code)] // for quick search
struct TestSegmentToThreadMapping;

#[derive(Clone)]
struct DummyIndexReader {
  max_doc: i32,
}

impl DummyIndexReader {
  fn new(max_doc: i32) -> Self {
    Self { max_doc }
  }
}

impl Display for DummyIndexReader {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "DummyIndexReader({})", self.max_doc)
  }
}

impl IndexReader for DummyIndexReader {
  type TermVectors = DummyTermVectors;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    dummy_unreachable!()
  }

  fn max_doc(&self) -> Result<i32> {
    Ok(self.max_doc)
  }

  fn num_docs(&self) -> Result<i32> {
    Ok(self.max_doc)
  }

  type StoredFields = DummyStoredFields;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    dummy_unreachable!()
  }

  fn do_close(&self) -> Result<()> {
    dummy_unreachable!()
  }

  type ReaderCacheHelper = DummyCacheHelper;

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    Ok(None)
  }

  fn doc_freq(&self, _term: &Term) -> Result<i32> {
    dummy_unreachable!()
  }

  fn total_term_freq(&self, _term: &Term) -> Result<i64> {
    dummy_unreachable!()
  }

  fn get_sum_doc_freq(&self, _field: &str) -> Result<i64> {
    dummy_unreachable!()
  }

  fn get_doc_count(&self, _field: &str) -> Result<i32> {
    dummy_unreachable!()
  }

  fn get_sum_total_term_freq(&self, _field: &str) -> Result<i64> {
    dummy_unreachable!()
  }

  fn index_base(&self) -> &IndexReaderBase {
    dummy_unreachable!()
  }
}

impl LeafReader for DummyIndexReader {
  type CacheHelper = DummyCacheHelper;

  fn get_core_cache_helper_ref(&self) -> Result<Option<&Self::CacheHelper>> {
    Ok(None)
  }

  fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>> {
    Ok(None)
  }

  type Terms = DummyTerms;

  fn terms(&self, _field: &str) -> Result<Option<Self::Terms>> {
    Ok(None)
  }

  type NumericDocValues = DummyNumericDocValues;

  fn get_numeric_doc_values(&self, _field: &str) -> Result<Option<Self::NumericDocValues>> {
    Ok(None)
  }

  type BinaryDocValues = DummyBinaryDocValues;

  fn get_binary_doc_values(&self, _field: &str) -> Result<Option<Self::BinaryDocValues>> {
    Ok(None)
  }

  type SortedDocValues = DummySortedDocValues;

  fn get_sorted_doc_values(&self, _field: &str) -> Result<Option<Self::SortedDocValues>> {
    Ok(None)
  }

  type SortedNumericDocValues = DummySortedNumericDocValues;

  fn get_sorted_numeric_doc_values(
    &self,
    _field: &str,
  ) -> Result<Option<Self::SortedNumericDocValues>> {
    Ok(None)
  }

  type SortedSetDocValues = DummySortedSetDocValues;

  fn get_sorted_set_doc_values(&self, _field: &str) -> Result<Option<Self::SortedSetDocValues>> {
    Ok(None)
  }

  type NormNumericDocValues = DummyNumericDocValues;

  fn get_norm_values(&self, _field: &str) -> Result<Option<Self::NormNumericDocValues>> {
    Ok(None)
  }

  type DocValuesSkipper = DummyDocValuesSkipper;

  fn get_doc_values_skipper(&self, _field: &str) -> Result<Option<Self::DocValuesSkipper>> {
    Ok(None)
  }

  type FloatVectorValues = DummyFloatVectorValues;

  fn get_float_vector_values(&self, _field: &str) -> Result<Option<Self::FloatVectorValues>> {
    Ok(None)
  }

  type ByteVectorValues = DummyByteVectorValues;

  fn get_byte_vector_values(&self, _field: &str) -> Result<Option<Self::ByteVectorValues>> {
    Ok(None)
  }

  fn search_nearest_vectors_f32<B, K>(
    &self,
    _field: &str,
    _target: Vec<f32>,
    _knn_collector: &mut K,
    _accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    Ok(())
  }

  fn search_nearest_vectors_u8<B, K>(
    &self,
    _field: &str,
    _target: Vec<u8>,
    _knn_collector: &mut K,
    _accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    Ok(())
  }

  fn get_field_infos(&self) -> Result<Arc<FieldInfos>> {
    Ok(Arc::new(FieldInfos::default()))
  }

  type Bits = DummyBits;

  fn get_live_docs(&self) -> Result<Option<Self::Bits>> {
    Ok(None)
  }

  type PointValues = DummyPointValues;

  fn get_point_values(&self, _field: &str) -> Result<Option<Self::PointValues>> {
    Ok(None)
  }

  fn check_integrity(&self) -> Result<()> {
    Ok(())
  }

  fn get_metadata(&self) -> Result<&LeafMetaData> {
    dummy_unreachable!()
  }
}

fn create_leaf_reader_contexts(max_docs: &[i32]) -> Vec<LeafReaderContext<DummyIndexReader>> {
  let parent = TopParentMeta {
    leaves_num: max_docs.len(),
    max_doc: max_docs.iter().sum(),
    ..Default::default()
  };
  let mut leaf_reader_contexts = Vec::new();
  let mut doc_base = 0;

  for (ord, max_doc) in max_docs.iter().copied().enumerate() {
    leaf_reader_contexts.push(LeafReaderContext::new(
      DummyIndexReader::new(max_doc),
      ord as i32,
      doc_base,
      ord,
      doc_base,
      parent.clone(),
    ));
    doc_base += max_doc as usize;
  }

  leaf_reader_contexts.shuffle(&mut random());
  leaf_reader_contexts
}

#[test]
fn test_single_slice() -> Result<()> {
  let leaf_reader_contexts = create_leaf_reader_contexts(&[50_000, 30_000, 30_000, 30_000]);
  let result_slices = do_slices(
    &leaf_reader_contexts,
    250_000,
    TestUtil::next_int(&mut random(), 4, 10) as usize,
    false,
  )?;
  assert_eq!(1, result_slices.len());
  assert_eq!(4, result_slices[0].partitions.len());
  Ok(())
}

#[test]
fn test_single_slice_with_partitions() -> Result<()> {
  let leaf_reader_contexts = create_leaf_reader_contexts(&[50_000, 30_000, 30_000, 30_000]);
  let result_slices = do_slices(
    &leaf_reader_contexts,
    250_000,
    TestUtil::next_int(&mut random(), 4, 10) as usize,
    true,
  )?;
  assert_eq!(1, result_slices.len());
  assert_eq!(4, result_slices[0].partitions.len());
  Ok(())
}

#[test]
fn test_max_segments_per_slice() -> Result<()> {
  let leaf_reader_contexts = create_leaf_reader_contexts(&[50_000, 30_000, 30_000, 30_000]);

  let result_slices = do_slices(&leaf_reader_contexts, 250_000, 3, false)?;
  assert_eq!(2, result_slices.len());
  assert_eq!(3, result_slices[0].partitions.len());
  assert_eq!(110_000, result_slices[0].max_docs());
  assert_eq!(1, result_slices[1].partitions.len());
  assert_eq!(30_000, result_slices[1].max_docs());

  let result_slices = do_slices(&leaf_reader_contexts, 250_000, 2, false)?;
  assert_eq!(2, result_slices.len());
  assert_eq!(2, result_slices[0].partitions.len());
  assert_eq!(80_000, result_slices[0].max_docs());
  assert_eq!(2, result_slices[1].partitions.len());
  assert_eq!(60_000, result_slices[1].max_docs());

  let result_slices = do_slices(&leaf_reader_contexts, 250_000, 1, false)?;
  assert_eq!(4, result_slices.len());
  assert_eq!(1, result_slices[0].partitions.len());
  assert_eq!(50_000, result_slices[0].max_docs());
  assert_eq!(1, result_slices[1].partitions.len());
  assert_eq!(30_000, result_slices[1].max_docs());
  assert_eq!(1, result_slices[2].partitions.len());
  assert_eq!(30_000, result_slices[2].max_docs());
  assert_eq!(1, result_slices[3].partitions.len());
  assert_eq!(30_000, result_slices[3].max_docs());
  Ok(())
}

#[test]
fn test_max_segments_per_slice_with_partitions() -> Result<()> {
  let leaf_reader_contexts = create_leaf_reader_contexts(&[50_000, 30_000, 30_000, 30_000]);

  let result_slices = do_slices(&leaf_reader_contexts, 250_000, 3, true)?;
  assert_eq!(2, result_slices.len());
  assert_eq!(3, result_slices[0].partitions.len());
  assert_eq!(110_000, result_slices[0].max_docs());
  assert_eq!(1, result_slices[1].partitions.len());
  assert_eq!(30_000, result_slices[1].max_docs());

  let result_slices = do_slices(&leaf_reader_contexts, 250_000, 2, true)?;
  assert_eq!(2, result_slices.len());
  assert_eq!(2, result_slices[0].partitions.len());
  assert_eq!(80_000, result_slices[0].max_docs());
  assert_eq!(2, result_slices[1].partitions.len());
  assert_eq!(60_000, result_slices[1].max_docs());

  let result_slices = do_slices(&leaf_reader_contexts, 250_000, 1, true)?;
  assert_eq!(4, result_slices.len());
  assert_eq!(1, result_slices[0].partitions.len());
  assert_eq!(50_000, result_slices[0].max_docs());
  assert_eq!(1, result_slices[1].partitions.len());
  assert_eq!(30_000, result_slices[1].max_docs());
  assert_eq!(1, result_slices[2].partitions.len());
  assert_eq!(30_000, result_slices[2].max_docs());
  assert_eq!(1, result_slices[3].partitions.len());
  assert_eq!(30_000, result_slices[3].max_docs());
  Ok(())
}

#[test]
fn test_small_segments() -> Result<()> {
  let leaf_reader_contexts = create_leaf_reader_contexts(&[
    10_000, 10_000, 10_000, 10_000, 10_000, 10_000, 130_000, 130_000,
  ]);

  let result_slices = do_slices(&leaf_reader_contexts, 250_000, 5, false)?;
  assert_eq!(3, result_slices.len());
  assert_eq!(2, result_slices[0].partitions.len());
  assert_eq!(260_000, result_slices[0].max_docs());
  assert_eq!(5, result_slices[1].partitions.len());
  assert_eq!(50_000, result_slices[1].max_docs());
  assert_eq!(1, result_slices[2].partitions.len());
  assert_eq!(10_000, result_slices[2].max_docs());

  let result_slices = do_slices(&leaf_reader_contexts, 130_000, 5, false)?;
  assert_eq!(3, result_slices.len());
  assert_eq!(2, result_slices[0].partitions.len());
  assert_eq!(260_000, result_slices[0].max_docs());
  assert_eq!(5, result_slices[1].partitions.len());
  assert_eq!(50_000, result_slices[1].max_docs());
  assert_eq!(1, result_slices[2].partitions.len());
  assert_eq!(10_000, result_slices[2].max_docs());
  Ok(())
}

#[test]
fn test_small_segments_with_partitions() -> Result<()> {
  let leaf_reader_contexts = create_leaf_reader_contexts(&[
    10_000, 10_000, 10_000, 10_000, 10_000, 10_000, 130_000, 130_000,
  ]);

  let result_slices = do_slices(&leaf_reader_contexts, 250_000, 5, true)?;
  assert_eq!(3, result_slices.len());
  assert_eq!(2, result_slices[0].partitions.len());
  assert_eq!(260_000, result_slices[0].max_docs());
  assert_eq!(5, result_slices[1].partitions.len());
  assert_eq!(50_000, result_slices[1].max_docs());
  assert_eq!(1, result_slices[2].partitions.len());
  assert_eq!(10_000, result_slices[2].max_docs());

  let result_slices = do_slices(&leaf_reader_contexts, 130_000, 5, true)?;
  assert_eq!(3, result_slices.len());
  assert_eq!(2, result_slices[0].partitions.len());
  assert_eq!(260_000, result_slices[0].max_docs());
  assert_eq!(5, result_slices[1].partitions.len());
  assert_eq!(50_000, result_slices[1].max_docs());
  assert_eq!(1, result_slices[2].partitions.len());
  assert_eq!(10_000, result_slices[2].max_docs());
  Ok(())
}

#[test]
fn test_large_slices() -> Result<()> {
  let leaf_reader_contexts = create_leaf_reader_contexts(&[290_900, 170_000, 170_000, 170_000]);
  let result_slices = do_slices(&leaf_reader_contexts, 250_000, 5, false)?;
  assert_eq!(3, result_slices.len());
  assert_eq!(1, result_slices[0].partitions.len());
  assert_eq!(2, result_slices[1].partitions.len());
  assert_eq!(1, result_slices[2].partitions.len());
  Ok(())
}

#[test]
fn test_large_slices_with_partitions() -> Result<()> {
  let leaf_reader_contexts = create_leaf_reader_contexts(&[290_900, 170_000, 170_000, 170_000]);
  let result_slices = do_slices(
    &leaf_reader_contexts,
    250_000,
    TestUtil::next_int(&mut random(), 5, 10) as usize,
    true,
  )?;
  assert_eq!(4, result_slices.len());
  assert_eq!(1, result_slices[0].partitions.len());
  assert_eq!(145_450, result_slices[0].max_docs());
  assert_eq!(1, result_slices[1].partitions.len());
  assert_eq!(145_450, result_slices[1].max_docs());
  assert_eq!(2, result_slices[2].partitions.len());
  assert_eq!(340_000, result_slices[2].max_docs());
  assert_eq!(1, result_slices[3].partitions.len());
  assert_eq!(170_000, result_slices[3].max_docs());
  Ok(())
}

#[test]
fn test_single_segment_partitions() -> Result<()> {
  let leaf_reader_contexts = create_leaf_reader_contexts(&[750_001]);
  let result_slices = do_slices(
    &leaf_reader_contexts,
    250_000,
    TestUtil::next_int(&mut random(), 1, 10) as usize,
    true,
  )?;
  assert_eq!(4, result_slices.len());
  assert_eq!(1, result_slices[0].partitions.len());
  assert_eq!(187_500, result_slices[0].max_docs());
  assert_eq!(1, result_slices[1].partitions.len());
  assert_eq!(187_500, result_slices[1].max_docs());
  assert_eq!(1, result_slices[2].partitions.len());
  assert_eq!(187_500, result_slices[2].max_docs());
  assert_eq!(1, result_slices[3].partitions.len());
  assert_eq!(187_501, result_slices[3].max_docs());
  Ok(())
}

#[test]
fn test_extreme_segments_partitioning() -> Result<()> {
  let leaf_reader_contexts = create_leaf_reader_contexts(&[2, 5, 10]);
  let result_slices = do_slices(&leaf_reader_contexts, 1, 1, true)?;

  assert_eq!(12, result_slices.len());
  for (i, leaf_slice) in result_slices.iter().enumerate() {
    if i > 4 {
      assert_eq!(1, leaf_slice.max_docs());
    } else {
      assert_eq!(2, leaf_slice.max_docs());
    }
    assert_eq!(1, leaf_slice.partitions.len());
  }
  Ok(())
}

#[test]
fn test_intra_slice_doc_id_order() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir);
  w.add_document(Document::new())?;
  w.add_document(Document::new())?;
  w.commit()?;
  w.add_document(Document::new())?;
  w.add_document(Document::new())?;
  w.commit()?;
  let r = w.get_reader()?;
  w.close()?;

  let s = IndexSearcher::from_cr(r)?;
  let slices = s.get_slices()?;
  assert!(!slices.is_empty());

  for leaf_slice in slices.as_slice() {
    let mut previous_doc_base = leaf_slice.partitions[0].doc_base;

    for leaf_reader_context_partition in &leaf_slice.partitions {
      assert!(previous_doc_base <= leaf_reader_context_partition.doc_base);
      previous_doc_base = leaf_reader_context_partition.doc_base;
    }
  }
  Ok(())
}

#[test]
fn test_intra_slice_doc_id_order_with_partitions() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir);
  w.add_document(Document::new())?;
  w.add_document(Document::new())?;
  w.commit()?;
  w.add_document(Document::new())?;
  w.add_document(Document::new())?;
  w.commit()?;
  let r = w.get_reader()?;
  w.close()?;

  let context = get_context(r)?;
  let mut s = IndexSearcher::with_threads(context, 2)?;
  s.set_slice_strategy(|leaves| do_slices(leaves, 1, 1, true));
  let slices = s.get_slices()?;
  assert!(!slices.is_empty());

  for leaf_slice in slices.as_slice() {
    let mut previous_doc_base = leaf_slice.partitions[0].doc_base;

    for leaf_reader_context_partition in &leaf_slice.partitions {
      assert!(previous_doc_base <= leaf_reader_context_partition.doc_base);
      previous_doc_base = leaf_reader_context_partition.doc_base;
    }
  }
  Ok(())
}

#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let max = 500_000;
  let min = 10_000;
  let num_segments = 1 + random.random_range(0..50);
  let max_docs = (0..num_segments)
    .map(|_| random.random_range(min..=max))
    .collect::<Vec<_>>();

  let leaf_reader_contexts = create_leaf_reader_contexts(&max_docs);
  let result_slices = do_slices(&leaf_reader_contexts, 250_000, 5, random.random_bool(0.5))?;
  assert!(!result_slices.is_empty());
  Ok(())
}

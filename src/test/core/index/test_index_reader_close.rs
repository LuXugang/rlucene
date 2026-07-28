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
use crate::core::index::directory_reader;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::index_reader::{
  CacheHelper, CacheKey, ClosedListener, IndexReader, IndexReaderBase, LeafReaderContextKind,
};
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_metadata::LeafMetaData;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::util::bits::Bits;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  get_only_leaf_reader, new_directory_shared, new_index_writer_config, random, rarely,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::RngExt;
use std::fmt::{Display, Formatter};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

#[allow(dead_code)] // for quick search
struct TestIndexReaderClose;

#[test]
fn test_close_under_exception() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;
  writer.add_document(Document::new())?;
  writer.commit()?;
  writer.close()?;
  let iters = 1000 + 1 + random.random_range(0..20);
  for _ in 0..iters {
    let open = directory_reader::open(dir.clone())?;
    let throw_on_close = !rarely(&mut random);
    let leaf = get_only_leaf_reader(open)?;
    let reader = CloseUnderExceptionFilterLeafReader::new(leaf, throw_on_close)?;
    let listener_count = random.random_range(0..20);
    let count = Arc::new(AtomicI32::new(0));
    let mut faulty_set = false;
    for _ in 0..listener_count {
      if rarely(&mut random) {
        faulty_set = true;
        reader
          .get_reader_cache_helper()?
          .unwrap()
          .add_closed_listener(Arc::new(FaultyListener))?;
      } else {
        count.fetch_add(1, Ordering::SeqCst);
        let cache_helper = reader.get_reader_cache_helper()?.unwrap();
        cache_helper.add_closed_listener(Arc::new(CountListener::new(
          count.clone(),
          cache_helper.get_key(),
        )))?;
      }
    }
    if !faulty_set && !throw_on_close {
      reader
        .get_reader_cache_helper()?
        .unwrap()
        .add_closed_listener(Arc::new(FaultyListener))?;
    }

    let expected = reader.close().expect_err("reader.close() should fail");
    assert!(matches!(expected, LuceneError::IllegalState(_)));

    if throw_on_close {
      assert_eq!("BOOM!", expected.to_string());
    } else {
      assert_eq!("GRRRRRRRRRRRR!", expected.to_string());
    }

    assert!(matches!(
      reader.terms("someField"),
      Err(LuceneError::AlreadyClosed(_))
    ));

    if random.random_bool(0.5) {
      reader.close()?; // call it again
    }
    assert_eq!(0, count.load(Ordering::SeqCst));
  }
  dir.close()?;
  Ok(())
}

#[test]
fn test_core_listener_on_wrapper_with_different_cache_key() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone())?;
  let num_docs = TestUtil::next_int(&mut random, 1, 5);
  for _ in 0..num_docs {
    w.add_document(&mut random, Document::new())?;
    if random.random_bool(0.5) {
      w.commit(&mut random)?;
    }
  }
  w.force_merge(&mut random, 1)?;
  w.commit(&mut random)?;
  w.close(&mut random)?;

  let reader = directory_reader::open(w.w.get_directory())?;
  // TODO IMPORTANT: Wrap this leaf with AssertingLeafReader once it has been migrated.
  let leaf_reader = get_only_leaf_reader(&reader)?;

  let num_listeners = TestUtil::next_int(&mut random, 1, 10);
  let mut listeners = Vec::new();
  let counter = Arc::new(AtomicI32::new(num_listeners));

  for _ in 0..num_listeners {
    let cache_helper = leaf_reader.get_core_cache_helper()?.unwrap();
    let listener = Arc::new(CountListener::new(counter.clone(), cache_helper.get_key()));
    listeners.push(listener.clone());
    cache_helper.add_closed_listener(listener)?;
  }
  for _ in 0..100 {
    let listener = listeners[random.random_range(0..listeners.len())].clone();
    leaf_reader
      .get_core_cache_helper()?
      .unwrap()
      .add_closed_listener(listener)?;
  }
  assert_eq!(num_listeners, counter.load(Ordering::SeqCst));
  // make sure listeners are registered on the wrapped reader and that closing any of them has the
  // same effect
  if random.random_bool(0.5) {
    reader.close()?;
  } else {
    leaf_reader.close()?;
  }
  assert_eq!(0, counter.load(Ordering::SeqCst));
  w.w.get_directory().close()?;
  Ok(())
}

struct CountListener {
  count: Arc<AtomicI32>,
  core_cache_key: CacheKey,
}

impl CountListener {
  fn new(count: Arc<AtomicI32>, core_cache_key: CacheKey) -> Self {
    Self {
      count,
      core_cache_key,
    }
  }
}

impl ClosedListener for CountListener {
  fn on_close(&self, core_cache_key: &CacheKey) -> Result<()> {
    assert_eq!(&self.core_cache_key, core_cache_key);
    self.count.fetch_sub(1, Ordering::SeqCst);
    Ok(())
  }
}

struct FaultyListener;

impl ClosedListener for FaultyListener {
  fn on_close(&self, _cache_key: &CacheKey) -> Result<()> {
    Err(LuceneError::illegal_state("GRRRRRRRRRRRR!"))
  }
}

#[test]
fn test_register_listener_on_closed_reader() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;
  w.add_document(Document::new())?;
  let r = Arc::new(directory_reader::open_from_writer(&w)?);
  w.close()?;

  let context = r.clone().get_context()?;
  let leaf = context.leaves()?[0].reader().clone();

  // The reader is open, everything should work
  r.get_reader_cache_helper()?
    .unwrap()
    .add_closed_listener(Arc::new(|_: &CacheKey| Ok(())))?;
  leaf
    .get_reader_cache_helper()?
    .unwrap()
    .add_closed_listener(Arc::new(|_: &CacheKey| Ok(())))?;
  leaf
    .get_core_cache_helper()?
    .unwrap()
    .add_closed_listener(Arc::new(|_: &CacheKey| Ok(())))?;

  // But now we close
  r.close()?;
  assert!(matches!(
    r.get_reader_cache_helper()?
      .unwrap()
      .add_closed_listener(Arc::new(|_: &CacheKey| Ok(()))),
    Err(LuceneError::AlreadyClosed(_))
  ));
  assert!(matches!(
    leaf
      .get_reader_cache_helper()?
      .unwrap()
      .add_closed_listener(Arc::new(|_: &CacheKey| Ok(()))),
    Err(LuceneError::AlreadyClosed(_))
  ));
  assert!(matches!(
    leaf
      .get_core_cache_helper()?
      .unwrap()
      .add_closed_listener(Arc::new(|_: &CacheKey| Ok(()))),
    Err(LuceneError::AlreadyClosed(_))
  ));

  dir.close()?;
  Ok(())
}

struct CloseUnderExceptionFilterLeafReader<LR>
where
  LR: LeafReader,
{
  reader: LR,
  throw_on_close: bool,
  index_base: IndexReaderBase,
}

impl<LR> CloseUnderExceptionFilterLeafReader<LR>
where
  LR: LeafReader,
{
  fn new(reader: LR, throw_on_close: bool) -> Result<Self> {
    let index_base = IndexReaderBase::new();
    reader.register_parent_reader(&index_base)?;
    Ok(Self {
      reader,
      throw_on_close,
      index_base,
    })
  }
}

impl<LR> Display for CloseUnderExceptionFilterLeafReader<LR>
where
  LR: LeafReader,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "FilterLeafReader({})", self.reader)
  }
}

impl<LR> IndexReader for CloseUnderExceptionFilterLeafReader<LR>
where
  LR: LeafReader,
{
  type ContextKind = LeafReaderContextKind;

  type TermVectors = LR::TermVectors;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    self.ensure_open()?;
    self.reader.term_vectors()
  }

  fn max_doc(&self) -> Result<i32> {
    self.reader.max_doc()
  }

  fn num_docs(&self) -> Result<i32> {
    self.reader.num_docs()
  }

  type StoredFields = LR::StoredFields;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    self.ensure_open()?;
    self.reader.stored_fields()
  }

  fn do_close(&self) -> Result<()> {
    let close_result = catch_unwind(AssertUnwindSafe(|| self.reader.close()));
    if self.throw_on_close {
      Err(LuceneError::illegal_state("BOOM!"))
    } else {
      match close_result {
        Ok(result) => result,
        Err(payload) => resume_unwind(payload),
      }
    }
  }

  type ReaderCacheHelper = LR::ReaderCacheHelper;

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    self.reader.get_reader_cache_helper()
  }

  fn doc_freq(&self, term: &Term) -> Result<i32> {
    IndexReader::doc_freq(&self.reader, term)
  }

  fn total_term_freq(&self, term: &Term) -> Result<i64> {
    self.reader.total_term_freq(term)
  }

  fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
    IndexReader::get_sum_doc_freq(&self.reader, field)
  }

  fn get_doc_count(&self, field: &str) -> Result<i32> {
    IndexReader::get_doc_count(&self.reader, field)
  }

  fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
    IndexReader::get_sum_total_term_freq(&self.reader, field)
  }

  fn index_base(&self) -> &IndexReaderBase {
    &self.index_base
  }
}

impl<LR> LeafReader for CloseUnderExceptionFilterLeafReader<LR>
where
  LR: LeafReader,
{
  type CacheHelper = LR::CacheHelper;

  fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>> {
    self.reader.get_core_cache_helper()
  }

  type Terms = LR::Terms;

  fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
    self.ensure_open()?;
    self.reader.terms(field)
  }

  type NumericDocValues = LR::NumericDocValues;

  fn get_numeric_doc_values(&self, field: &str) -> Result<Option<Self::NumericDocValues>> {
    self.ensure_open()?;
    self.reader.get_numeric_doc_values(field)
  }

  type BinaryDocValues = LR::BinaryDocValues;

  fn get_binary_doc_values(&self, field: &str) -> Result<Option<Self::BinaryDocValues>> {
    self.ensure_open()?;
    self.reader.get_binary_doc_values(field)
  }

  type SortedDocValues = LR::SortedDocValues;

  fn get_sorted_doc_values(&self, field: &str) -> Result<Option<Self::SortedDocValues>> {
    self.ensure_open()?;
    self.reader.get_sorted_doc_values(field)
  }

  type SortedNumericDocValues = LR::SortedNumericDocValues;

  fn get_sorted_numeric_doc_values(
    &self,
    field: &str,
  ) -> Result<Option<Self::SortedNumericDocValues>> {
    self.ensure_open()?;
    self.reader.get_sorted_numeric_doc_values(field)
  }

  type SortedSetDocValues = LR::SortedSetDocValues;

  fn get_sorted_set_doc_values(&self, field: &str) -> Result<Option<Self::SortedSetDocValues>> {
    self.ensure_open()?;
    self.reader.get_sorted_set_doc_values(field)
  }

  type NormNumericDocValues = LR::NormNumericDocValues;

  fn get_norm_values(&self, field: &str) -> Result<Option<Self::NormNumericDocValues>> {
    self.ensure_open()?;
    self.reader.get_norm_values(field)
  }

  type DocValuesSkipper = LR::DocValuesSkipper;

  fn get_doc_values_skipper(&self, field: &str) -> Result<Option<Self::DocValuesSkipper>> {
    self.ensure_open()?;
    self.reader.get_doc_values_skipper(field)
  }

  type FloatVectorValues = LR::FloatVectorValues;

  fn get_float_vector_values(&self, field: &str) -> Result<Option<Self::FloatVectorValues>> {
    self.reader.get_float_vector_values(field)
  }

  type ByteVectorValues = LR::ByteVectorValues;

  fn get_byte_vector_values(&self, field: &str) -> Result<Option<Self::ByteVectorValues>> {
    self.reader.get_byte_vector_values(field)
  }

  fn search_nearest_vectors_f32<B, K>(
    &self,
    field: &str,
    target: Vec<f32>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    self
      .reader
      .search_nearest_vectors_f32(field, target, knn_collector, accept_docs)
  }

  fn search_nearest_vectors_u8<B, K>(
    &self,
    field: &str,
    target: Vec<u8>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    self
      .reader
      .search_nearest_vectors_u8(field, target, knn_collector, accept_docs)
  }

  fn get_field_infos(&self) -> Result<Arc<FieldInfos>> {
    self.reader.get_field_infos()
  }

  type Bits = LR::Bits;

  fn get_live_docs(&self) -> Result<Option<Self::Bits>> {
    self.ensure_open()?;
    self.reader.get_live_docs()
  }

  type PointValues = LR::PointValues;

  fn get_point_values(&self, field: &str) -> Result<Option<Self::PointValues>> {
    self.reader.get_point_values(field)
  }

  fn check_integrity(&self) -> Result<()> {
    self.ensure_open()?;
    self.reader.check_integrity()
  }

  fn get_metadata(&self) -> Result<&LeafMetaData> {
    self.ensure_open()?;
    self.reader.get_metadata()
  }
}

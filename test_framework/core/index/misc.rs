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
use crate::core::document::string_field::StringField;
use crate::core::index::BytesRef;
use crate::core::index::doc_values_field_updates::{
  DocValuesFieldInnerIter, DocValuesFieldIterator, DocValuesFieldIteratorEnum,
  DocValuesFieldUpdatesBase,
};
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::field_invert_state::FieldInvertState;
use crate::core::index::freq_prox_terms_writer::FreqProxTermsWriter;
use crate::core::index::freq_prox_terms_writer_per_field::FreqProxTermsWriterPerField;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::multi_doc_values::MultiDocValues;
use crate::core::index::multi_reader::MultiReader;
use crate::core::index::parallel_postings_array::PostingsArrayEnum;
use crate::core::index::term_vectors_consumer::TermVectorsConsumer;
use crate::core::index::terms_hash_per_field::TermsHashPerField;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::int_block_pool::IntBlockPool;
use crate::core::util::{AtomicCounter, ByteBlockPool};
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use crate::core::index::documents_writer_flush_control::{DocumentsWriterFlushControl, Inner};
use crate::core::index::documents_writer_per_thread::DocumentsWriterPerThread;
use crate::core::index::documents_writer_per_thread_pool::DwptWrapper;
use crate::core::index::flush_by_ram_or_counts_policy::FlushByRamOrCountsPolicy;
use crate::core::index::flush_policy::FlushPolicy;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::OneMergeSR;
use crate::core::index::merge_scheduler::{MergeScheduler, MergeSource};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::store::directory::Directory;
use crate::core::util::close::CloseableRef;
use crate::test::support::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::support::core::index::doc_helper::{DATA, DocHelper, FIELDS};
use crate::test::support::core::util::lucene_test_case::{
  new_field, new_index_writer_config_with_analyzer, new_text_field, random,
};
use parking_lot::MutexGuard;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, AtomicI64 as FlushAtomicI64};

static STORED_TEXT_TYPE: LazyLock<FieldType> = LazyLock::new(|| {
  FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)
    .expect("should not fail")
});

pub(crate) fn add_doc<D, R>(
  random: &mut R,
  writer: &IndexWriter<D>,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory + 'static,
  R: rand::Rng + ?Sized,
{
  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "content",
    "aaa",
    Store::No,
    field_types,
  )?);
  let _ = writer.add_document(doc)?;
  Ok(())
}

pub(crate) fn add_doc_with_index<D, R>(
  random: &mut R,
  writer: &IndexWriter<D>,
  index: i32,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory + 'static,
  R: rand::Rng + ?Sized,
{
  let mut doc = Document::new();
  doc.add(new_field(
    random,
    "content",
    format!("aaa {}", index),
    &STORED_TEXT_TYPE,
    field_types,
  )?);
  doc.add(StringField::from_string(
    "id",
    index.to_string(),
    Store::No,
  )?);

  writer.add_document(doc).map(|_| ())
}

pub(crate) fn assert_no_unreferenced_files<D>(dir: Arc<D>, message: &str) -> Result<()>
where
  D: Directory + 'static,
{
  let mut start_files = dir.list_all()?;
  let mut random = random();
  let mock = MockAnalyzer::new(&mut random);
  let writer = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, mock)?,
  )?;
  writer.close()?;
  let mut end_files = dir.list_all()?;

  start_files.sort();
  end_files.sort();

  assert_eq!(
    start_files,
    end_files,
    "{}: before delete:\n    {}\n  after delete:\n    {}",
    message,
    start_files.join("\n    "),
    end_files.join("\n    ")
  );

  Ok(())
}

pub(crate) struct TestSegmentReader;

impl TestSegmentReader {
  pub(crate) fn check_norms<LR>(reader: LR) -> Result<()>
  where
    LR: LeafReader + Clone,
  {
    let multi_readers = MultiReader::with_leaf_reader(vec![reader.clone()])?;
    for f in FIELDS.iter() {
      if *f.field_type().index_options() != IndexOptions::None {
        let field_name = f.name();
        let norms_opt = reader.get_norm_values(field_name)?;
        assert_eq!(norms_opt.is_some(), !f.field_type().omit_norms());
        assert_eq!(norms_opt.is_some(), !DATA.no_norms.contains_key(field_name));
        if norms_opt.is_none() {
          let norms2 = MultiDocValues::get_norm_values(&multi_readers, field_name)?;
          assert!(norms2.is_none());
        }
      }
    }
    Ok(())
  }
}

pub(crate) fn create_index_no_close<D>(
  multi_segment: bool,
  index_name: &str,
  w: &IndexWriter<D>,
) -> Result<()>
where
  D: Directory + 'static,
{
  for i in 0..100 {
    w.add_document(DocHelper::create_document(i, index_name, 4))?;
  }
  if !multi_segment {
    w.force_merge(1)?;
  }
  Ok(())
}

pub(crate) struct TermsHashPerFieldMock {
  pub(crate) field_state: FieldInvertState,
  pub(crate) new_called: AtomicI64,
  pub(crate) add_called: AtomicI64,
  pub(crate) base: Option<FreqProxTermsWriterPerField>,
}

impl TermsHashPerFieldMock {
  pub(crate) fn new_term(
    &mut self,
    term_id: i32,
    doc_id: i32,
    base: &mut TermsHashPerField,
  ) -> Result<()> {
    self.new_called.fetch_add(1, Ordering::SeqCst);
    let term_id = term_id as usize;
    match base
      .bytes_hash
      .bytes_start_array
      .per_field
      .postings_array
      .as_mut()
      .unwrap()
    {
      PostingsArrayEnum::FreqProx(f) => {
        f.last_doc_ids[term_id] = doc_id;
        f.last_doc_codes[term_id] = doc_id << 1;
        match &mut f.term_freqs {
          Some(term_freqs) => {
            term_freqs[term_id] = 1;
          },
          None => unreachable!(),
        }
        Ok(())
      },
      _ => unreachable!(),
    }
  }

  pub(crate) fn add_term(
    &mut self,
    term_id: i32,
    doc_id: i32,
    base: &mut TermsHashPerField,
    int_pool: &mut IntBlockPool,
    byte_pool: &mut ByteBlockPool,
  ) -> Result<()> {
    self.add_called.fetch_add(1, Ordering::SeqCst);
    let term_id = term_id as usize;
    let mut v = Vec::new();
    let mut need_write = false;
    match base
      .bytes_hash
      .bytes_start_array
      .per_field
      .postings_array
      .as_mut()
      .unwrap()
    {
      PostingsArrayEnum::FreqProx(postings) => {
        if doc_id != postings.last_doc_ids[term_id] {
          match &mut postings.term_freqs {
            Some(term_freqs) => {
              need_write = true;
              if 1 == term_freqs[term_id] {
                v.push(postings.last_doc_codes[term_id] | 1);
              } else {
                v.push(postings.last_doc_codes[term_id]);
                v.push(term_freqs[term_id]);
              }
              term_freqs[term_id] = 1;
            },
            None => unreachable!(),
          }
          postings.last_doc_codes[term_id] = (doc_id - postings.last_doc_ids[term_id]) << 1;
          postings.last_doc_ids[term_id] = doc_id;
        } else {
          match &mut postings.term_freqs {
            Some(term_freqs) => {
              let value = term_freqs[term_id] as i64 + 1;
              if value > i32::MAX as i64 {
                return Err(LuceneError::number_overflow("term_freqs"));
              }
              term_freqs[term_id] += 1;
            },
            None => unreachable!(),
          }
        }
      },
      _ => unreachable!(),
    }
    if need_write {
      for x in v {
        base.write_vint(0, x, int_pool, byte_pool)?;
      }
    }
    Ok(())
  }
}

pub(crate) fn new_terms_hash_per_field_mock(
  new_called: AtomicI64,
  add_called: AtomicI64,
) -> TermsHashPerFieldMock {
  let bytes_used = Arc::new(AtomicCounter::new());
  let writer = FreqProxTermsWriter::new(bytes_used, TermVectorsConsumer::default());

  let field_state = FieldInvertState::default();
  let mut field_info = FieldInfo::default();
  field_info.index_options = IndexOptions::DocsAndFreqs;

  let base = FreqProxTermsWriterPerField::new(&writer, Arc::new(field_info), None).unwrap();

  TermsHashPerFieldMock {
    field_state,
    new_called,
    add_called,
    base: Option::from(base),
  }
}

#[allow(dead_code)]
pub struct MockDefaultFlushPolicy {
  pub peak_bytes_without_flush: FlushAtomicI64,
  pub peak_doc_count_without_flush: FlushAtomicI64,
  pub has_marked_pending: AtomicBool,
  pub base: FlushByRamOrCountsPolicy,
}

impl MockDefaultFlushPolicy {
  pub fn new() -> Self {
    Self {
      peak_bytes_without_flush: FlushAtomicI64::new(i32::MIN as i64),
      peak_doc_count_without_flush: FlushAtomicI64::new(i32::MIN as i64),
      has_marked_pending: AtomicBool::new(false),
      base: FlushByRamOrCountsPolicy::new(),
    }
  }
}

impl Default for MockDefaultFlushPolicy {
  fn default() -> Self {
    Self::new()
  }
}

impl FlushPolicy for MockDefaultFlushPolicy {
  fn on_change<D, L>(
    &self,
    control: &DocumentsWriterFlushControl<D>,
    inner: &mut Inner<D>,
    per_thread: Option<&MutexGuard<'_, DocumentsWriterPerThread<D>>>,
    config: &L,
  ) -> Result<()>
  where
    D: Directory,
    L: LiveIndexWriterConfig,
  {
    let Some(dwpt) = per_thread else {
      unreachable!("");
    };

    let mut pending = Vec::new();
    let mut not_pending = Vec::new();
    find_pending(control, &mut pending, &mut not_pending);

    let flush_current = dwpt.is_flush_pending();
    let active_bytes = control.active_bytes(Some(inner));
    let to_flush = if flush_current {
      find_dwpt(&pending, &dwpt.state.id)
    } else if self.base.flush_on_doc_count(config)
      && dwpt.get_num_docs_in_ram() >= config.get_max_buffered_docs()
    {
      find_dwpt(&not_pending, &dwpt.state.id)
    } else if self.base.flush_on_ram(config)
      && active_bytes >= (config.get_ram_buffer_size_mb() * 1024.0 * 1024.0) as i64
    {
      let to_flush = self
        .base
        .find_largest_non_pending_writer_for_thread(control, dwpt)?;
      if let Some(to_flush) = to_flush {
        assert!(!to_flush.state.is_flush_pending());
        Some(to_flush)
      } else {
        None
      }
    } else {
      None
    };

    self.base.on_change(control, inner, Some(dwpt), config)?;

    if let Some(to_flush) = to_flush {
      let list = if flush_current {
        &mut pending
      } else {
        &mut not_pending
      };
      let pos = list
        .iter()
        .position(|dwpt| Arc::ptr_eq(dwpt, &to_flush) || dwpt.state.id == to_flush.state.id)
        .expect("expected DWPT in pending snapshot");
      list.remove(pos);
      assert!(to_flush.state.is_flush_pending());
      self.has_marked_pending.store(true, Ordering::SeqCst);
    } else {
      self
        .peak_bytes_without_flush
        .fetch_max(active_bytes, Ordering::SeqCst);
      self
        .peak_doc_count_without_flush
        .fetch_max(dwpt.get_num_docs_in_ram() as i64, Ordering::SeqCst);
    }

    for per_thread in not_pending {
      assert!(!per_thread.state.is_flush_pending());
    }

    Ok(())
  }
}

fn find_pending<D>(
  flush_control: &DocumentsWriterFlushControl<D>,
  pending: &mut Vec<Arc<DwptWrapper<D>>>,
  not_pending: &mut Vec<Arc<DwptWrapper<D>>>,
) where
  D: Directory,
{
  for (_id, next) in flush_control.per_thread_pool.iterator() {
    if next.state.is_flush_pending() {
      pending.push(next);
    } else {
      not_pending.push(next);
    }
  }
}

fn find_dwpt<D>(writers: &[Arc<DwptWrapper<D>>], state_id: &str) -> Option<Arc<DwptWrapper<D>>>
where
  D: Directory,
{
  writers
    .iter()
    .find(|dwpt| dwpt.state.id == state_id)
    .cloned()
}

pub(crate) struct TestSingleUpdateDocValuesFieldUpdates {
  docs_changed: Vec<i32>,
  has_value: bool,
}

impl TestSingleUpdateDocValuesFieldUpdates {
  pub(crate) fn new(docs_changed: Vec<i32>, has_value: bool) -> Self {
    Self {
      docs_changed,
      has_value,
    }
  }
}

impl Accountable for TestSingleUpdateDocValuesFieldUpdates {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(0)
  }
}

impl DocValuesFieldUpdatesBase for TestSingleUpdateDocValuesFieldUpdates {
  fn finish(&mut self) {}

  fn add_value(&mut self, _doc: i32, _value: i64, _index: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation("add_value"))
  }

  fn add_byte_ref(&mut self, _doc: i32, _value: &BytesRef<Vec<u8>>, _index: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation("add_byte_ref"))
  }

  fn add_iterator<T>(&mut self, _doc_id: i32, _iterator: &mut T, _index: usize) -> Result<()>
  where
    T: DocValuesFieldIterator,
  {
    Err(LuceneError::unsupported_operation("add_iterator"))
  }

  fn iterator(
    &self,
    _inner: DocValuesFieldInnerIter,
    del_gen: i64,
  ) -> Result<DocValuesFieldIteratorEnum> {
    Ok(DocValuesFieldIteratorEnum::SingleUpdate(
      TestSingleUpdateDocValuesFieldIterator::new(
        self.docs_changed.clone(),
        del_gen,
        self.has_value,
      ),
    ))
  }

  fn swap(&mut self, _i: usize, _j: usize) -> Result<()> {
    Ok(())
  }

  fn grow(&mut self, _size: i32) -> Result<()> {
    Ok(())
  }

  fn resize(&mut self, _size: i32) -> Result<()> {
    Ok(())
  }

  fn sub_type(&self) -> DocValuesType {
    DocValuesType::Numeric
  }
}

pub(crate) struct TestSingleUpdateDocValuesFieldIterator {
  docs_changed: Vec<i32>,
  idx: usize,
  doc: i32,
  del_gen: i64,
  has_value: bool,
}

impl TestSingleUpdateDocValuesFieldIterator {
  fn new(docs_changed: Vec<i32>, del_gen: i64, has_value: bool) -> Self {
    Self {
      docs_changed,
      idx: 0,
      doc: -1,
      del_gen,
      has_value,
    }
  }
}

impl DocValuesIterator for TestSingleUpdateDocValuesFieldIterator {}

impl DocIdSetIterator for TestSingleUpdateDocValuesFieldIterator {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    if self.idx >= self.docs_changed.len() {
      self.doc = NO_MORE_DOCS;
      return Ok(self.doc);
    }
    self.doc = self.docs_changed[self.idx];
    self.idx += 1;
    Ok(self.doc)
  }
}

impl DocValuesFieldIterator for TestSingleUpdateDocValuesFieldIterator {
  fn long_value(&self) -> Result<i64> {
    Ok(1)
  }

  fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    Err(LuceneError::unsupported_operation("binary_value"))
  }

  fn del_gen(&self) -> i64 {
    self.del_gen
  }

  fn has_value(&self) -> Result<bool> {
    Ok(self.has_value)
  }
}

pub struct SerialMergeSchedulerImpl {
  may_merge: Arc<AtomicBool>,
  base: SerialMergeScheduler,
}

impl SerialMergeSchedulerImpl {
  pub(crate) fn new(may_merge: Arc<AtomicBool>) -> Self {
    Self {
      may_merge,
      base: SerialMergeScheduler::new(),
    }
  }
}

impl CloseableRef for SerialMergeSchedulerImpl {
  fn close(&self) -> Result<()> {
    self.base.close()
  }
}

impl MergeScheduler for SerialMergeSchedulerImpl {
  fn merge<MS, D>(&self, merge_source: MS, trigger: MergeTrigger) -> Result<()>
  where
    MS: MergeSource<D> + Clone + 'static,
    D: Directory + 'static,
    OneMergeSR<D>: Send + 'static,
  {
    if !self.may_merge.load(Ordering::SeqCst) {
      let merge = merge_source.get_next_merge()?;
      if merge.is_some() {
        return Err(LuceneError::illegal_argument(
          "TEST: we should not need any merging, yet merge policy returned merge",
        ));
      }
    }
    self.base.merge(merge_source, trigger)
  }

  type Directory<D>
    = <SerialMergeScheduler as MergeScheduler>::Directory<D>
  where
    D: Directory;

  fn wrap_for_merge<D>(&self, in_: D) -> Result<Self::Directory<D>>
  where
    D: Directory,
  {
    self.base.wrap_for_merge(in_)
  }

  fn initialize<D>(&mut self, directory: &D) -> Result<()>
  where
    D: Directory,
  {
    self.base.initialize(directory)
  }
}

pub struct TestMergeScheduler {
  ex: Arc<AtomicBool>,
}

impl TestMergeScheduler {
  pub(crate) fn new(ex: Arc<AtomicBool>) -> Self {
    Self { ex }
  }
}

impl CloseableRef for TestMergeScheduler {}

impl MergeScheduler for TestMergeScheduler {
  fn merge<MS, D>(&self, merge_source: MS, _trigger: MergeTrigger) -> Result<()>
  where
    MS: MergeSource<D> + Clone + 'static,
    D: Directory + 'static,
    OneMergeSR<D>: Send + 'static,
  {
    while let Some(mut merge) = merge_source.get_next_merge()? {
      let result: Result<()> = merge_source.merge(&mut merge);
      if result.is_err() {
        self.ex.store(true, Ordering::Relaxed);
        return result;
      }
    }
    Ok(())
  }

  type Directory<D>
    = D
  where
    D: Directory;

  fn wrap_for_merge<D>(&self, in_: D) -> Result<Self::Directory<D>>
  where
    D: Directory,
  {
    Ok(in_)
  }
}

pub struct MyMergeScheduler;

impl CloseableRef for MyMergeScheduler {
  fn close(&self) -> Result<()> {
    Ok(())
  }
}

impl MergeScheduler for MyMergeScheduler {
  fn merge<MS, D>(&self, merge_source: MS, _trigger: MergeTrigger) -> Result<()>
  where
    MS: MergeSource<D> + Clone + 'static,
    D: Directory + 'static,
    OneMergeSR<D>: Send + 'static,
  {
    loop {
      let mut merge = match merge_source.get_next_merge()? {
        Some(merge) => merge,
        None => break,
      };
      merge_source.merge(&mut merge)?;
      if let Some(info) = merge.info.as_ref() {
        assert!(info.info.max_doc()? > 0);
      }
    }
    Ok(())
  }

  type Directory<D>
    = D
  where
    D: Directory;

  fn wrap_for_merge<D>(&self, in_: D) -> Result<Self::Directory<D>>
  where
    D: Directory,
  {
    Ok(in_)
  }
}

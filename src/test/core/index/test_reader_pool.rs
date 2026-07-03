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
use crate::core::document::field::Store::Yes;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::directory_reader;
use crate::core::index::doc_values_field_updates::{
  DocValuesFieldUpdates, DocValuesFieldUpdatesBase,
};
use crate::core::index::field_infos::FieldNumbersLock;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::numeric_doc_values_field_updates::NumericDocValuesFieldUpdates;
use crate::core::index::reader_pool::ReaderPool;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::store::lock_validating_directory_wrapper::LockValidatingDirectoryWrapper;
use crate::core::util::bits::Bits;
use crate::core::util::dummy::dummy_comparator::DummyComparator;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::info_stream::InfoStreamEnum;
use crate::core::util::long_supplier::LongSupplier;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, random,
};

use crate::core::index::codec_reader::CodecReader;
use crate::core::index::index_writer::Inner as IWInner;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::merge_policy::{
  DefaultMergeSpecification, MergeContext, MergePolicy, MergePolicyBase, MergeSpecification,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::segment_reader::DefaultLeafReader;
use rand::Rng;
use rand::RngExt;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestReaderPool;

#[derive(Default)]
struct LongSupplierImpl;
impl LongSupplier for LongSupplierImpl {
  fn get_as_long(&self) -> i64 {
    0
  }
}

#[test]
fn test_drop() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;

  let (field_numbers, index_created_version_major) = build_index(directory.clone(), &mut random)?;

  let mut reader = directory_reader::open(directory.clone())?;
  let segment_infos = &mut reader.segment_infos;
  let lock = directory.obtain_lock("writer_lock")?;
  let lock_dir = Arc::new(LockValidatingDirectoryWrapper::new(directory.clone(), lock));
  let pool = ReaderPool::new::<String, DummyComparator>(
    lock_dir,
    directory.clone(),
    segment_infos,
    Arc::new(InfoStreamEnum::default()),
    None,
    LongSupplierImpl,
    None,
    index_created_version_major,
  )?;
  let idx = random.random_range(0..segment_infos.segments.len());
  let readers_and_updates = {
    let commit_info = segment_infos.info_idx_mut(idx).unwrap();
    pool.get(commit_info.to_meta()?, true, None)?.unwrap()
  };

  let same = pool
    .get(segment_infos.info(idx).unwrap().to_meta()?, false, None)?
    .unwrap();
  assert!(Arc::ptr_eq(&readers_and_updates, &same));
  let info_id = segment_infos
    .info(idx)
    .unwrap()
    .info
    .get_id_key()
    .to_string();
  assert!(pool.drop(&info_id, segment_infos)?);

  if random.random_bool(0.5) {
    assert!(!pool.drop(&info_id, segment_infos)?);
  }
  assert!(
    pool
      .get(segment_infos.info(idx).unwrap().to_meta()?, false, None)?
      .is_none()
  );
  pool.release(
    &readers_and_updates,
    random.random_bool(0.5),
    segment_infos,
    None,
    &field_numbers.lock(),
  )?;
  pool.close(segment_infos)?;
  Ok(())
}
#[test]
fn test_pool_readers() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;

  let (field_numbers, index_created_version_major) = build_index(directory.clone(), &mut random)?;

  let mut reader = directory_reader::open(directory.clone())?;
  let segment_infos = &mut reader.segment_infos;

  let lock = directory.obtain_lock("writer_lock")?;
  let lock_dir = Arc::new(LockValidatingDirectoryWrapper::new(directory.clone(), lock));

  let pool = ReaderPool::new::<String, DummyComparator>(
    lock_dir,
    directory.clone(),
    segment_infos,
    Arc::new(InfoStreamEnum::default()),
    None,
    LongSupplierImpl,
    None,
    index_created_version_major,
  )?;

  let idx = random.random_range(0..segment_infos.segments.len());

  assert!(!pool.is_reader_pooling_enabled());

  let rau = {
    let commit_info = segment_infos.info_idx_mut(idx).unwrap();
    pool.get(commit_info.to_meta()?, true, None)?.unwrap()
  };
  pool.release(
    &rau,
    random.random_bool(0.5),
    segment_infos,
    None,
    &field_numbers.lock(),
  )?;

  assert!(
    pool
      .get(segment_infos.info(idx).unwrap().to_meta()?, false, None)?
      .is_none()
  );
  // now start pooling
  pool.enable_reader_pooling();
  assert!(pool.is_reader_pooling_enabled());

  let rau = {
    let commit_info = segment_infos.info_idx_mut(idx).unwrap();
    pool.get(commit_info.to_meta()?, true, None)?.unwrap()
  };
  pool.release(
    &rau,
    random.random_bool(0.5),
    segment_infos,
    None,
    &field_numbers.lock(),
  )?;

  let pooled = pool
    .get(segment_infos.info(idx).unwrap().to_meta()?, false, None)?
    .unwrap();
  let pooled_again = pool
    .get(segment_infos.info(idx).unwrap().to_meta()?, false, None)?
    .unwrap();
  assert!(Arc::ptr_eq(&pooled, &pooled_again));

  let info_id = segment_infos
    .info(idx)
    .unwrap()
    .info
    .get_id_key()
    .to_string();
  pool.drop(&info_id, segment_infos)?;

  assert_eq!(0, pool.ram_bytes_used());

  for idx in 0..segment_infos.segments.len() {
    let rau = {
      let info = segment_infos.info_idx_mut(idx).unwrap();
      pool.get(info.to_meta()?, true, None)?.unwrap()
    };
    pool.release(
      &rau,
      random.random_bool(0.5),
      segment_infos,
      None,
      &field_numbers.lock(),
    )?;
    assert_eq!(
      0,
      pool.ram_bytes_used(),
      "actual: {}",
      pool.ram_bytes_used()
    );

    let a = pool
      .get(segment_infos.info(idx).unwrap().to_meta()?, false, None)?
      .unwrap();
    let b = pool
      .get(segment_infos.info(idx).unwrap().to_meta()?, false, None)?
      .unwrap();
    assert!(Arc::ptr_eq(&a, &b));
  }
  assert_eq!(0, pool.ram_bytes_used());

  pool.drop_all(segment_infos)?;

  for idx in 0..segment_infos.segments.len() {
    let info = segment_infos.info(idx).unwrap();
    assert!(pool.get(info.to_meta()?, false, None)?.is_none());
  }

  assert_eq!(0, pool.ram_bytes_used());

  pool.close(segment_infos)?;
  Ok(())
}

#[test]
fn test_update() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;

  let (field_numbers, index_created_version_major) = build_index(directory.clone(), &mut random)?;

  let mut reader = directory_reader::open(directory.clone())?;
  let segment_infos = &mut reader.segment_infos;

  let lock = directory.obtain_lock("writer_lock")?;
  let lock_dir = Arc::new(LockValidatingDirectoryWrapper::new(directory.clone(), lock));

  let pool = ReaderPool::new::<String, DummyComparator>(
    lock_dir,
    directory.clone(),
    segment_infos,
    Arc::new(InfoStreamEnum::default()),
    None,
    LongSupplierImpl,
    None,
    index_created_version_major,
  )?;

  let id = random.random_range(0..10);

  if random.random_bool(0.5) {
    pool.enable_reader_pooling();
  }

  for (idx, seg_id) in segment_infos.seg_ids().clone().iter().enumerate() {
    let (read_only_clone, max_doc, readers_and_updates, mut postings) = {
      let commit_info = segment_infos.info_idx_mut(idx).unwrap();
      let readers_and_updates = pool.get(commit_info.to_meta()?, true, None)?.unwrap();
      let read_only_clone = readers_and_updates
        .get_read_only_clone(&IOContext::default_io_context()?, commit_info)?
        .unwrap();

      let term = Term::from_text("id", id.to_string());
      let postings = read_only_clone.postings(&term)?;
      (
        read_only_clone,
        commit_info.info.max_doc()?,
        readers_and_updates,
        postings,
      )
    };
    let mut expect_update = false;
    let mut doc = -1_i32;

    if let Some(ref mut postings) = postings {
      if postings.next_doc()? != NO_MORE_DOCS {
        let sub_update1 = NumericDocValuesFieldUpdates::new()?;
        let mut number_updates =
          DocValuesFieldUpdates::new(max_doc, 0, "number", sub_update1.sub_type(), sub_update1)?;
        doc = postings.doc_id();
        number_updates.add_value(doc, 1000_i64)?;
        number_updates.finish()?;

        readers_and_updates.add_dv_update(number_updates)?;
        expect_update = true;

        assert_eq!(NO_MORE_DOCS, postings.next_doc()?);
        assert!(pool.any_doc_values_changes());
      } else {
        assert!(!pool.any_doc_values_changes());
      }
    } else {
      assert!(!pool.any_doc_values_changes());
    }
    read_only_clone.close()?;
    let written_to_disk: bool;
    if pool.is_reader_pooling_enabled() {
      if random.random_bool(0.5) {
        written_to_disk =
          pool.write_all_doc_values_updates(segment_infos, &field_numbers.lock())?;
        assert!(!readers_and_updates.is_merging());
      } else if random.random_bool(0.5) {
        written_to_disk = pool.commit(segment_infos, &field_numbers.lock())?;
        assert!(!readers_and_updates.is_merging());
      } else {
        written_to_disk = pool.write_doc_values_updates_for_merge(
          vec![seg_id.clone()].as_ref(),
          segment_infos,
          &field_numbers.lock(),
        )?;
        assert!(readers_and_updates.is_merging());
      }
      assert!(!pool.release(
        &readers_and_updates,
        random.random_bool(0.5),
        segment_infos,
        None,
        &field_numbers.lock(),
      )?);
    } else if random.random_bool(0.5) {
      written_to_disk = pool.release(
        &readers_and_updates,
        random.random_bool(0.5),
        segment_infos,
        None,
        &field_numbers.lock(),
      )?;
      assert!(!readers_and_updates.is_merging());
    } else {
      written_to_disk = pool.write_doc_values_updates_for_merge(
        vec![seg_id.clone()].as_ref(),
        segment_infos,
        &field_numbers.lock(),
      )?;
      assert!(readers_and_updates.is_merging());

      assert!(!pool.release(
        &readers_and_updates,
        random.random_bool(0.5),
        segment_infos,
        None,
        &field_numbers.lock(),
      )?);
    }

    assert!(!pool.any_doc_values_changes());
    assert_eq!(expect_update, written_to_disk);

    if expect_update {
      let (readers_and_updates, updated_reader) = {
        let commit_info = segment_infos.info_idx_mut(idx).unwrap();
        let readers_and_updates = pool.get(commit_info.to_meta()?, true, None)?.unwrap();
        let updated_reader = readers_and_updates
          .get_read_only_clone(&IOContext::default_io_context()?, commit_info)?
          .unwrap();
        (readers_and_updates, updated_reader)
      };

      assert_ne!(-1, doc);

      let mut number =
        LeafReader::get_numeric_doc_values(&updated_reader, "number")?.expect("numeric dv missing");

      assert_eq!(doc, number.advance(doc)?);
      assert_eq!(1000_i64, number.long_value()?);

      readers_and_updates.release(updated_reader.as_ref(), None)?;
      assert!(!pool.release(
        &readers_and_updates,
        random.random_bool(0.5),
        segment_infos,
        None,
        &field_numbers.lock(),
      )?);
    }
  }

  pool.close(segment_infos)?;
  Ok(())
}
#[test]
fn test_deletes() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;

  let (field_numbers, index_created_version_major) = build_index(directory.clone(), &mut random)?;

  let mut reader = directory_reader::open(directory.clone())?;
  let segment_infos = &mut reader.segment_infos;

  let lock = directory.obtain_lock("writer_lock")?;
  let lock_dir = Arc::new(LockValidatingDirectoryWrapper::new(directory.clone(), lock));

  let pool = ReaderPool::new::<String, DummyComparator>(
    lock_dir,
    directory.clone(),
    segment_infos,
    Arc::new(InfoStreamEnum::default()),
    None,
    LongSupplierImpl,
    None,
    index_created_version_major,
  )?;

  let id = random.random_range(0..10);

  if random.random_bool(0.5) {
    pool.enable_reader_pooling();
  }

  for idx in 0..segment_infos.segments.len() {
    let (read_only_clone, _max_doc, readers_and_updates, mut postings) = {
      let commit_info = segment_infos.info_idx_mut(idx).unwrap();
      let readers_and_updates = pool.get(commit_info.to_meta()?, true, None)?.unwrap();
      let read_only_clone = readers_and_updates
        .get_read_only_clone(&IOContext::default_io_context()?, commit_info)?
        .unwrap();

      let term = Term::from_text("id", id.to_string());
      let postings = read_only_clone.postings(&term)?;
      (
        read_only_clone,
        commit_info.info.max_doc()?,
        readers_and_updates,
        postings,
      )
    };
    let mut expect_update = false;
    let mut doc = -1_i32;
    if let Some(ref mut postings) = postings
      && postings.next_doc()? != NO_MORE_DOCS
    {
      doc = postings.doc_id();
      assert!(readers_and_updates.delete(
        postings.doc_id(),
        segment_infos.info_idx_mut(idx).unwrap(),
        None
      )?);
      expect_update = true;
      assert_eq!(NO_MORE_DOCS, postings.next_doc()?);
    };
    assert!(!pool.any_doc_values_changes()); // deletes are not accounted here
    read_only_clone.close()?;
    let written_to_disk: bool;
    if pool.is_reader_pooling_enabled() {
      written_to_disk = pool.commit(segment_infos, &field_numbers.lock())?;
      assert!(!pool.release(
        &readers_and_updates,
        random.random_bool(0.5),
        segment_infos,
        None,
        &field_numbers.lock(),
      )?);
    } else {
      written_to_disk = pool.release(
        &readers_and_updates,
        random.random_bool(0.5),
        segment_infos,
        None,
        &field_numbers.lock(),
      )?;
    }

    assert!(!pool.any_doc_values_changes());
    assert_eq!(expect_update, written_to_disk);

    let commit_info = segment_infos.info_idx_mut(idx).unwrap().clone();
    if expect_update {
      let v = commit_info.to_meta()?;
      let readers_and_updates = pool.get(v, true, None)?.unwrap();
      let updated_reader = readers_and_updates
        .get_read_only_clone(&IOContext::default_io_context()?, &commit_info)?
        .unwrap();

      assert_ne!(-1, doc);
      assert!(
        !updated_reader
          .get_live_docs()?
          .as_ref()
          .unwrap()
          .get(doc as usize)?
      );
      readers_and_updates.release(updated_reader.as_ref(), None)?;
      assert!(!pool.release(
        &readers_and_updates,
        random.random_bool(0.5),
        segment_infos,
        None,
        &field_numbers.lock(),
      )?);
    }
  }
  pool.close(segment_infos)?;
  Ok(())
}

#[test]
fn test_pass_reader_to_merge_policy_concurrently() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;

  let (field_numbers, index_created_version_major) = build_index(directory.clone(), &mut random)?;

  let mut reader = directory_reader::open(directory.clone())?;
  let max_doc = reader.max_doc()?;
  let num_segments = reader.segment_infos.segments.len();

  let lock = directory.obtain_lock("writer_lock")?;
  let lock_dir = Arc::new(LockValidatingDirectoryWrapper::new(directory.clone(), lock));

  let pool = Arc::new(ReaderPool::new::<String, DummyComparator>(
    lock_dir,
    directory.clone(),
    &reader.segment_infos,
    Arc::new(InfoStreamEnum::default()),
    None,
    LongSupplierImpl,
    None,
    index_created_version_major,
  )?);

  if random.random_bool(0.5) {
    pool.enable_reader_pooling();
  }

  let merge_policy = KeepFullyDeletedSegmentsMergePolicy::default();

  use std::sync::Barrier;
  use std::sync::atomic::{AtomicBool, Ordering};
  use std::thread;

  let is_done = Arc::new(AtomicBool::new(false));
  let latch = Arc::new(Barrier::new(2));

  let pool_bg = pool.clone();
  let is_done_bg = is_done.clone();
  let latch_bg = latch.clone();
  let bg_dir = directory.clone();
  let bg_field_numbers = field_numbers.clone();
  let bg_num_segments = num_segments;

  let mut bg_random = crate::test_framework::core::util::lucene_test_case::random();
  let refresher = thread::spawn(move || -> Result<()> {
    let mut bg_reader = directory_reader::open(bg_dir)?;
    latch_bg.wait();
    while !is_done_bg.load(Ordering::SeqCst) {
      for idx in 0..bg_num_segments {
        let seg_infos = &mut bg_reader.segment_infos;
        let commit_info = seg_infos.info_idx_mut(idx).unwrap();
        let readers_and_updates = pool_bg.get(commit_info.to_meta()?, true, None)?.unwrap();
        let segment_reader = readers_and_updates
          .get_read_only_clone(&IOContext::default_io_context()?, commit_info)?;
        if let Some(ref sr) = segment_reader {
          readers_and_updates.release(sr.as_ref(), None)?;
        }
        pool_bg.release(
          &readers_and_updates,
          bg_random.random_bool(0.5),
          seg_infos,
          None,
          &bg_field_numbers.lock(),
        )?;
      }
    }
    Ok(())
  });

  latch.wait();

  for i in 0..max_doc {
    for idx in 0..num_segments {
      let commit_info = reader.segment_infos.info_idx_mut(idx).unwrap();
      let readers_and_updates = pool.get(commit_info.to_meta()?, true, None)?.unwrap();
      let read_only_clone = readers_and_updates
        .get_read_only_clone(&IOContext::default_io_context()?, commit_info)?
        .unwrap();

      let term = Term::from_text("id", i.to_string());
      let mut postings = read_only_clone.postings(&term)?;

      if let Some(ref mut postings) = postings {
        let mut doc_id = postings.next_doc()?;
        while doc_id != NO_MORE_DOCS {
          readers_and_updates.delete(
            doc_id,
            reader.segment_infos.info_idx_mut(idx).unwrap(),
            None,
          )?;
          assert!(readers_and_updates.keep_fully_deleted_segment(
            &merge_policy,
            reader.segment_infos.info_idx_mut(idx).unwrap(),
          )?);
          doc_id = postings.next_doc()?;
        }
      }

      assert!(readers_and_updates.keep_fully_deleted_segment(
        &merge_policy,
        reader.segment_infos.info_idx_mut(idx).unwrap(),
      )?);

      read_only_clone.close()?;

      pool.release(
        &readers_and_updates,
        random.random_bool(0.5),
        &mut reader.segment_infos,
        None,
        &field_numbers.lock(),
      )?;
    }
  }

  is_done.store(true, Ordering::SeqCst);
  refresher.join().unwrap()?;

  pool.close(&mut reader.segment_infos)?;

  Ok(())
}
#[test]
fn test_get_reader_by_ram() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;

  let (_field_numbers, index_created_version_major) = build_index(directory.clone(), &mut random)?;

  let mut reader = directory_reader::open(directory.clone())?;
  let segment_infos = &mut reader.segment_infos;

  let lock = directory.obtain_lock("writer_lock")?;
  let lock_dir = Arc::new(LockValidatingDirectoryWrapper::new(directory.clone(), lock));

  let pool = ReaderPool::new::<String, DummyComparator>(
    lock_dir,
    directory.clone(),
    segment_infos,
    Arc::new(InfoStreamEnum::default()),
    None,
    LongSupplierImpl,
    None,
    index_created_version_major,
  )?;
  assert_eq!(0, pool.get_readers_by_ram().len());

  for idx in 0..segment_infos.segments.len() {
    let commit_info = segment_infos.info_idx_mut(idx).unwrap();
    let readers_and_updates = pool.get(commit_info.to_meta()?, true, None)?.unwrap();
    let sub_update = NumericDocValuesFieldUpdates::new()?;
    let mut updates = DocValuesFieldUpdates::new(
      commit_info.info.max_doc()?,
      0,
      "number",
      sub_update.sub_type(),
      sub_update,
    )?;
    updates.add_value(0, idx as i64)?;
    updates.finish()?;
    readers_and_updates.add_dv_update(updates)?;
  }

  let readers_by_ram = pool.get_readers_by_ram();
  assert_eq!(segment_infos.segments.len(), readers_by_ram.len());
  let mut previous_ram = i64::MAX;
  for rld in readers_by_ram {
    let ram_bytes_used = rld
      .ram_bytes_used
      .load(std::sync::atomic::Ordering::Relaxed);
    assert!(
      previous_ram >= ram_bytes_used,
      "previous: {} now: {}",
      previous_ram,
      ram_bytes_used
    );
    previous_ram = ram_bytes_used;
    rld.drop_changes();
    pool.drop(rld.get_info_id(), segment_infos)?;
  }

  pool.close(segment_infos)?;
  Ok(())
}

fn build_index<D, R>(directory: Arc<D>, random: &mut R) -> Result<(FieldNumbersLock, i32)>
where
  D: Directory + 'static,
  R: Rng + ?Sized,
{
  let writer = IndexWriter::new(directory, new_index_writer_config(random)?)?;
  for i in 0..10 {
    let mut document = Document::new();
    document.add(StringField::from_string("id", i.to_string(), Yes)?);
    document.add(NumericDocValuesField::new("number", i));

    writer.add_document(document)?;

    if random.random_bool(0.5) {
      writer.flush()?;
    }
  }
  writer.commit()?;
  let field_numbers = writer.global_field_number_map.clone();

  writer.close()?;

  Ok((field_numbers, writer.get_index_major_version_created()))
}
/// A MergePolicy wrapper that always keeps fully deleted segments,
/// and accesses the supplied reader to verify it is valid.
/// Used to test concurrent reader pool access.
#[derive(Default)]
struct KeepFullyDeletedSegmentsMergePolicy {
  in_: NoMergePolicy,
}

impl Display for KeepFullyDeletedSegmentsMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "KeepFullyDeletedSegmentsMergePolicy")
  }
}

impl<D> MergePolicy<D> for KeepFullyDeletedSegmentsMergePolicy
where
  D: Directory,
{
  fn get_base(&self) -> &MergePolicyBase {
    MergePolicy::<D>::get_base(&self.in_)
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    MergePolicy::<D>::get_base_mut(&mut self.in_)
  }

  fn find_merges<MC>(
    &self,
    merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&IWInner<D>>,
    merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
  {
    self
      .in_
      .find_merges(merge_trigger, segment_infos, inner, merge_context)
  }

  fn find_merges_readers<CR>(&self, readers: Vec<CR>) -> Result<Option<MergeSpecification<D, CR>>>
  where
    CR: CodecReader,
  {
    self.in_.find_merges_readers(readers)
  }

  fn find_forced_merges<MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    max_segment_count: usize,
    segments_to_merge: &HashMap<String, Option<bool>>,
    inner: Option<&IWInner<D>>,
    merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
  {
    self.in_.find_forced_merges(
      segment_infos,
      max_segment_count,
      segments_to_merge,
      inner,
      merge_context,
    )
  }

  fn find_forced_deletes_merges<MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&IWInner<D>>,
    merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
  {
    self
      .in_
      .find_forced_deletes_merges(segment_infos, inner, merge_context)
  }

  fn find_full_flush_merges<MC>(
    &self,
    merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&IWInner<D>>,
    merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
  {
    self
      .in_
      .find_full_flush_merges(merge_trigger, segment_infos, inner, merge_context)
  }

  fn use_compound_file<MC>(
    &self,
    infos: &SegmentInfos<D>,
    merged_info: &SegmentCommitInfo<D>,
    merge_context: &MC,
  ) -> Result<bool>
  where
    MC: MergeContext<D>,
  {
    self
      .in_
      .use_compound_file(infos, merged_info, merge_context)
  }

  fn size<MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    MC: MergeContext<D>,
  {
    self.in_.size(info, merge_context)
  }

  fn max_full_flush_merge_size(&self) -> i64 {
    MergePolicy::<D>::max_full_flush_merge_size(&self.in_)
  }

  fn has_merged<MC>(
    &self,
    infos: &SegmentInfos<D>,
    info: &SegmentCommitInfo<D>,
    merge_context: &MC,
  ) -> Result<bool>
  where
    MC: MergeContext<D>,
  {
    self.in_.has_merged(infos, info, merge_context)
  }

  fn keep_fully_deleted_segment<F>(&self, reader_supplier: F) -> Result<bool>
  where
    F: Fn() -> Result<DefaultLeafReader<D>>,
  {
    let reader = reader_supplier()?;
    assert!(reader.max_doc()? > 0); // just try to access the reader
    Ok(true)
  }

  fn num_deletes_to_merge<F>(
    &self,
    info: &SegmentCommitInfo<D>,
    del_count: i32,
    reader_supplier: F,
  ) -> Result<i32>
  where
    F: Fn() -> Result<DefaultLeafReader<D>>,
  {
    self
      .in_
      .num_deletes_to_merge(info, del_count, reader_supplier)
  }
}

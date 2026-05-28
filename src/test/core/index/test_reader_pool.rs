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
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config, random,
};

use crate::core::index::leaf_reader::LeafReader;
use rand::Rng;
use rand::RngExt;
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
  let commit_info = segment_infos.info_idx_mut(idx).unwrap();

  let readers_and_updates = pool.get((&*commit_info).into(), true, None)?.unwrap();

  let same = pool.get((&*commit_info).into(), false, None)?.unwrap();
  assert!(Arc::ptr_eq(&readers_and_updates, &same));
  assert!(pool.drop(commit_info.info.get_id_key())?);

  if random.random_bool(0.5) {
    assert!(!pool.drop(commit_info.info.get_id_key())?);
  }
  assert!(pool.get((&*commit_info).into(), false, None)?.is_none());
  pool.release(
    &readers_and_updates,
    random.random_bool(0.5),
    Some(commit_info),
    &field_numbers.lock(),
  )?;
  pool.close()?;
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
  let commit_info = segment_infos.info_idx_mut(idx).unwrap();

  assert!(!pool.is_reader_pooling_enabled());

  let rau = pool.get((&*commit_info).into(), true, None)?.unwrap();
  pool.release(
    &rau,
    random.random_bool(0.5),
    Some(commit_info),
    &field_numbers.lock(),
  )?;

  assert!(pool.get((&*commit_info).into(), false, None)?.is_none());
  // now start pooling
  pool.enable_reader_pooling();
  assert!(pool.is_reader_pooling_enabled());

  let rau = pool.get((&*commit_info).into(), true, None)?.unwrap();
  pool.release(
    &rau,
    random.random_bool(0.5),
    Some(commit_info),
    &field_numbers.lock(),
  )?;

  let pooled = pool.get((&*commit_info).into(), false, None)?.unwrap();
  let pooled_again = pool.get((&*commit_info).into(), false, None)?.unwrap();
  assert!(Arc::ptr_eq(&pooled, &pooled_again));

  pool.drop(commit_info.info.get_id_key())?;

  // let mut ram_bytes_used = 0_i64;
  // TODO: memory calculation not implement
  // assert_eq!(0, pool.ram_bytes_used());

  for idx in 0..segment_infos.segments.len() {
    let info = segment_infos.info_idx_mut(idx).unwrap();

    let rau = pool.get((&*info).into(), true, None)?.unwrap();
    pool.release(
      &rau,
      random.random_bool(0.5),
      Some(info),
      &field_numbers.lock(),
    )?;
    // TODO: memory calculation not implement
    // assert_eq!(
    //     0,
    //     pool.ram_bytes_used(),
    //     " used: {} actual: {}",
    //     ram_bytes_used,
    //     pool.ram_bytes_used()
    // );

    // ram_bytes_used = pool.ram_bytes_used();

    let a = pool.get((&*info).into(), false, None)?.unwrap();
    let b = pool.get((&*info).into(), false, None)?.unwrap();
    assert!(Arc::ptr_eq(&a, &b));
  }
  // TODO: memory calculation not implement
  // assert_ne!(0, pool.ram_bytes_used());

  pool.drop_all()?;

  for idx in 0..segment_infos.segments.len() {
    let info = segment_infos.info(idx).unwrap();
    assert!(pool.get(info.into(), false, None)?.is_none());
  }

  // TODO: memory calculation not implement
  // assert_eq!(0, pool.ram_bytes_used());

  pool.close()?;
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
      let readers_and_updates = pool.get((&*commit_info).into(), true, None)?.unwrap();
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
        segment_infos.info_idx_mut(idx),
        &field_numbers.lock(),
      )?);
    } else if random.random_bool(0.5) {
      written_to_disk = pool.release(
        &readers_and_updates,
        random.random_bool(0.5),
        segment_infos.info_idx_mut(idx),
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
        segment_infos.info_idx_mut(idx),
        &field_numbers.lock(),
      )?);
    }

    assert!(!pool.any_doc_values_changes());
    assert_eq!(expect_update, written_to_disk);

    let commit_info = segment_infos.info_idx_mut(idx).unwrap();
    if expect_update {
      let readers_and_updates = pool.get((&*commit_info).into(), true, None)?.unwrap();
      let updated_reader = readers_and_updates
        .get_read_only_clone(&IOContext::default_io_context()?, commit_info)?
        .unwrap();

      assert_ne!(-1, doc);

      let mut number = updated_reader
        .get_numeric_doc_values("number")?
        .expect("numeric dv missing");

      assert_eq!(doc, number.advance(doc)?);
      assert_eq!(1000_i64, number.long_value()?);

      readers_and_updates.release(updated_reader.as_ref(), None)?;
      assert!(!pool.release(
        &readers_and_updates,
        random.random_bool(0.5),
        Some(commit_info),
        &field_numbers.lock(),
      )?);
    }
  }

  pool.close()?;
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
      let readers_and_updates = pool.get((&*commit_info).into(), true, None)?.unwrap();
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
      let commit_info = segment_infos.info_idx_mut(idx).unwrap();
      assert!(!pool.release(
        &readers_and_updates,
        random.random_bool(0.5),
        Some(commit_info),
        &field_numbers.lock(),
      )?);
    } else {
      let commit_info = segment_infos.info_idx_mut(idx).unwrap();
      written_to_disk = pool.release(
        &readers_and_updates,
        random.random_bool(0.5),
        Some(commit_info),
        &field_numbers.lock(),
      )?;
    }

    assert!(!pool.any_doc_values_changes());
    assert_eq!(expect_update, written_to_disk);

    let mut commit_info = segment_infos.info_idx_mut(idx).unwrap().clone();
    if expect_update {
      let v = (&commit_info).into();
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
        Some(&mut commit_info),
        &field_numbers.lock(),
      )?);
    }
  }
  pool.close()?;
  Ok(())
}

fn test_pass_reader_to_merge_policy_concurrently() -> Result<()> {
  // TODO
  Ok(())
}
fn test_get_reader_by_ram() -> Result<()> {
  // TODO: memory calculation not implement
  Ok(())
}

fn build_index<D, R>(directory: Arc<D>, random: &mut R) -> Result<(FieldNumbersLock, i32)>
where
  D: Directory,
  R: Rng + ?Sized,
{
  let writer = IndexWriter::new(directory, new_index_writer_config(random))?;
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

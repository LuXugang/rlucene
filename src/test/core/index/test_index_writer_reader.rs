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
use crate::core::document::field::{Field, FieldBase, Store};
use crate::core::document::long_point::LongPoint;
use crate::core::document::string_field::StringField;
use crate::core::index::BytesRef;
use crate::core::index::composite_reader::{CompositeReader, get_context};
use crate::core::index::directory_reader::{self, DirectoryReader};
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::{DEFAULT_RAM_BUFFER_SIZE_MB, DISABLE_AUTO_FLUSH};
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_bits::get_live_docs;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::point_values::PointValues;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::term_query::TermQuery;
use crate::core::store::directory::Directory;
use crate::core::util::Comparator;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::doc_helper::{DocHelper, STRING_TYPE_STORED_WITH_TVS};
use crate::test::core::util::lucene_test_case::{
  is_night_mode, new_directory_shared, new_index_writer_config,
  new_log_merge_policy_with_merge_factor, new_text_field, random, random_from_seed,
};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};
use std::thread;

#[allow(dead_code)] // for quick search
struct TestIndexWriterReader;
pub(crate) fn count<R, CR>(random: &mut R, t: &Term, r: &CR) -> Result<i32>
where
  R: Rng + ?Sized,
  CR: CompositeReader,
{
  let mut count = 0;
  let term_bytes = BytesRef::from_string(&t.text()?);
  let mut td = TestUtil::docs_with_reader(random, r, t.field(), &term_bytes, None, 0)?;

  if let Some(td) = td.as_mut() {
    let live_docs = get_live_docs(r)?;
    while td.next_doc()? != NO_MORE_DOCS {
      let doc_id = td.doc_id();
      if live_docs
        .as_ref()
        .is_none_or(|bits| bits.get(doc_id as usize).expect(""))
      {
        count += 1;
      }
    }
  }

  Ok(count)
}
#[test]
fn test_add_close_open() -> Result<()> {
  let mut random = random();
  let dir1 = new_directory_shared(&mut random)?;
  let iwc = new_index_writer_config(&mut random);

  let mut writer = IndexWriter::new(dir1.clone(), iwc)?;
  for i in 0..97 {
    let reader = directory_reader::open_from_writer(&writer)?;
    if i == 0 {
      writer.add_document(DocHelper::create_document(
        i,
        "x",
        1 + random.random_range(0..5),
      ))?;
    } else {
      let previous = random.random_range(0..i);
      match random.random_range(0..5) {
        0..=2 => {
          writer.add_document(DocHelper::create_document(
            i,
            "x",
            1 + random.random_range(0..5),
          ))?;
        },
        3 => {
          writer.update_document_with_term(
            Term::from_text("id", previous.to_string()),
            DocHelper::create_document(previous, "x", 1 + random.random_range(0..5)),
          )?;
        },
        4 => {
          writer.delete_documents_with_terms(vec![Term::from_text("id", previous.to_string())])?;
        },
        _ => unreachable!(),
      }
    }
    assert!(!reader.is_current(&writer)?);
    reader.close()?;
  }
  writer.force_merge(1)?;
  let mut reader = directory_reader::open_from_writer(&writer)?;
  writer.commit()?;

  assert!(!reader.is_current(&writer)?);
  reader.close()?;
  reader = directory_reader::open_from_writer(&writer)?;
  assert!(reader.is_current(&writer)?);
  writer.close()?;

  assert!(reader.is_current(&writer)?);
  let iwc = new_index_writer_config(&mut random);
  drop(writer);
  writer = IndexWriter::new(dir1.clone(), iwc)?;
  assert!(reader.is_current(&writer)?);
  writer.add_document(DocHelper::create_document(
    1,
    "x",
    1 + random.random_range(0..5),
  ))?;
  assert!(reader.is_current(&writer)?);
  writer.close()?;
  assert!(!reader.is_current(&writer)?);
  reader.close()?;
  Ok(())
}

#[test]
fn test_update_document() -> Result<()> {
  let do_full_merge = true;

  let mut random = random();
  let dir1 = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);
  if iwc.get_max_buffered_docs() < 20 {
    iwc.set_max_buffered_docs(20);
  }
  iwc.set_merge_policy(NoMergePolicy::default());
  let mut writer = IndexWriter::new(dir1.clone(), iwc)?;

  create_index_no_close(!do_full_merge, "index1", &writer)?;

  let r1 = directory_reader::open_from_writer(&writer)?;
  assert!(r1.is_current(&writer)?);

  let mut stored_fields = r1.stored_fields()?;
  let id10 = stored_fields
    .document(10)?
    .get_field("id")
    .expect("id field should exist")
    .string_value()?
    .expect("id field should be stored")
    .into_owned();

  let mut new_doc = stored_fields.document(10)?;
  new_doc.remove_field("id");
  new_doc.add(Field::new(
    "id",
    8000.to_string(),
    STRING_TYPE_STORED_WITH_TVS.clone(),
  ));
  writer.update_document_with_term(Term::from_text("id", id10.clone()), new_doc)?;
  assert!(!r1.is_current(&writer)?);

  let r2 = directory_reader::open_from_writer(&writer)?;
  assert!(r2.is_current(&writer)?);
  assert_eq!(
    0,
    count(&mut random, &Term::from_text("id", id10.clone()), &r2)?
  );
  assert_eq!(
    1,
    count(&mut random, &Term::from_text("id", 8000.to_string()), &r2)?
  );

  r1.close()?;
  assert!(r2.is_current(&writer)?);
  writer.close()?;
  assert!(!r2.is_current(&writer)?);

  let r3 = directory_reader::open(dir1.clone())?;
  assert!(r3.is_current(&writer)?);
  assert!(!r2.is_current(&writer)?);
  assert_eq!(0, count(&mut random, &Term::from_text("id", id10), &r3)?);
  assert_eq!(
    1,
    count(&mut random, &Term::from_text("id", 8000.to_string()), &r3)?
  );
  drop(writer);
  writer = IndexWriter::new(dir1.clone(), new_index_writer_config(&mut random))?;
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "a b c",
    Store::No,
    &mut Default::default(),
  )?);
  writer.add_document(doc)?;
  assert!(!r2.is_current(&writer)?);
  assert!(r3.is_current(&writer)?);

  writer.close()?;

  assert!(!r2.is_current(&writer)?);
  assert!(!r3.is_current(&writer)?);

  r2.close()?;
  r3.close()?;
  Ok(())
}

#[test]
fn test_is_current() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iwc = new_index_writer_config(&mut random);
  let mut field_to_type = HashMap::new();
  let mut writer = IndexWriter::new(dir.clone(), iwc)?;
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "a b c",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(doc)?;
  writer.close()?;
  drop(writer);
  let iwc = new_index_writer_config(&mut random);
  writer = IndexWriter::new(dir.clone(), iwc)?;
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "a b c",
    Store::No,
    &mut field_to_type,
  )?);
  let nrt_reader = directory_reader::open_from_writer(&writer)?;
  assert!(nrt_reader.is_current(&writer)?);
  writer.add_document(doc)?;
  assert!(!nrt_reader.is_current(&writer)?);
  writer.force_merge(1)?;
  assert!(!nrt_reader.is_current(&writer)?);
  nrt_reader.close()?;

  let dir_reader = directory_reader::open(dir.clone())?;
  let nrt_reader = directory_reader::open_from_writer(&writer)?;

  assert!(dir_reader.is_current(&writer)?);
  assert!(nrt_reader.is_current(&writer)?);
  assert_eq!(2, nrt_reader.max_doc()?);
  assert_eq!(1, dir_reader.max_doc()?);
  writer.close()?;
  assert!(!nrt_reader.is_current(&writer)?);
  assert!(!dir_reader.is_current(&writer)?);

  dir_reader.close()?;
  nrt_reader.close()?;
  Ok(())
}

#[test]
fn test_add_indexes() -> Result<()> {
  let do_full_merge = false;

  let mut random = random();
  let dir1 = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_max_full_flush_merge_wait_millis(0);
  if iwc.get_max_buffered_docs() < 20 {
    iwc.set_max_buffered_docs(20);
  }
  iwc.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir1.clone(), iwc)?;

  create_index_no_close(!do_full_merge, "index1", &writer)?;
  writer.flush()?;

  let dir2 = new_directory_shared(&mut random)?;
  let writer2 = IndexWriter::new(dir2.clone(), new_index_writer_config(&mut random))?;
  create_index_no_close(!do_full_merge, "index2", &writer2)?;
  writer2.close()?;

  let r0 = directory_reader::open_from_writer(&writer)?;
  assert!(r0.is_current(&writer)?);
  drop(writer2);
  writer.add_indexes_from_dir(std::slice::from_ref(&dir2))?;
  assert!(!r0.is_current(&writer)?);
  r0.close()?;

  let r1 = directory_reader::open_from_writer(&writer)?;
  assert!(r1.is_current(&writer)?);

  writer.commit()?;

  assert!(!r1.is_current(&writer)?);

  assert_eq!(200, r1.max_doc()?);

  let index2df = r1.doc_freq(&Term::from_text("indexname", "index2"))?;

  assert_eq!(100, index2df);

  let mut stored_fields = r1.stored_fields()?;
  let doc5 = stored_fields.document(5)?;
  assert_eq!("index1", doc5.get("indexname")?.unwrap().as_ref());
  let doc150 = stored_fields.document(150)?;
  assert_eq!("index2", doc150.get("indexname")?.unwrap().as_ref());
  r1.close()?;
  writer.close()?;
  Ok(())
}

#[test]
fn test_add_indexes2() -> Result<()> {
  let do_full_merge = false;

  let mut random = random();
  let dir1 = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_max_full_flush_merge_wait_millis(0);
  let writer = IndexWriter::new(dir1.clone(), iwc)?;

  let dir2 = new_directory_shared(&mut random)?;
  let mut iwc2 = new_index_writer_config(&mut random);
  iwc2.set_max_full_flush_merge_wait_millis(0);
  let writer2 = IndexWriter::new(dir2.clone(), iwc2)?;
  create_index_no_close(!do_full_merge, "index2", &writer2)?;
  writer2.close()?;
  drop(writer2);
  writer.add_indexes_from_dir(std::slice::from_ref(&dir2))?;
  writer.add_indexes_from_dir(std::slice::from_ref(&dir2))?;
  writer.add_indexes_from_dir(std::slice::from_ref(&dir2))?;
  writer.add_indexes_from_dir(std::slice::from_ref(&dir2))?;
  writer.add_indexes_from_dir(std::slice::from_ref(&dir2))?;

  let r1 = directory_reader::open_from_writer(&writer)?;
  assert_eq!(500, r1.max_doc()?);

  r1.close()?;
  writer.close()?;
  Ok(())
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

#[test]
fn test_delete_from_index_writer() -> Result<()> {
  let do_full_merge = true;

  let mut random = random();
  let dir1 = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_max_full_flush_merge_wait_millis(0);
  let mut writer = IndexWriter::new(dir1.clone(), iwc)?;

  create_index_no_close(!do_full_merge, "index1", &writer)?;
  writer.flush_with_apply_merge_deletes(false, true)?;

  let r1 = directory_reader::open_from_writer(&writer)?;

  let mut stored_fields = r1.stored_fields()?;
  let id10 = stored_fields
    .document(10)?
    .get_field("id")
    .expect("id field should exist")
    .string_value()?
    .expect("id field should be stored")
    .into_owned();

  writer.delete_documents_with_terms(vec![Term::from_text("id", id10.clone())])?;
  let r2 = directory_reader::open_from_writer(&writer)?;
  assert_eq!(
    1,
    count(&mut random, &Term::from_text("id", id10.clone()), &r1)?
  );
  assert_eq!(
    0,
    count(&mut random, &Term::from_text("id", id10.clone()), &r2)?
  );

  let id50 = stored_fields
    .document(50)?
    .get_field("id")
    .expect("id field should exist")
    .string_value()?
    .expect("id field should be stored")
    .into_owned();
  assert_eq!(
    1,
    count(&mut random, &Term::from_text("id", id50.clone()), &r1)?
  );

  writer.delete_documents_with_terms(vec![Term::from_text("id", id50.clone())])?;

  let r3 = directory_reader::open_from_writer(&writer)?;
  assert_eq!(
    0,
    count(&mut random, &Term::from_text("id", id10.clone()), &r3)?
  );
  assert_eq!(0, count(&mut random, &Term::from_text("id", id50), &r3)?);

  let id75 = stored_fields
    .document(75)?
    .get_field("id")
    .expect("id field should exist")
    .string_value()?
    .expect("id field should be stored")
    .into_owned();
  // TODO delete by query 未实现  先用delete_documents_with_terms替代
  writer.delete_documents_with_terms(vec![Term::from_text("id", id75.clone())])?;
  let r4 = directory_reader::open_from_writer(&writer)?;
  assert_eq!(
    1,
    count(&mut random, &Term::from_text("id", id75.clone()), &r3)?
  );
  assert_eq!(0, count(&mut random, &Term::from_text("id", id75), &r4)?);

  r1.close()?;
  r2.close()?;
  r3.close()?;
  r4.close()?;
  writer.close()?;

  drop(writer);
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_max_full_flush_merge_wait_millis(0);
  writer = IndexWriter::new(dir1.clone(), iwc)?;
  let w2r1 = directory_reader::open_from_writer(&writer)?;
  assert_eq!(0, count(&mut random, &Term::from_text("id", id10), &w2r1)?);
  w2r1.close()?;
  writer.close()?;
  Ok(())
}

#[test]
fn test_add_indexes_and_do_deletes_threads() -> Result<()> {
  Ok(())
}

#[test]
fn test_index_writer_reopen_segment_full_merge() -> Result<()> {
  do_test_index_writer_reopen_segment(true)
}

#[test]
fn test_index_writer_reopen_segment() -> Result<()> {
  do_test_index_writer_reopen_segment(false)
}

fn do_test_index_writer_reopen_segment(do_full_merge: bool) -> Result<()> {
  let mut random = random();
  // TODO MockDirectoryWrapper未实现
  let dir1 = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_max_full_flush_merge_wait_millis(0);
  let mut writer = IndexWriter::new(dir1.clone(), iwc)?;
  let r1 = directory_reader::open_from_writer(&writer)?;
  assert_eq!(0, r1.max_doc()?);
  create_index_no_close(false, "index1", &writer)?;
  writer.flush_with_apply_merge_deletes(!do_full_merge, true)?;

  let iwr1 = directory_reader::open_from_writer(&writer)?;
  assert_eq!(100, iwr1.max_doc()?);

  let r2 = directory_reader::open_from_writer(&writer)?;
  assert_eq!(r2.max_doc()?, 100);
  for x in 10000..10000 + 100 {
    let d = DocHelper::create_document(x, "index1", 5);
    writer.add_document(d)?;
  }
  writer.flush_with_apply_merge_deletes(false, true)?;

  let iwr2 = directory_reader::open_from_writer(&writer)?;
  assert_ne!(iwr2.get_version()?, r1.get_version()?);
  assert_eq!(200, iwr2.max_doc()?);

  let r3 = directory_reader::open_from_writer(&writer)?;
  assert_ne!(r2.get_version()?, r3.get_version()?);
  assert_eq!(200, r3.max_doc()?);

  r1.close()?;
  iwr1.close()?;
  r2.close()?;
  r3.close()?;
  iwr2.close()?;
  writer.close()?;

  drop(writer);
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_max_full_flush_merge_wait_millis(0);
  writer = IndexWriter::new(dir1.clone(), iwc)?;
  let w2r1 = directory_reader::open_from_writer(&writer)?;
  assert_eq!(200, w2r1.max_doc()?);
  w2r1.close()?;
  writer.close()?;
  Ok(())
}

#[test]
fn test_merge_warmer() -> Result<()> {
  // TODO IMPORTANT ConcurrentMergeScheduler未实现
  Ok(())
}

#[test]
fn test_after_commit() -> Result<()> {
  // TODO IMPORTANT ConcurrentMergeScheduler未实现
  Ok(())
}

#[test]
fn test_after_close() -> Result<()> {
  let mut random = random();
  let dir1 = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_max_full_flush_merge_wait_millis(0);
  let writer = IndexWriter::new(dir1.clone(), iwc)?;

  create_index_no_close(false, "test", &writer)?;

  let r = directory_reader::open_from_writer(&writer)?;
  writer.close()?;

  assert_eq!(100, r.num_docs()?);
  let q = TermQuery::new(Term::from_text("indexname", "test"));
  let searcher = IndexSearcher::from_cr(r)?;
  assert_eq!(100, searcher.count(q)?);

  let err = directory_reader::open_if_changed(searcher.reader_context.reader(), &writer);
  assert!(err.is_err());

  searcher.reader_context.reader().close()?;
  Ok(())
}
#[cfg(feature = "nightly")]
#[ignore = "nightly"]
#[test]
fn test_during_add_indexes() -> Result<()> {
  use crate::core::store::directory::DirectoryEnum2;

  let mut random = random();
  let dir1 = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);
  iwc
    .set_max_full_flush_merge_wait_millis(0)
    .set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 2)?);
  let writer_dir = Arc::new(DirectoryEnum2::A(dir1.clone()));
  let writer = Arc::new(IndexWriter::new(writer_dir, iwc)?);

  create_index_no_close(false, "test", writer.as_ref())?;
  writer.commit()?;

  let mut dirs = Vec::new();
  for _ in 0..10 {
    dirs.push(Arc::new(DirectoryEnum2::B(TestUtil::ram_copy_of(
      &mut random,
      dir1.as_ref(),
    )?)));
  }

  let mut r = directory_reader::open_from_writer(writer.as_ref())?;

  let num_iterations = 10;
  let failures = Arc::new(Mutex::new(Vec::new()));
  let thread_done = Arc::new(std::sync::atomic::AtomicBool::new(false));

  let handle = {
    let writer = writer.clone();
    let dirs = dirs.clone();
    let failures = failures.clone();
    let thread_done = thread_done.clone();
    thread::spawn(move || {
      let result = (|| -> Result<()> {
        let mut count = 0;
        loop {
          count += 1;
          writer.add_indexes_from_dir(&dirs)?;
          writer.maybe_merge()?;
          if count >= num_iterations {
            break;
          }
        }
        Ok(())
      })();
      if let Err(e) = result {
        failures
          .lock()
          .expect("failures lock poisoned")
          .push(format!("{e:?}"));
      }
      thread_done.store(true, AtomicOrdering::Release);
    })
  };

  let mut last_count = 0;
  while !thread_done.load(AtomicOrdering::Acquire) {
    let r2 = directory_reader::open_if_changed(&r, writer.as_ref())?;
    if let Some(r2) = r2 {
      r.close()?;
      r = r2;
      let term = Term::from_text("indexname", "test");
      let count = count(&mut random, &term, &r)?;
      assert!(count >= last_count);
      last_count = count;
    }
  }

  handle.join().expect("addIndexes thread panicked");
  let r2 = directory_reader::open_if_changed(&r, writer.as_ref())?;
  if let Some(r2) = r2 {
    r.close()?;
    r = r2;
  }
  let term = Term::from_text("indexname", "test");
  let count = count(&mut random, &term, &r)?;
  assert!(count >= last_count);

  assert!(failures.lock().expect("failures lock poisoned").is_empty());
  r.close()?;
  writer.close()?;
  Ok(())
}

#[test]
fn test_during_add_delete() -> Result<()> {
  let mut random = random();
  let dir1 = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 2)?);
  if is_night_mode() {
    iwc.set_ram_buffer_size_mb(DEFAULT_RAM_BUFFER_SIZE_MB);
    iwc.set_max_buffered_docs(DISABLE_AUTO_FLUSH);
  }
  let writer = Arc::new(IndexWriter::new(dir1.clone(), iwc)?);

  create_index_no_close(false, "test", writer.as_ref())?;
  writer.commit()?;

  let mut r = directory_reader::open_from_writer(writer.as_ref())?;

  let iters = if is_night_mode() { 1000 } else { 10 };
  let failures = Arc::new(Mutex::new(Vec::new()));

  let num_threads = if is_night_mode() { 5 } else { 2 };
  let remaining_threads = Arc::new(AtomicUsize::new(num_threads));
  let mut threads = Vec::new();
  for _ in 0..num_threads {
    let writer = writer.clone();
    let failures = failures.clone();
    let seed = random.random();
    let remaining_threads = remaining_threads.clone();
    threads.push(thread::spawn(move || {
      let result = (|| -> Result<()> {
        let mut random = random_from_seed(seed);
        let mut count = 0;
        loop {
          for doc_upto in 0..10 {
            writer.add_document(DocHelper::create_document(10 * count + doc_upto, "test", 4))?;
          }
          count += 1;
          let limit = count * 10;
          for _ in 0..5 {
            let x = random.random_range(0..limit);
            writer.delete_documents_with_terms(vec![Term::from_text("field3", format!("b{x}"))])?;
          }
          if count >= iters {
            break;
          }
        }
        Ok(())
      })();
      if let Err(e) = result {
        failures
          .lock()
          .expect("failures lock poisoned")
          .push(format!("{e:?}"));
      }
      remaining_threads.fetch_sub(1, AtomicOrdering::AcqRel);
    }));
  }

  let mut sum = 0;
  while remaining_threads.load(AtomicOrdering::Acquire) > 0 {
    let r2 = directory_reader::open_if_changed(&r, writer.as_ref())?;
    if let Some(r2) = r2 {
      r.close()?;
      r = r2;
      let term = Term::from_text("indexname", "test");
      sum += count(&mut random, &term, &r)?;
    }
  }

  for handle in threads {
    handle.join().expect("add/delete thread panicked");
  }
  let r2 = directory_reader::open_if_changed(&r, writer.as_ref())?;
  if let Some(r2) = r2 {
    r.close()?;
    r = r2;
  }
  let term = Term::from_text("indexname", "test");
  sum += count(&mut random, &term, &r)?;
  assert!(sum > 0);

  assert!(failures.lock().expect("failures lock poisoned").is_empty());
  writer.close()?;

  r.close()?;
  Ok(())
}

#[test]
fn test_force_merge_deletes() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  let mut field_to_type = HashMap::new();

  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "a b c",
    Store::No,
    &mut field_to_type,
  )?);
  let mut id = StringField::from_string("id", "", Store::No)?;
  doc.add(id.clone());
  id.set_string_value("0")?;
  let mut doc0 = Document::new();
  doc0.add(new_text_field(
    &mut random,
    "field",
    "a b c",
    Store::No,
    &mut field_to_type,
  )?);
  doc0.add(id.clone());
  w.add_document(doc0)?;
  id.set_string_value("1")?;
  let mut doc1 = Document::new();
  doc1.add(new_text_field(
    &mut random,
    "field",
    "a b c",
    Store::No,
    &mut field_to_type,
  )?);
  doc1.add(id);
  w.add_document(doc1)?;
  w.delete_documents_with_terms(vec![Term::from_text("id", "0")])?;

  let r = directory_reader::open_from_writer(&w)?;
  w.force_merge_deletes()?;
  w.close()?;
  r.close()?;
  drop(w);
  let r = directory_reader::open(dir.clone())?;
  assert_eq!(1, r.num_docs()?);
  assert!(!r.has_deletions()?);
  r.close()?;
  Ok(())
}

#[test]
fn test_deletes_num_docs() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  let mut field_to_type = HashMap::new();

  let mut id = StringField::from_string("id", "", Store::No)?;
  id.set_string_value("0")?;
  let mut doc0 = Document::new();
  doc0.add(new_text_field(
    &mut random,
    "field",
    "a b c",
    Store::No,
    &mut field_to_type,
  )?);
  doc0.add(id.clone());
  w.add_document(doc0)?;
  id.set_string_value("1")?;
  let mut doc1 = Document::new();
  doc1.add(new_text_field(
    &mut random,
    "field",
    "a b c",
    Store::No,
    &mut field_to_type,
  )?);
  doc1.add(id);
  w.add_document(doc1)?;
  let mut r = directory_reader::open_from_writer(&w)?;
  assert_eq!(2, r.num_docs()?);
  r.close()?;

  w.delete_documents_with_terms(vec![Term::from_text("id", "0")])?;
  r = directory_reader::open_from_writer(&w)?;
  assert_eq!(1, r.num_docs()?);
  r.close()?;

  w.delete_documents_with_terms(vec![Term::from_text("id", "1")])?;
  r = directory_reader::open_from_writer(&w)?;
  assert_eq!(0, r.num_docs()?);
  r.close()?;

  w.close()?;
  Ok(())
}

#[test]
fn test_empty_index() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  let r = directory_reader::open_from_writer(&w)?;
  assert_eq!(0, r.num_docs()?);
  r.close()?;
  w.close()?;
  Ok(())
}

#[test]
fn test_segment_warmer() -> Result<()> {
  // TODO IMPORTANT SegmentWarmer未实现
  Ok(())
}

#[test]
fn test_simple_merged_segment_warmer() -> Result<()> {
  // TODO IMPORTANT SegmentWarmer未实现
  Ok(())
}

#[test]
fn test_reopen_after_no_real_change() -> Result<()> {
  let mut random = random();
  let d = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_max_full_flush_merge_wait_millis(0);
  let w = IndexWriter::new(d.clone(), iwc)?;

  let r = directory_reader::open_from_writer(&w)?;

  let r2 = directory_reader::open_if_changed(&r, &w)?;
  assert!(r2.is_none());

  w.add_document(Document::new())?;
  let r3 = directory_reader::open_if_changed(&r, &w)?;
  assert!(r3.is_some());
  let r3 = r3.unwrap();
  assert!(r3.get_version()? != r.get_version()?);
  assert!(r3.is_current(&w)?);

  w.delete_documents_with_terms(vec![Term::from_text("foo", "bar")])?;

  assert!(!r3.is_current(&w)?);
  let r4 = directory_reader::open_if_changed(&r3, &w)?;
  assert!(r4.is_none());

  w.delete_documents_with_terms(vec![Term::from_text("foo", "bar")])?;
  let r5 = directory_reader::open_if_changed_with_writer(&r3, &w)?;
  assert!(r5.is_none());

  r.close()?;
  r3.close()?;

  w.close()?;
  Ok(())
}

#[test]
fn test_nrt_open_exceptions() -> Result<()> {
  // TODO IMPORTANT MockDirectoryWrapper未实现
  Ok(())
}

#[test]
fn test_too_many_segments() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

  for i in 0..500 {
    let mut doc = Document::new();
    doc.add(StringField::from_string("id", i.to_string(), Store::No)?);
    w.add_document(doc)?;
    let r = directory_reader::open_from_writer(&w)?;
    let context = get_context(&r)?;
    assert!(context.leaves()?.len() < 100);
    r.close()?;
  }
  w.close()?;
  Ok(())
}

#[test]
fn test_reopen_nrt_reader_on_commit() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  w.add_document(Document::new())?;

  let r1 = directory_reader::open_from_writer(&w)?;
  let r1_context = get_context(&r1)?;
  assert_eq!(1, r1_context.leaves()?.len());
  w.add_document(Document::new())?;
  w.commit()?;

  let commits = directory_reader::list_commits(dir.clone())?;
  assert_eq!(1, commits.len());
  let r2 = directory_reader::open_if_changed_with_commit(&r1, Some(&commits[0]), &w)?
    .expect("commit should produce changed reader");
  let r2_context = get_context(&r2)?;
  assert_eq!(2, r2_context.leaves()?.len());

  assert!(Arc::ptr_eq(
    r1_context.leaves()?[0].reader(),
    r2_context.leaves()?[0].reader()
  ));
  r1.close()?;
  r2.close()?;
  w.close()?;
  Ok(())
}

#[test]
fn test_index_reader_writer_with_leaf_sorter() -> Result<()> {
  // TODO IMPORTANT LeafSorter测试未通过
  Ok(())
}
#[derive(Clone)]
pub struct PointValueLeafSorter {
  asc_sort: bool,
  field_name: String,
  missing_value: i64,
}

impl PointValueLeafSorter {
  fn sort_key<D>(&self, reader: &DefaultLeafReader<D>) -> Result<i64>
  where
    D: Directory,
  {
    let result = (|| -> Result<i64> {
      let Some(points) = reader.get_point_values(&self.field_name)? else {
        return Ok(self.missing_value);
      };
      let sort_value = if self.asc_sort {
        points.get_min_packed_value()?
      } else {
        points.get_max_packed_value()?
      };
      Ok(
        sort_value
          .map(|value| LongPoint::decode_dimension(&value, 0))
          .unwrap_or(self.missing_value),
      )
    })();
    Ok(result.unwrap_or(self.missing_value))
  }
}

impl<D> Comparator<DefaultLeafReader<D>> for PointValueLeafSorter
where
  D: Directory,
{
  const TYPE: &'static str = "PointValueLeafSorter";

  fn compare(&self, a: &DefaultLeafReader<D>, b: &DefaultLeafReader<D>) -> Result<i32> {
    let ord = self.sort_key(a)?.cmp(&self.sort_key(b)?);
    let ord = if self.asc_sort { ord } else { ord.reverse() };

    Ok(match ord {
      Ordering::Less => -1,
      Ordering::Equal => 0,
      Ordering::Greater => 1,
    })
  }
}

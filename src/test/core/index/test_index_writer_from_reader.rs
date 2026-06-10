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
use crate::core::index::directory_reader::{self, DirectoryReader};
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::{IndexCommitWrapper, IndexWriter};
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::no_deletion_policy::NoDeletionPolicy;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_directory_shared, new_index_writer_config, new_string_field, random,
};
use rand::RngExt;
use std::collections::{HashMap, HashSet};

#[allow(dead_code)] // for quick search
struct TestIndexWriterFromReader;

#[test]
fn test_right_after_commit() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  w.add_document(Document::new())?;
  w.commit()?;

  let r = directory_reader::open_from_writer(&w)?;
  assert_eq!(1, r.max_doc()?);
  w.close()?;

  let commit = r.get_index_commit()?;
  let w2 = IndexWriter::with_index_commit(
    dir.clone(),
    new_index_writer_config(&mut random),
    IndexCommitWrapper::new(Some(commit), Some(r), Some(w))?,
  )?;

  assert_eq!(1, w2.get_doc_stats()?.max_doc);
  w2.add_document(Document::new())?;
  assert_eq!(2, w2.get_doc_stats()?.max_doc);
  w2.close()?;

  let r2 = directory_reader::open(dir.clone())?;
  assert_eq!(2, r2.max_doc()?);
  r2.close()?;
  Ok(())
}

#[test]
fn test_from_non_nrt_reader() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  w.add_document(Document::new())?;
  w.close()?;
  drop(w);

  let r = directory_reader::open(dir.clone())?;
  assert_eq!(1, r.max_doc()?);
  let commit = r.get_index_commit()?;

  let w2 = IndexWriter::with_index_commit(
    dir.clone(),
    new_index_writer_config(&mut random),
    IndexCommitWrapper::new(Some(commit), Some(r), None)?,
  )?;

  assert_eq!(1, w2.get_doc_stats()?.max_doc);
  w2.add_document(Document::new())?;
  assert_eq!(2, w2.get_doc_stats()?.max_doc);
  w2.close()?;

  let r2 = directory_reader::open(dir.clone())?;
  assert_eq!(2, r2.max_doc()?);
  r2.close()?;
  Ok(())
}

#[test]
fn test_with_no_first_commit() -> Result<()> {
  Ok(())
}

#[test]
fn test_after_commit_then_index() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  w.add_document(Document::new())?;
  w.commit()?;
  w.add_document(Document::new())?;

  let r = directory_reader::open_from_writer(&w)?;
  assert_eq!(2, r.max_doc()?);
  w.close()?;

  let commit = r.get_index_commit()?;
  let result = IndexWriter::with_index_commit(
    dir.clone(),
    new_index_writer_config(&mut random),
    IndexCommitWrapper::new(Some(commit), Some(r), Some(w))?,
  );
  match result {
    Err(err) => assert!(
      err
        .to_string()
        .contains("the provided reader is stale: its prior commit file")
    ),
    Ok(_) => panic!("expected stale reader error"),
  }
  Ok(())
}

#[test]
fn test_nrt_rollback() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  w.add_document(Document::new())?;
  w.commit()?;

  let r = directory_reader::open_from_writer(&w)?;
  assert_eq!(1, r.max_doc()?);

  w.add_document(Document::new())?;
  assert_eq!(2, w.get_doc_stats()?.max_doc);
  w.close()?;

  let commit = r.get_index_commit()?;
  let result = IndexWriter::with_index_commit(
    dir.clone(),
    new_index_writer_config(&mut random),
    IndexCommitWrapper::new(Some(commit), Some(r), Some(w))?,
  );
  match result {
    Err(err) => assert!(
      err
        .to_string()
        .contains("the provided reader is stale: its prior commit file")
    ),
    Ok(_) => panic!("expected stale reader error"),
  }
  Ok(())
}

#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let num_ops = at_least(&mut random, 100);

  let mut w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

  w.commit()?;

  let mut r = directory_reader::open_from_writer(&w)?;
  let mut nrt_reader_num_docs = 0;
  let mut writer_num_docs = 0;

  let mut commit_after_nrt = false;

  let mut live_ids = HashSet::new();
  let mut nrt_live_ids = HashSet::new();
  let mut field_types = HashMap::<String, FieldType>::new();

  for op in 0..num_ops {
    assert_eq!(nrt_reader_num_docs, r.num_docs()?);
    let x = random.random_range(0..5);

    match x {
      0 => {
        let mut doc = Document::new();
        doc.add(new_string_field(
          &mut random,
          "id",
          op.to_string(),
          Store::No,
          &mut field_types,
        )?);
        w.add_document(doc)?;
        live_ids.insert(op);
        writer_num_docs += 1;
      },
      1 => {
        if !live_ids.is_empty() {
          let id = random.random_range(0..op);
          w.delete_documents_with_terms(vec![Term::from_text("id", id.to_string())])?;
          if live_ids.remove(&id) {
            writer_num_docs -= 1;
          }
        }
      },
      2 => {
        if let Some(r2) = directory_reader::open_if_changed(&r, &w)? {
          r.close()?;
          r = r2;
          nrt_reader_num_docs = writer_num_docs;
          nrt_live_ids = live_ids.clone();
        } else {
          assert_eq!(nrt_reader_num_docs, r.num_docs()?);
        }
        commit_after_nrt = false;
      },
      3 => {
        if !commit_after_nrt {
          if random.random_bool(0.5) {
            w.close()?;
            drop(w);
            r.close()?;
            r = directory_reader::open(dir.clone())?;
            assert_eq!(writer_num_docs, r.num_docs()?);
            nrt_reader_num_docs = writer_num_docs;
            nrt_live_ids = live_ids.clone();
            let commit = r.get_index_commit()?;
            w = IndexWriter::with_index_commit(
              dir.clone(),
              new_index_writer_config(&mut random),
              IndexCommitWrapper::new(Some(commit), Some(r), None)?,
            )?;
          } else {
            w.rollback()?;
            let commit = r.get_index_commit()?;
            w = IndexWriter::with_index_commit(
              dir.clone(),
              new_index_writer_config(&mut random),
              IndexCommitWrapper::new(Some(commit), Some(r), Some(w))?,
            )?;
          }
          writer_num_docs = nrt_reader_num_docs;
          live_ids = nrt_live_ids.clone();
          r = directory_reader::open_from_writer(&w)?;
        }
      },
      4 => {
        w.commit()?;
        commit_after_nrt = true;
      },
      _ => unreachable!(),
    }
  }

  w.close()?;
  r.close()?;
  Ok(())
}

#[test]
fn test_consistent_field_numbers() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  let mut field_types = HashMap::<String, FieldType>::new();

  w.commit()?;

  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "f0",
    "foo",
    Store::No,
    &mut field_types,
  )?);
  w.add_document(doc)?;

  let r = directory_reader::open_from_writer(&w)?;
  assert_eq!(1, r.max_doc()?);

  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "f1",
    "foo",
    Store::No,
    &mut field_types,
  )?);
  w.add_document(doc)?;

  let r2 = directory_reader::open_if_changed(&r, &w)?.expect("reader should change");
  r.close()?;
  assert_eq!(2, r2.max_doc()?);
  w.rollback()?;

  let commit = r2.get_index_commit()?;
  let w2 = IndexWriter::with_index_commit(
    dir.clone(),
    new_index_writer_config(&mut random),
    IndexCommitWrapper::new(Some(commit), Some(r2), Some(w))?,
  )?;

  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "f1",
    "foo",
    Store::No,
    &mut field_types,
  )?);
  doc.add(new_string_field(
    &mut random,
    "f0",
    "foo",
    Store::No,
    &mut field_types,
  )?);
  w2.add_document(doc)?;
  w2.close()?;
  Ok(())
}

#[test]
fn test_invalid_open_mode() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  w.add_document(Document::new())?;
  w.commit()?;

  let r = directory_reader::open_from_writer(&w)?;
  assert_eq!(1, r.max_doc()?);
  w.close()?;

  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_open_mode(OpenMode::Create);
  let commit = r.get_index_commit()?;
  let result = IndexWriter::with_index_commit(
    dir.clone(),
    iwc,
    IndexCommitWrapper::new(Some(commit), Some(r), Some(w))?,
  );
  match result {
    Err(err) => assert!(err.to_string().contains("OpenMode.CREATE")),
    Ok(_) => panic!("expected invalid open mode error"),
  }
  Ok(())
}
#[test]
fn test_on_closed_reader() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  w.add_document(Document::new())?;
  w.commit()?;

  let r = directory_reader::open_from_writer(&w)?;
  assert_eq!(1, r.max_doc()?);
  let commit = r.get_index_commit()?;
  r.close()?;
  w.close()?;

  let result = IndexWriter::with_index_commit(
    dir.clone(),
    new_index_writer_config(&mut random),
    IndexCommitWrapper::new(Some(commit), Some(r), Some(w))?,
  );
  match result {
    Err(LuceneError::AlreadyClosed(_)) => {},
    Err(err) => panic!("expected AlreadyClosed error, got {err}"),
    Ok(_) => panic!("expected AlreadyClosed error"),
  }
  Ok(())
}

#[test]
fn test_stale_nrt_reader() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  w.add_document(Document::new())?;
  w.commit()?;

  let r = directory_reader::open_from_writer(&w)?;
  assert_eq!(1, r.max_doc()?);
  w.add_document(Document::new())?;

  let r2 = directory_reader::open_if_changed(&r, &w)?.expect("reader should change");
  r2.close()?;
  w.rollback()?;

  let commit = r.get_index_commit()?;
  let w = IndexWriter::with_index_commit(
    dir.clone(),
    new_index_writer_config(&mut random),
    IndexCommitWrapper::new(Some(commit), Some(r), Some(w))?,
  )?;
  assert_eq!(1, w.get_doc_stats()?.num_docs);

  let r3 = directory_reader::open_from_writer(&w)?;
  assert_eq!(1, r3.num_docs()?);

  w.add_document(Document::new())?;
  let r4 = directory_reader::open_if_changed(&r3, &w)?.expect("reader should change");
  r3.close()?;
  assert_eq!(2, r4.num_docs()?);
  r4.close()?;
  w.close()?;
  Ok(())
}

#[test]
fn test_after_rollback() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  w.add_document(Document::new())?;
  w.commit()?;
  w.add_document(Document::new())?;

  let r = directory_reader::open_from_writer(&w)?;
  assert_eq!(2, r.max_doc()?);
  w.rollback()?;

  let commit = r.get_index_commit()?;
  let w = IndexWriter::with_index_commit(
    dir.clone(),
    new_index_writer_config(&mut random),
    IndexCommitWrapper::new(Some(commit), Some(r), Some(w))?,
  )?;
  assert_eq!(2, w.get_doc_stats()?.num_docs);
  w.close()?;

  let r2 = directory_reader::open(dir.clone())?;
  assert_eq!(2, r2.num_docs()?);
  r2.close()?;
  Ok(())
}

#[test]
fn test_after_commit_then_index_keep_commits() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);

  iwc.set_index_deletion_policy(NoDeletionPolicy);

  let w = IndexWriter::new(dir.clone(), iwc)?;
  w.add_document(Document::new())?;
  w.commit()?;
  w.add_document(Document::new())?;

  let r = directory_reader::open_from_writer(&w)?;
  assert_eq!(2, r.max_doc()?);
  w.add_document(Document::new())?;

  let r2 = directory_reader::open_from_writer(&w)?;
  assert_eq!(3, r2.max_doc()?);
  r2.close()?;
  w.close()?;

  let commit = r.get_index_commit()?;
  let w2 = IndexWriter::with_index_commit(
    dir.clone(),
    new_index_writer_config(&mut random),
    IndexCommitWrapper::new(Some(commit), Some(r), Some(w))?,
  )?;
  assert_eq!(2, w2.get_doc_stats()?.max_doc);
  w2.close()?;
  Ok(())
}

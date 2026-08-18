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
use crate::core::index::directory_reader::DirectoryReader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::term::Term;
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  is_light_mode, new_directory_shared, new_text_field, random,
};
use rand_chacha::rand_core::Rng;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

#[allow(dead_code)] // for quick search
struct TestIsCurrent;

static LIGHT_DIR: LazyLock<Arc<DirEnum>> = LazyLock::new(|| {
  let mut random = random();
  let (dir, writer) = build_set_up(&mut random).expect("failed to initialize TestIsCurrent");
  writer
    .close(&mut random)
    .expect("failed to close TestIsCurrent writer");
  dir
});
static LIGHT_LOCK: Mutex<()> = Mutex::new(());

fn set_up<R>(
  random: &mut R,
) -> Result<(RandomIndexWriter<DirEnum>, Option<MutexGuard<'static, ()>>)>
where
  R: Rng + ?Sized,
{
  if is_light_mode() {
    let guard = LIGHT_LOCK
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    let directory = LIGHT_DIR.clone();
    let writer = RandomIndexWriter::new(random, directory)?;

    writer.w.delete_all()?;
    let mut field_types = HashMap::<String, FieldType>::new();
    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      "UUID",
      "1",
      Store::Yes,
      &mut field_types,
    )?);
    writer.add_document(random, doc)?;
    writer.commit(random)?;
    return Ok((writer, Some(guard)));
  }

  Ok((build_set_up(random)?.1, None))
}

fn build_set_up<R>(random: &mut R) -> Result<(Arc<DirEnum>, RandomIndexWriter<DirEnum>)>
where
  R: Rng + ?Sized,
{
  // initialize directory
  let directory = new_directory_shared(random)?;
  let writer = RandomIndexWriter::new(random, directory.clone())?;

  // write document
  let mut field_types = HashMap::<String, FieldType>::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "UUID",
    "1",
    Store::Yes,
    &mut field_types,
  )?);
  writer.add_document(random, doc)?;
  writer.commit(random)?;

  Ok((directory, writer))
}

/** Failing testcase showing the trouble */
#[test]
fn test_delete_by_term_is_current() -> Result<()> {
  let mut random = random();
  let (writer, _guard) = set_up(&mut random)?;

  // get reader
  let reader = writer.get_reader(&mut random)?;

  // assert index has a document and reader is up2date
  assert_eq!(1, writer.get_doc_stats()?.num_docs);
  assert!(reader.is_current()?);

  // remove document
  let id_term = Term::from_text("UUID", "1");
  writer.delete_documents_with_terms(&mut random, vec![id_term])?;
  writer.commit(&mut random)?;

  // assert document has been deleted (index changed), reader is stale
  assert_eq!(0, writer.get_doc_stats()?.num_docs);
  assert!(!reader.is_current()?);

  reader.close()?;
  writer.close(&mut random)?;
  Ok(())
}

/** Testcase for example to show that writer.deleteAll() is working as expected */
#[test]
fn test_delete_all_is_current() -> Result<()> {
  let mut random = random();
  let (writer, _guard) = set_up(&mut random)?;

  // get reader
  let reader = writer.get_reader(&mut random)?;

  // assert index has a document and reader is up2date
  assert_eq!(1, writer.get_doc_stats()?.num_docs);
  assert!(reader.is_current()?);

  // remove all documents
  writer.w.delete_all()?;
  writer.commit(&mut random)?;

  // assert document has been deleted (index changed), reader is stale
  assert_eq!(0, writer.get_doc_stats()?.num_docs);
  assert!(!reader.is_current()?);

  reader.close()?;
  writer.close(&mut random)?;
  Ok(())
}

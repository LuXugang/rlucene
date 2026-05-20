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
use crate::core::document::field::{Field, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::{TextField, text_field_type};
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config, new_index_writer_config_with_analyzer,
  new_log_merge_policy, random,
};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestDirectoryReaderReopen;

#[test]
fn test_reopen() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_commit_reopen() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  do_test_reopen_with_commit(&mut random, dir, true)?;
  Ok(())
}

#[test]
fn test_commit_recreate() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  do_test_reopen_with_commit(&mut random, dir, false)?;
  Ok(())
}

fn do_test_reopen_with_commit<R, D>(random: &mut R, dir: Arc<D>, with_reopen: bool) -> Result<()>
where
  R: rand::Rng + ?Sized,
  D: Directory,
{
  let analyzer = MockAnalyzer::new(random);
  let mut config = new_index_writer_config_with_analyzer(random, analyzer);
  config.set_open_mode(OpenMode::Create);
  config.set_merge_scheduler(SerialMergeScheduler::new());
  config.set_merge_policy(new_log_merge_policy(random)?);
  let iwriter = IndexWriter::new(dir.clone(), config)?;
  iwriter.commit()?;
  let mut reader = directory_reader::open(dir.clone())?;

  let m = 3;
  let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type.set_tokenized(false)?;
  let mut custom_type2 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type2.set_tokenized(false)?;
  custom_type2.set_omit_norms(true)?;
  let mut custom_type3 = FieldType::new();
  custom_type3.set_stored(true)?;

  for i in 0..4 {
    for j in 0..m {
      let mut doc = Document::new();
      doc.add(Field::from_string(
        "id",
        format!("{i}_{j}"),
        custom_type.clone(),
      )?);
      doc.add(Field::from_string(
        "id2",
        format!("{i}_{j}"),
        custom_type2.clone(),
      )?);
      doc.add(Field::from_string(
        "id3",
        format!("{i}_{j}"),
        custom_type3.clone(),
      )?);
      iwriter.add_document(doc)?;
      if i > 0 {
        let k = i - 1;
        let n = j + k * m;
        let mut stored_fields = reader.stored_fields()?;
        let previous_iteration_doc = stored_fields.document(n)?;
        let id = previous_iteration_doc.get("id")?;
        assert_eq!(Some(format!("{k}_{j}")), id.map(|value| value.into_owned()));
      }
    }
    iwriter.commit()?;
    if with_reopen {
      // TODO IMPORTANT: openIfChanged未实现
      let r2 = directory_reader::open_from_writer(&iwriter)?;
      reader.close()?;
      reader = r2;
    } else {
      reader.close()?;
      reader = directory_reader::open(dir.clone())?;
    }
  }

  iwriter.close()?;
  reader.close()?;
  Ok(())
}

#[test]
fn test_thread_safety() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}
fn create_document(n: i32, num_fields: i32) -> Result<Document> {
  let mut value = format!("a{n}");
  let mut doc = Document::new();
  let mut custom_type2 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type2.set_tokenized(false)?;
  custom_type2.set_omit_norms(true)?;
  let mut custom_type3 = FieldType::new();
  custom_type3.set_stored(true)?;
  doc.add(TextField::from_string("field1", value.clone(), Store::Yes)?);
  doc.add(Field::from_string("fielda", value.clone(), custom_type2)?);
  doc.add(Field::from_string("fieldb", value.clone(), custom_type3)?);
  value.push_str(&format!(" b{n}"));
  for i in 1..num_fields {
    doc.add(TextField::from_string(
      format!("field{}", i + 1),
      value.clone(),
      Store::Yes,
    )?);
  }
  Ok(doc)
}

#[test]
fn test_reopen_on_commit() -> Result<()> {
  // TODO IMPORTANT list_commits未实现
  Ok(())
}

#[test]
fn test_open_if_changed_nrt_to_commit() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let analyzer = MockAnalyzer::new(&mut random);
  let w = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer),
  )?;
  let mut doc = Document::new();
  doc.add(StringField::from_string("field", "value", Store::No)?);
  w.add_document(doc.clone())?;
  w.commit()?;
  w.add_document(doc)?;
  let r = directory_reader::open_from_writer(&w)?;

  assert_eq!(2, r.num_docs()?);
  // TODO IMPORTANT: openIfChanged未实现
  let r2 = directory_reader::open_from_writer(&w)?;
  r.close()?;
  assert_eq!(2, r2.num_docs()?);
  w.close()?;
  r2.close()?;
  Ok(())
}

fn test_over_dec_ref_during_reopen() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_npe_after_invalid_reindex1() -> Result<()> {
  let mut random = random();
  // TODO IMPORTANT ByteBuffersDirectory 未实现
  let dir = new_directory_shared(&mut random)?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer);
  config.set_merge_policy(NoMergePolicy::default());
  let mut w = IndexWriter::new(dir.clone(), config)?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("id", "id", Store::No)?);
  w.add_document(doc)?;

  doc = Document::new();
  doc.add(StringField::from_string("id", "id2", Store::No)?);
  w.add_document(doc)?;

  w.delete_documents_with_terms(vec![Term::from_text("id", "id")])?;
  w.commit()?;
  w.close()?;
  drop(w);

  let r = directory_reader::open(dir.clone())?;

  for file_name in dir.list_all()? {
    dir.delete_file(&file_name)?;
  }

  let analyzer = MockAnalyzer::new(&mut random);
  w = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer),
  )?;

  doc = Document::new();
  doc.add(StringField::from_string("id", "id", Store::No)?);
  doc.add(NumericDocValuesField::new("ndv", 13));
  w.add_document(doc)?;

  doc = Document::new();
  doc.add(StringField::from_string("id", "id2", Store::No)?);
  w.add_document(doc)?;

  w.commit()?;

  doc = Document::new();
  doc.add(StringField::from_string("id", "id2", Store::No)?);
  w.add_document(doc)?;

  w.update_numeric_doc_value(Term::from_text("id", "id"), "ndv", 17)?;
  w.commit()?;
  w.close()?;

  // TODO IMPORTANT: openIfChanged 未实现
  // let err = directory_reader::open_if_changed(&r);
  // assert!(matches!(err, Err(LuceneError::IllegalState(_))));

  r.close()?;
  Ok(())
}

#[test]
fn test_npe_after_invalid_reindex2() -> Result<()> {
  let mut random = random();
  // TODO IMPORTANT ByteBuffersDirectory未实现
  let dir = new_directory_shared(&mut random)?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer);
  config.set_merge_policy(NoMergePolicy::default());
  let mut w = IndexWriter::new(dir.clone(), config)?;
  let mut doc = Document::new();
  doc.add(StringField::from_string("id", "id", Store::No)?);
  w.add_document(doc)?;
  doc = Document::new();
  doc.add(StringField::from_string("id", "id2", Store::No)?);
  w.add_document(doc)?;
  w.delete_documents_with_terms(vec![Term::from_text("id", "id")])?;
  w.commit()?;
  w.close()?;
  drop(w);

  let r = directory_reader::open(dir.clone())?;

  for name in dir.list_all()? {
    dir.delete_file(&name)?;
  }

  let analyzer = MockAnalyzer::new(&mut random);
  w = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer),
  )?;
  doc = Document::new();
  doc.add(StringField::from_string("id", "id", Store::No)?);
  doc.add(NumericDocValuesField::new("ndv", 13));
  w.add_document(doc)?;
  w.commit()?;
  doc = Document::new();
  doc.add(StringField::from_string("id", "id2", Store::No)?);
  w.add_document(doc)?;
  w.commit()?;

  // TODO IMPORTANT: openIfChanged 未实现
  // let err = directory_reader::open_if_changed(&r);
  // assert!(matches!(err, Err(LuceneError::IllegalState(_))));

  w.close()?;
  r.close()?;
  Ok(())
}

#[test]
fn test_nrt_mdeletes() -> Result<()> {
  // TODO IMPORTANT SnapshotDeletionPolicy未实现
  Ok(())
}

#[test]
fn test_nrt_mdeletes2() -> Result<()> {
  // TODO IMPORTANT SnapshotDeletionPolicy未实现
  Ok(())
}

#[test]
fn test_nrt_mupdates() -> Result<()> {
  // TODO IMPORTANT SnapshotDeletionPolicy未实现
  Ok(())
}

#[test]
fn test_nrt_mupdates2() -> Result<()> {
  // TODO IMPORTANT SnapshotDeletionPolicy未实现
  Ok(())
}

#[test]
fn test_delete_index_files_while_reader_still_open() -> Result<()> {
  let mut random = random();
  // TODO IMPORTANT ByteBuffersDirectory未实现
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut w = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer),
  )?;
  let mut doc = Document::new();
  doc.add(StringField::from_string("field", "value", Store::No)?);
  w.add_document(doc)?;
  w.close()?;
  drop(w);

  let r = directory_reader::open(dir.clone())?;

  for file in dir.list_all()? {
    dir.delete_file(&file)?;
  }

  let analyzer = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer);
  config.set_merge_policy(NoMergePolicy::default());
  w = IndexWriter::new(dir.clone(), config)?;
  doc = Document::new();
  doc.add(StringField::from_string("field", "value", Store::No)?);
  w.add_document(doc)?;

  doc = Document::new();
  doc.add(StringField::from_string("field", "value2", Store::No)?);
  w.add_document(doc.clone())?;

  w.commit()?;

  w.delete_documents_with_terms(vec![Term::from_text("field", "value2")])?;

  w.add_document(doc)?;

  // TODO IMPORTANT: openIfChanged 未实现
  // let err = directory_reader::open_if_changed(&r);
  // assert!(matches!(err, Err(LuceneError::IllegalState(_))));

  w.close()?;
  r.close()?;
  Ok(())
}

#[test]
fn test_reuse_unchanged_leaf_reader_on_dv_update() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut index_writer_config = new_index_writer_config(&mut random);
  index_writer_config.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir.clone(), index_writer_config)?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  doc.add(StringField::from_string("version", "1", Store::Yes)?);
  doc.add(NumericDocValuesField::new("some_docvalue", 2));
  writer.add_document(doc)?;
  doc = Document::new();
  doc.add(StringField::from_string("id", "2", Store::Yes)?);
  doc.add(StringField::from_string("version", "1", Store::Yes)?);
  writer.add_document(doc)?;
  writer.commit()?;
  let mut reader = directory_reader::open(dir.clone())?;
  assert_eq!(2, reader.num_docs()?);
  assert_eq!(2, reader.max_doc()?);
  assert_eq!(0, reader.num_deleted_docs()?);

  doc = Document::new();
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  doc.add(StringField::from_string("version", "2", Store::Yes)?);
  writer.update_doc_values(
    Term::from_text("id", "1"),
    vec![NumericDocValuesField::new("some_docvalue", 1).into()],
  )?;
  writer.commit()?;
  // TODO IMPORTANT: openIfChanged未实现
  let mut new_reader = directory_reader::open_from_writer(&writer)?;
  reader.close()?;
  reader = new_reader;
  assert_eq!(2, reader.num_docs()?);
  assert_eq!(2, reader.max_doc()?);
  assert_eq!(0, reader.num_deleted_docs()?);

  doc = Document::new();
  doc.add(StringField::from_string("id", "3", Store::Yes)?);
  doc.add(StringField::from_string("version", "3", Store::Yes)?);
  writer.update_document_with_term(Some(Term::from_text("id", "3")), doc)?;
  writer.commit()?;

  // TODO IMPORTANT: openIfChanged未实现
  new_reader = directory_reader::open_from_writer(&writer)?;
  assert_eq!(2, new_reader.get_sequential_sub_readers().len());
  assert_eq!(1, reader.get_sequential_sub_readers().len());
  reader.close()?;
  reader = new_reader;
  assert_eq!(3, reader.num_docs()?);
  assert_eq!(3, reader.max_doc()?);
  assert_eq!(0, reader.num_deleted_docs()?);
  reader.close()?;
  writer.close()?;
  Ok(())
}

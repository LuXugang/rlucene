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
use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::{TextField, text_field_type};
use crate::core::index::BytesRef;
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader;
use crate::core::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::{IndexWriter, IndexWriterBase, read_field_infos};
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::{
  MergeContext, MergePolicy, MergePolicyBase, MergePolicyEnum, MergeSpecificationNoReader, OneMerge,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::segment_reader::SegmentReader;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::index::tiered_merge_policy::SegmentDocAndID;
use crate::core::store::directory::Directory;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{LATEST, StringHelper};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::store::base_directory_test_case::EXTRA_FILE_NAME;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  get_only_leaf_reader, new_directory_shared, new_field, new_index_writer_config,
  new_index_writer_config_with_analyzer, new_string_field, new_text_field, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
use rand_xoshiro::rand_core::Rng;
use std::clone::Clone;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::vec;

static STORED_TEXT_TYPE: LazyLock<FieldType> = LazyLock::new(|| {
  FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED).expect("should not fail")
});
pub(crate) struct TestIndexWriter;
#[test]
fn test_doc_count() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  {
    let a = MockAnalyzer::new(&mut random);
    let config = new_index_writer_config_with_analyzer(&mut random, a);
    let writer = IndexWriter::new(dir.clone(), config)?;
    for i in 0..100 {
      add_doc_with_index(&mut random, &writer, i, &mut field_types)?;
      if random.random_bool(0.5) {
        writer.commit()?;
      }
    }
    let doc_stats = writer.get_doc_stats()?;
    assert_eq!(100, doc_stats.max_doc);
    assert_eq!(100, doc_stats.num_docs);
    writer.close()?;
  }

  {
    let mut config = new_index_writer_config(&mut random);
    config.set_merge_policy(KeepFullyDeletedSegmentsMergePolicy::default());
    let writer = IndexWriter::new(dir.clone(), config)?;
    for i in 0..40 {
      writer.delete_documents_with_terms(vec![Term::from_text("id", i.to_string())])?;
      if random.random_bool(0.5) {
        writer.commit()?;
      }
    }
    writer.flush()?;
    let doc_stats = writer.get_doc_stats()?;
    assert_eq!(100, doc_stats.max_doc);
    assert_eq!(60, doc_stats.num_docs);
    writer.close()?;
  }

  {
    let reader = directory_reader::open(dir.clone())?;
    assert_eq!(60, reader.num_docs()?);
    reader.close()?;
  }

  {
    let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
    assert_eq!(60, writer.get_doc_stats()?.num_docs);
    writer.force_merge(1)?;
    let doc_stats = writer.get_doc_stats()?;
    assert_eq!(60, doc_stats.max_doc);
    assert_eq!(60, doc_stats.num_docs);
    writer.close()?;
  }

  {
    let reader = directory_reader::open(dir.clone())?;
    assert_eq!(60, reader.max_doc()?);
    assert_eq!(60, reader.num_docs()?);
    reader.close()?;
  }

  {
    let mut config = new_index_writer_config(&mut random);
    config.set_open_mode(OpenMode::Create);
    let writer = IndexWriter::new(dir, config)?;
    let doc_stats = writer.get_doc_stats()?;
    assert_eq!(0, doc_stats.max_doc);
    assert_eq!(0, doc_stats.num_docs);
    writer.close()?;
  }
  Ok(())
}
// Make sure we can flush segment w/ norms, then add empty doc (no norms) and flush
#[test]
fn test_empty_doc_after_flushing_real_doc() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let config = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut field_types = HashMap::new();
  let mut doc = Document::new();

  let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type.set_store_term_vectors(true)?;
  custom_type.set_store_term_vector_positions(true)?;
  custom_type.set_store_term_vector_offsets(true)?;

  doc.add(new_field(
    &mut random,
    "field",
    "aaa",
    &custom_type,
    &mut field_types,
  )?);
  writer.add_document(doc)?;
  writer.commit()?;
  if cfg!(feature = "test_log_verbose") {
    println!("TEST: now add empty doc");
  }
  let empty_doc = Document::new();
  writer.add_document(empty_doc)?;
  writer.close()?;
  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(2, reader.num_docs()?);

  Ok(())
}
#[test]
fn test_bad_segment() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let config = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut field_types = HashMap::new();
  let mut doc = Document::new();
  let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
  custom_type.set_store_term_vectors(true)?;
  doc.add(new_field(
    &mut random,
    "tvtest",
    "",
    &custom_type,
    &mut field_types,
  )?);

  writer.add_document(doc)?;
  writer.close()?;
  Ok(())
}
#[test]
fn test_max_thread_priority() -> Result<()> {
  // TODO
  Ok(())
}
#[test]
fn test_variable_schema() -> Result<()> {
  // TODO
  Ok(())
}
#[test]
fn test_unlimited_max_field_length() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let config = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut field_types = HashMap::new();
  let mut doc = Document::new();
  let text = " a".repeat(10_000) + " x";
  doc.add(new_text_field(
    &mut random,
    "field",
    &text,
    Store::No,
    &mut field_types,
  )?);
  writer.add_document(doc)?;
  writer.close()?;

  let reader = directory_reader::open(dir.clone())?;
  let t = Term::from_text("field", "x");
  assert_eq!(1, reader.doc_freq(&t)?);
  Ok(())
}
#[test]
fn test_empty_field_name() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let config = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut field_types = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "",
    "a b c",
    Store::No,
    &mut field_types,
  )?);
  writer.add_document(doc)?;
  writer.close()?;

  Ok(())
}
#[test]
fn test_empty_field_name_terms() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let config = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "",
    "a b c",
    Store::No,
    &mut field_to_type,
  )?);

  writer.add_document(doc)?;
  writer.close()?;

  let reader = directory_reader::open(dir)?;
  let subreader = get_only_leaf_reader(&reader)?;

  let terms = subreader.terms("")?.unwrap();
  let mut te = terms.iterator()?;

  assert_eq!(&BytesRef::from_string("a"), te.next()?.unwrap().as_ref());
  assert_eq!(&BytesRef::from_string("b"), te.next()?.unwrap().as_ref());
  assert_eq!(&BytesRef::from_string("c"), te.next()?.unwrap().as_ref());
  assert_eq!(None, te.next()?);

  Ok(())
}
#[test]
fn test_empty_field_name_with_empty_term() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let config = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();

  doc.add(new_string_field(
    &mut random,
    "",
    "",
    Store::No,
    &mut field_to_type,
  )?);
  doc.add(new_string_field(
    &mut random,
    "",
    "a",
    Store::No,
    &mut field_to_type,
  )?);
  doc.add(new_string_field(
    &mut random,
    "",
    "b",
    Store::No,
    &mut field_to_type,
  )?);
  doc.add(new_string_field(
    &mut random,
    "",
    "c",
    Store::No,
    &mut field_to_type,
  )?);

  writer.add_document(doc)?;
  writer.close()?;

  let reader = directory_reader::open(dir)?;
  let subreader = get_only_leaf_reader(&reader)?;

  let terms = subreader.terms("")?.unwrap();
  let mut te = terms.iterator()?;

  assert_eq!(&BytesRef::from_string(""), te.next()?.unwrap().as_ref());
  assert_eq!(&BytesRef::from_string("a"), te.next()?.unwrap().as_ref());
  assert_eq!(&BytesRef::from_string("b"), te.next()?.unwrap().as_ref());
  assert_eq!(&BytesRef::from_string("c"), te.next()?.unwrap().as_ref());
  assert_eq!(None, te.next()?);

  Ok(())
}
struct MockIndexWriter {
  after_was_called: AtomicBool,
  before_was_called: AtomicBool,
}
impl MockIndexWriter {
  fn new() -> Self {
    MockIndexWriter {
      after_was_called: AtomicBool::new(false),
      before_was_called: AtomicBool::new(false),
    }
  }
}
impl IndexWriterBase for MockIndexWriter {
  fn do_after_flush(&self) -> Result<()> {
    self.after_was_called.store(true, SeqCst);
    Ok(())
  }

  fn do_before_flush(&self) -> Result<()> {
    self.before_was_called.store(true, SeqCst);
    Ok(())
  }
}

#[test]
fn test_do_before_after_flush() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock_index_writer = MockIndexWriter::new();
  let writer = IndexWriter::with_sub(
    dir.clone(),
    new_index_writer_config(&mut random),
    Some(mock_index_writer),
  )?;

  let mut field_types = HashMap::new();
  let mut doc = Document::new();
  let custom_type = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  doc.add(new_field(
    &mut random,
    "field",
    "a field",
    &custom_type,
    &mut field_types,
  )?);
  writer.add_document(doc)?;
  writer.commit()?;

  assert!(writer.sub.as_ref().unwrap().before_was_called.load(SeqCst));
  assert!(writer.sub.as_ref().unwrap().after_was_called.load(SeqCst));
  writer
    .sub
    .as_ref()
    .unwrap()
    .before_was_called
    .store(false, SeqCst);
  writer
    .sub
    .as_ref()
    .unwrap()
    .after_was_called
    .store(false, SeqCst);

  writer.delete_documents_with_terms(vec![Term::from_text("field", "field"); 1])?;
  writer.commit()?;

  assert!(writer.sub.as_ref().unwrap().before_was_called.load(SeqCst));
  assert!(writer.sub.as_ref().unwrap().after_was_called.load(SeqCst));

  writer.close()?;

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(0, reader.num_docs()?);

  Ok(())
}
#[test]
fn test_negative_positions() -> Result<()> {
  // TODO TokenStream 不支持
  Ok(())
}
#[test]
fn test_position_increment_gap_empty_field() -> Result<()> {
  // TODO
  Ok(())
}
#[test]
fn test_dead_lock() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_thread_interrupt_dead_lock() -> Result<()> {
  // TODO
  Ok(())
}
#[test]
fn test_index_store_combos() -> Result<()> {
  // TODO
  Ok(())
}
#[test]
fn test_no_docs_index() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), iwc)?;
  writer.add_document(Document::new())?;
  writer.close()?;

  Ok(())
}
#[test]
fn test_delete_unused_files() -> Result<()> {
  // TODO
  Ok(())
}
#[test]
fn test_delete_unused_files2() -> Result<()> {
  // TODO
  Ok(())
}
#[test]
fn test_empty_fsdir_with_no_lock() -> Result<()> {
  // TODO
  Ok(())
}
#[test]
fn test_empty_dir_roll_back() -> Result<()> {
  // TODO : rollback 未实现
  Ok(())
}
#[test]
fn test_delete_same_term_across_fields() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let mut doc = Document::new();
  doc.add(TextField::from_string("a", "foo", Store::No)?);
  writer.add_document(doc)?;

  writer.delete_documents_with_terms(vec![
    Term::from_text("a", "xxx"),
    Term::from_text("b", "foo"),
  ])?;

  let reader = directory_reader::open_from_writer(&writer)?;
  writer.close()?;
  assert_eq!(1, reader.num_docs()?);

  Ok(())
}
fn assert_files<D, L, B>(writer: &IndexWriter<D, L, B>) -> Result<()>
where
  D: Directory,
  L: LiveIndexWriterConfig,
  B: IndexWriterBase,
{
  use std::collections::HashSet;

  let filter = |file: &str| !file.starts_with("segments") && file != "write.lock";
  // remove segment files we don't know if we have committed and what is kept around
  let seg_files: HashSet<String> = writer
    .clone_segment_infos()?
    .files(true)?
    .into_iter()
    .filter(|f| filter(f))
    .collect();

  let dir_files: HashSet<String> = writer
    .get_directory()
    .list_all()?
    .into_iter()
    .filter(|f| f != EXTRA_FILE_NAME)
    .filter(|f| filter(f))
    .collect();

  assert_eq!(seg_files.len(), dir_files.len(),);

  Ok(())
}

#[test]
fn test_fully_deleted_segments_release_files() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let config = new_index_writer_config(&mut random);
  // TODO: 没有定义flush条件
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut d = Document::new();
  d.add(StringField::from_string("id", "doc-0", Store::Yes)?);
  writer.add_document(d)?;
  writer.flush()?;

  let mut d = Document::new();
  d.add(StringField::from_string("id", "doc-1", Store::Yes)?);
  writer.add_document(d)?;
  writer.delete_documents_with_terms(vec![Term::from_text("id", "doc-1")])?;

  assert_eq!(1, writer.clone_segment_infos()?.size());
  writer.flush()?;
  assert_eq!(1, writer.clone_segment_infos()?.size());
  writer.commit()?;

  assert_files(&writer)?;
  assert_eq!(1, writer.clone_segment_infos()?.size());
  writer.close()?;
  Ok(())
}
#[test]
fn test_segment_info_is_snapshot() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut config = new_index_writer_config(&mut random);
  config.set_ram_buffer_size_mb(i32::MAX as f64);
  config.set_max_buffered_docs(2);
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut d = Document::new();
  d.add(StringField::from_string("id", "doc-0", Store::Yes)?);
  writer.add_document(d)?;

  let mut d = Document::new();
  d.add(StringField::from_string("id", "doc-1", Store::Yes)?);
  writer.add_document(d)?;

  let reader = directory_reader::open_from_writer(&writer)?;
  let context = get_context(reader)?;
  let r = context.leaves()?;
  let segment_reader = r.first().unwrap().reader();
  let segment_info = segment_reader.get_segment_info();
  let original_info_id = segment_reader.get_original_segment_info_id();
  let clone_segment_infos = writer.clone_segment_infos()?;
  let original_info = clone_segment_infos.index_of(original_info_id).unwrap();

  assert_eq!(0, original_info.get_del_count());
  assert_eq!(0, segment_info.get_del_count());

  writer.delete_documents_with_terms(vec![Term::from_text("id", "doc-0")])?;
  writer.commit()?;
  // snapshot
  assert_eq!(0, segment_info.get_del_count());
  writer.close()?;
  Ok(())
}
#[test]
fn test_prevent_changing_soft_deletes_field() -> Result<()> {
  // TODO IMPORTANT SoftDeletesRetentionMergePolicy未实现
  Ok(())
}
fn test_prevent_adding_indexes_with_different_soft_deletes_field() -> Result<()> {
  let mut random = random();

  let dir1 = new_directory_shared(&mut random)?;
  let mut config = new_index_writer_config(&mut random);
  config.set_soft_deletes_field("soft_deletes_1");
  let w1 = IndexWriter::new(dir1.clone(), config)?;

  for i in 0..2 {
    let mut d = Document::new();
    d.add(StringField::from_string("id", "1", Store::Yes)?);
    d.add(StringField::from_string(
      "version",
      i.to_string(),
      Store::Yes,
    )?);

    w1.soft_update_document(
      Term::from_text("id", "1"),
      d,
      vec![NumericDocValuesField::new("soft_deletes_1", 1).into()],
    )?;
  }

  w1.commit()?;
  w1.close()?;
  drop(w1);

  let dir2 = new_directory_shared(&mut random)?;
  let mut config = new_index_writer_config(&mut random);
  config.set_soft_deletes_field("soft_deletes_2");
  let w2 = IndexWriter::new(dir2.clone(), config)?;

  let err = w2.add_indexes_from_dir(std::slice::from_ref(&dir1));
  match err {
    Ok(_) => panic!("expected IllegalArgument error"),
    Err(err) => {
      assert!(matches!(err, LuceneError::IllegalArgument(_)));
      assert_eq!(
        "cannot configure [soft_deletes_2] as soft-deletes; this index uses [soft_deletes_1] as soft-deletes already",
        err.to_string()
      );
    },
  }

  w2.close()?;

  let dir3 = new_directory_shared(&mut random)?;
  let mut config = new_index_writer_config(&mut random);
  config.set_soft_deletes_field("soft_deletes_1");
  let w3 = IndexWriter::new(dir3, config)?;

  w3.add_indexes_from_dir(std::slice::from_ref(&dir1))?;

  for si in w3.clone_segment_infos()?.iter() {
    let field_infos = read_field_infos(si)?;
    let soft_delete_field = field_infos.field_info_by_name("soft_deletes_1").unwrap();
    assert!(soft_delete_field.is_soft_deletes_field());
  }

  w3.close()?;

  Ok(())
}

#[test]
fn test_not_allow_using_existing_field_as_soft_deletes() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

  for _ in 0..2 {
    let mut d = Document::new();
    d.add(StringField::from_string("id", "1", Store::Yes)?);

    if random.random_bool(0.5) {
      d.add(NumericDocValuesField::new("dv_field", 1));
      w.update_documents_with_term(Term::from_text("id", "1"), d)?;
    } else {
      w.soft_update_document(
        Term::from_text("id", "1"),
        d,
        vec![NumericDocValuesField::new("dv_field", 1).into()],
      )?;
    }
  }

  w.commit()?;
  w.close()?;
  drop(w);
  let soft_deletes_field = if random.random_bool(0.5) {
    "id"
  } else {
    "dv_field"
  };

  let mut config = new_index_writer_config(&mut random);
  config.set_soft_deletes_field(soft_deletes_field);

  let err = IndexWriter::new(dir.clone(), config);
  match err {
    Ok(_) => panic!("expected IllegalArgument error"),
    Err(err) => {
      assert!(matches!(err, LuceneError::IllegalArgument(_)));
      assert_eq!(
        format!(
          "cannot configure [{}] as soft-deletes; this index uses [{}] as non-soft-deletes already",
          soft_deletes_field, soft_deletes_field
        ),
        err.to_string()
      );
    },
  }

  let mut config = new_index_writer_config(&mut random);
  config.set_soft_deletes_field("non-existing-field");

  let w = IndexWriter::new(dir, config)?;
  w.close()?;

  Ok(())
}
#[test]
fn test_broken_payload() -> Result<()> {
  // TODO CannedTokenStream未实现
  Ok(())
}
fn test_soft_and_hard_live_docs() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let mut index_writer_config = new_index_writer_config(&mut random);
  let soft_deletes_field = "soft_delete";
  index_writer_config.set_soft_deletes_field(soft_deletes_field);

  let writer = IndexWriter::new(dir.clone(), index_writer_config)?;
  let mut unique_docs = HashSet::new();

  for _ in 0..100 {
    let doc_id = random.random_range(0..5);
    unique_docs.insert(doc_id);

    let mut doc = Document::new();
    doc.add(StringField::from_string(
      "id",
      doc_id.to_string(),
      Store::Yes,
    )?);

    if doc_id % 2 == 0 {
      writer.update_documents_with_term(Term::from_text("id", doc_id.to_string()), doc)?;
    } else {
      writer.soft_update_document(
        Term::from_text("id", doc_id.to_string()),
        doc,
        vec![NumericDocValuesField::new(soft_deletes_field, 0).into()],
      )?;
    }

    if random.random_bool(0.5) {
      assert_hard_live_docs(&writer, &unique_docs)?;
    }
  }

  if random.random_bool(0.5) {
    writer.commit()?;
  }
  assert_hard_live_docs(&writer, &unique_docs)?;

  writer.close()?;

  Ok(())
}
#[test]
fn test_abort_fully_deleted_segment() -> Result<()> {
  // TODO 未实现
  Ok(())
}
fn assert_hard_live_docs<D, L, B>(
  _writer: &IndexWriter<D, L, B>,
  _unique_docs: &HashSet<i32>,
) -> Result<()>
where
  D: Directory,
  L: LiveIndexWriterConfig,
  B: IndexWriterBase,
{
  // TODO IMPORTANT 未实现
  Ok(())
}
#[test]
fn test_set_index_created_version() -> Result<()> {
  let mut random = random();

  let mut iwc = new_index_writer_config(&mut random);
  let err = iwc.set_index_created_version_major(LATEST.major + 1);
  match err {
    Ok(_) => unreachable!("expected IllegalArgument error"),
    Err(err) => {
      assert!(matches!(err, LuceneError::IllegalArgument(_)));
      assert_eq!(
        format!(
          "indexCreatedVersionMajor may not be in the future: current major version is {}, but got: {}",
          LATEST.major,
          LATEST.major + 1
        ),
        err.to_string()
      );
    },
  }

  let mut iwc = new_index_writer_config(&mut random);
  let err = iwc.set_index_created_version_major(LATEST.major - 2);
  match err {
    Ok(_) => unreachable!("expected IllegalArgument error"),
    Err(err) => {
      assert!(matches!(err, LuceneError::IllegalArgument(_)));
      assert_eq!(
        format!(
          "indexCreatedVersionMajor may not be less than the minimum supported version: {}, but got: {}",
          LATEST.major - 1,
          LATEST.major - 2
        ),
        err.to_string()
      );
    },
  }

  for previous_major in LATEST.major - 1..=LATEST.major {
    for new_major in LATEST.major - 1..=LATEST.major {
      for open_mode in [OpenMode::Create, OpenMode::Append, OpenMode::CreateOrAppend] {
        let dir = new_directory_shared(&mut random)?;

        {
          let mut iwc = new_index_writer_config(&mut random);
          iwc.set_index_created_version_major(previous_major)?;
          let w = IndexWriter::new(dir.clone(), iwc)?;
          w.close()?;
        }

        let mut infos = SegmentInfos::read_latest_commit(dir.clone())?;
        assert_eq!(previous_major, infos.get_index_created_version_major());

        {
          let mut iwc = new_index_writer_config(&mut random);
          iwc.set_open_mode(open_mode);
          iwc.set_index_created_version_major(new_major)?;
          let w = IndexWriter::new(dir.clone(), iwc)?;
          w.close()?;
        }

        infos = SegmentInfos::read_latest_commit(dir)?;
        if open_mode == OpenMode::Create {
          assert_eq!(new_major, infos.get_index_created_version_major());
        } else {
          assert_eq!(previous_major, infos.get_index_created_version_major());
        }
      }
    }
  }

  Ok(())
}

#[test]
fn test_flush_while_starting_new_threads() -> Result<()> {
  // TODO IMPORTANT 多线程未实现
  Ok(())
}

#[test]
fn test_refresh_and_rollback_concurrently() -> Result<()> {
  // TODO IMPORTANT 多线程未实现
  Ok(())
}

#[test]
fn test_closeable_queue() -> Result<()> {
  // TODO IMPORTANT 多线程未实现
  Ok(())
}
#[test]
fn test_random_operations() -> Result<()> {
  // TODO IMPORTANT 多线程未实现
  Ok(())
}
#[test]
fn test_random_operations_with_soft_deletes() -> Result<()> {
  // TODO IMPORTANT 多线程未实现
  Ok(())
}

#[test]
fn test_max_completed_sequence_number() -> Result<()> {
  // TODO IMPORTANT 多线程未实现
  Ok(())
}

#[test]
fn test_ensure_max_seq_no_is_accurate_during_flush() -> Result<()> {
  // TODO IMPORTANT 多线程未实现
  Ok(())
}
#[test]
fn test_segment_commit_info_id() -> Result<()> {
  let mut random = random();

  {
    let dir = new_directory_shared(&mut random)?;
    let v = {
      let mut iwc = new_index_writer_config(&mut random);
      iwc.set_merge_policy(NoMergePolicy::default());
      let writer = IndexWriter::new(dir.clone(), iwc)?;

      let mut doc = Document::new();
      doc.add(NumericDocValuesField::new("num", 1));
      doc.add(StringField::from_string("id", "1", Store::No)?);
      writer.add_document(doc)?;

      let mut doc = Document::new();
      doc.add(NumericDocValuesField::new("num", 1));
      doc.add(StringField::from_string("id", "2", Store::No)?);
      writer.add_document(doc)?;

      writer.commit()?;
      let segment_commit_infos = SegmentInfos::read_latest_commit(dir.clone())?;
      let mut id = segment_commit_infos.info(0).unwrap().get_id();
      let seg_info_id = segment_commit_infos.info(0).unwrap().info.get_id();

      writer.update_numeric_doc_value(Term::from_text("id", "1"), "num", 2)?;
      writer.commit()?;

      let segment_commit_infos = SegmentInfos::read_latest_commit(dir.clone())?;
      assert_eq!(1, segment_commit_infos.size());
      assert_ne!(
        StringHelper::id_to_string(id),
        StringHelper::id_to_string(segment_commit_infos.info(0).unwrap().get_id())
      );
      assert_eq!(
        StringHelper::id_to_string(Some(seg_info_id)),
        StringHelper::id_to_string(Some(segment_commit_infos.info(0).unwrap().info.get_id()))
      );

      id = segment_commit_infos.info(0).unwrap().get_id();

      writer.add_document(Document::new())?;
      writer.commit()?;

      let segment_commit_infos = SegmentInfos::read_latest_commit(dir.clone())?;
      assert_eq!(2, segment_commit_infos.size());
      assert_eq!(
        StringHelper::id_to_string(id),
        StringHelper::id_to_string(segment_commit_infos.info(0).unwrap().get_id())
      );
      assert_eq!(
        StringHelper::id_to_string(Some(seg_info_id)),
        StringHelper::id_to_string(Some(segment_commit_infos.info(0).unwrap().info.get_id()))
      );

      let mut doc = Document::new();
      doc.add(NumericDocValuesField::new("num", 5));
      doc.add(StringField::from_string("id", "1", Store::No)?);
      writer.update_documents_with_term(Term::from_text("id", "1"), doc)?;
      writer.commit()?;

      let segment_commit_infos = SegmentInfos::read_latest_commit(dir.clone())?;
      assert_eq!(3, segment_commit_infos.size());
      assert_ne!(
        StringHelper::id_to_string(id),
        StringHelper::id_to_string(segment_commit_infos.info(0).unwrap().get_id())
      );
      assert_eq!(
        StringHelper::id_to_string(Some(seg_info_id)),
        StringHelper::id_to_string(Some(segment_commit_infos.info(0).unwrap().info.get_id()))
      );

      writer.close()?;
      segment_commit_infos
    };

    {
      let dir2 = new_directory_shared(&mut random)?;
      let mut iwc = new_index_writer_config(&mut random);
      iwc.set_merge_policy(NoMergePolicy::default());
      let writer2 = IndexWriter::new(dir2.clone(), iwc)?;

      writer2.add_indexes_from_dir(std::slice::from_ref(&dir))?;
      writer2.commit()?;

      let infos2 = SegmentInfos::read_latest_commit(dir2)?;
      assert_eq!(infos2.size(), v.size());

      for i in 0..infos2.size() {
        assert_eq!(
          StringHelper::id_to_string(infos2.info(i).unwrap().get_id()),
          StringHelper::id_to_string(v.info(i).unwrap().get_id())
        );
        assert_eq!(
          StringHelper::id_to_string(Some(infos2.info(i).unwrap().info.get_id())),
          StringHelper::id_to_string(Some(v.info(i).unwrap().info.get_id()))
        );
      }

      writer2.close()?;
    }
  }

  let mut ids = HashSet::new();

  for _ in 0..2 {
    let dir = new_directory_shared(&mut random)?;
    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_merge_policy(NoMergePolicy::default());
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("num", 1));
    doc.add(StringField::from_string("id", "1", Store::No)?);
    writer.add_document(doc)?;
    writer.commit()?;

    let mut segment_commit_infos = SegmentInfos::read_latest_commit(dir.clone())?;
    let id = StringHelper::id_to_string(segment_commit_infos.info(0).unwrap().get_id());
    assert!(ids.insert(id));

    writer.update_numeric_doc_value(Term::from_text("id", "1"), "num", 2)?;
    writer.commit()?;

    segment_commit_infos = SegmentInfos::read_latest_commit(dir)?;
    let id = StringHelper::id_to_string(segment_commit_infos.info(0).unwrap().get_id());
    assert!(ids.insert(id));

    writer.close()?;
  }

  Ok(())
}
#[test]
fn test_merge_zero_docs_merge_is_closed_once() -> Result<()> {
  // TODO IMPORTANT OneMergeWrappingMergePolicy未实现
  Ok(())
}

#[test]
fn test_merge_on_commit_keep_fully_deleted_segments() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_max_full_flush_merge_wait_millis(30 * 1000);
  iwc.set_merge_policy(KeepFullyDeletedSegmentsMergePolicy::with_full_flush_merges());
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let mut d = Document::new();
  d.add(StringField::from_string("id", "1", Store::Yes)?);
  writer.add_document(d.clone())?;
  writer.commit()?;
  writer.update_documents_with_term(Term::from_text("id", "1"), d)?;
  writer.commit()?;

  let reader = directory_reader::open_from_writer(&writer)?;
  assert_eq!(1, reader.num_docs()?);
  reader.close()?;
  writer.close()?;
  Ok(())
}
#[test]
fn test_pending_num_docs() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let num_docs = random.random_range(0..100);

  {
    let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
    for i in 0..num_docs {
      let mut d = Document::new();
      d.add(StringField::from_string("id", i.to_string(), Store::Yes)?);
      writer.add_document(d)?;
      assert_eq!(i as i64 + 1, writer.get_pending_num_docs());
    }
    assert_eq!(num_docs as i64, writer.get_pending_num_docs());
    writer.flush()?;
    assert_eq!(num_docs as i64, writer.get_pending_num_docs());
    writer.close()?;
  }

  {
    let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
    assert_eq!(num_docs as i64, writer.get_pending_num_docs());
    writer.close()?;
  }
  Ok(())
}
#[test]
fn test_index_writer_blocks_on_stall() -> Result<()> {
  // TODO IMPORTANT 多线程未实现
  Ok(())
}
#[test]
fn test_get_field_names() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  {
    let mut writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
    let mut field_types = HashMap::new();

    assert_eq!(HashSet::<String>::new(), writer.get_field_names());

    add_doc_with_field(&mut random, &mut writer, "f1", &mut field_types)?;
    assert_eq!(HashSet::from(["f1".to_string()]), writer.get_field_names());

    let field_set = writer.get_field_names();

    add_doc_with_field(&mut random, &mut writer, "f2", &mut field_types)?;
    assert_eq!(
      HashSet::from(["f1".to_string(), "f2".to_string()]),
      writer.get_field_names()
    );
    assert_eq!(HashSet::from(["f1".to_string()]), field_set);

    // flush should not change field names
    writer.flush()?;
    assert_eq!(
      HashSet::from(["f1".to_string(), "f2".to_string()]),
      writer.get_field_names()
    );

    // commit should not change field names
    writer.commit()?;
    assert_eq!(
      HashSet::from(["f1".to_string(), "f2".to_string()]),
      writer.get_field_names()
    );

    writer.close()?;
  }

  // reopen writer — should detect committed fields
  let mock = MockAnalyzer::new(&mut random);
  let config = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), config)?;
  assert_eq!(
    HashSet::from(["f1".to_string(), "f2".to_string()]),
    writer.get_field_names()
  );

  writer.delete_all()?;
  assert_eq!(HashSet::<String>::new(), writer.get_field_names());

  writer.close()?;
  Ok(())
}

fn add_doc_with_field<D, L, B, R>(
  random: &mut R,
  writer: &mut IndexWriter<D, L, B>,
  field: &str,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory,
  L: LiveIndexWriterConfig,
  B: IndexWriterBase,
  R: Rng + ?Sized,
{
  let mut doc = Document::new();
  let stored_text_type = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  doc.add(new_field(
    random,
    field,
    "value",
    &stored_text_type,
    field_types,
  )?);
  let _ = writer.add_document(doc)?;
  Ok(())
}

#[test]
fn test_parent_and_soft_deletes_are_the_same() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut index_writer_config = new_index_writer_config_with_analyzer(&mut random, mock);
  index_writer_config.set_soft_deletes_field("foo");
  index_writer_config.set_parent_field("foo");

  let err = IndexWriter::new(dir, index_writer_config);
  match err {
    Ok(_) => unreachable!("expected IllegalArgument error"),
    Err(err) => {
      assert!(matches!(err, LuceneError::IllegalArgument(_)));
      assert_eq!(
        "parent document and soft-deletes field can't be the same field \"foo\"",
        err.to_string()
      );
    },
  }

  Ok(())
}

#[test]
fn test_parent_field_existing_index() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  {
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut field_to_type = HashMap::new();
    let mut d = Document::new();
    d.add(new_text_field(
      &mut random,
      "f",
      "a",
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(d)?;
    writer.close()?;
  }

  {
    let mock = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    iwc.set_open_mode(OpenMode::Append);
    iwc.set_parent_field("foo");

    let err = IndexWriter::new(dir.clone(), iwc);
    match err {
      Ok(_) => unreachable!("expected IllegalArgument error"),
      Err(err) => {
        assert!(matches!(err, LuceneError::IllegalArgument(_)));
        assert_eq!(
          "can't add a parent field to an already existing index without a parent field",
          err.to_string()
        );
      },
    }
  }

  {
    let mock = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    iwc.set_open_mode(OpenMode::CreateOrAppend);
    iwc.set_parent_field("foo");

    let err = IndexWriter::new(dir.clone(), iwc);
    match err {
      Ok(_) => unreachable!("expected IllegalArgument error"),
      Err(err) => {
        assert!(matches!(err, LuceneError::IllegalArgument(_)));
        assert_eq!(
          "can't add a parent field to an already existing index without a parent field",
          err.to_string()
        );
      },
    }
  }

  {
    let mock = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    iwc.set_open_mode(OpenMode::Create);
    iwc.set_parent_field("foo");

    let writer = IndexWriter::new(dir, iwc)?;
    writer.add_document(Document::new())?;
    writer.close()?;
  }

  Ok(())
}
#[test]
fn test_parent_field_is_already_used() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  {
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(StringField::from_string(
      "parent",
      1.to_string(),
      Store::Yes,
    )?);
    writer.add_document(doc)?;
    writer.commit()?;
    writer.close()?;
  }

  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config.set_parent_field("parent");

  let err = IndexWriter::new(dir, config);

  assert!(err.is_err());

  let err = match err {
    Ok(_) => unreachable!(),
    Err(err) => err,
  };

  assert!(matches!(err, LuceneError::IllegalArgument(_)));
  assert_eq!(
    "can't add [parent] as non parent document field; this IndexWriter is configured with [parent] as parent document field",
    err.to_string()
  );

  Ok(())
}
#[test]
fn test_parent_field_empty_index() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  {
    let mock = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    iwc.set_parent_field("parent");
    let writer = IndexWriter::new(dir.clone(), iwc)?;
    writer.commit()?;
    writer.close()?;
  }

  {
    let mock = MockAnalyzer::new(&mut random);
    let mut iwc2 = new_index_writer_config_with_analyzer(&mut random, mock);
    iwc2.set_parent_field("parent");
    let writer = IndexWriter::new(dir, iwc2)?;
    writer.commit()?;
    writer.close()?;
  }

  Ok(())
}

#[test]
fn test_doc_values_mixed_skipping_index() -> Result<()> {
  let mut random = random();
  {
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir, iwc)?;

    let mut doc1 = Document::new();
    doc1.add(SortedNumericDocValuesField::indexed_field(
      "test",
      random.random(),
    ));
    writer.add_document(doc1)?;

    let mut doc2 = Document::new();
    doc2.add(SortedNumericDocValuesField::new("test", random.random()));

    let err = writer.add_document(doc2);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    let err = err.unwrap_err();
    assert_eq!(
      "Inconsistency of field data structures across documents for field [test] of doc [1]. doc values skip index type: expected 'Range', but it has 'None'.",
      err.to_string()
    );

    writer.close()?;
  }

  {
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir, iwc)?;

    let mut doc1 = Document::new();
    doc1.add(SortedSetDocValuesField::new(
      "test",
      TestUtil::random_binary_term(&mut random),
    ));
    writer.add_document(doc1)?;

    let mut doc2 = Document::new();
    doc2.add(SortedSetDocValuesField::indexed_field(
      "test",
      TestUtil::random_binary_term(&mut random),
    ));

    let err = writer.add_document(doc2);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    let err = err.unwrap_err();
    assert_eq!(
      "Inconsistency of field data structures across documents for field [test] of doc [1]. doc values skip index type: expected 'None', but it has 'Range'.",
      err.to_string()
    );

    writer.close()?;
  }

  Ok(())
}
#[test]
fn test_doc_values_skipping_index_without_doc_values() -> Result<()> {
  let mut random = random();

  for doc_values_type in [DocValuesType::None, DocValuesType::Binary] {
    let mut field_type = FieldType::new();
    field_type.set_stored(true)?;
    field_type.set_doc_values_type(doc_values_type)?;
    field_type.set_doc_values_skip_index_type(DocValuesSkipIndexType::Range)?;
    field_type.freeze();
    // TODO IMPORTANT newMockDirectory未实现
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir, iwc)?;

    let mut doc1 = Document::new();
    doc1.add(Field::from_binary("test", vec![0u8; 10], field_type)?);

    let err = writer.add_document(doc1);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(
      err
        .unwrap_err()
        .to_string()
        .starts_with("field 'test' cannot have docValuesSkipIndexType=Range")
    );

    writer.close()?;
  }

  Ok(())
}

pub(crate) fn add_doc<D, L, B, R>(
  random: &mut R,
  writer: &IndexWriter<D, L, B>,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory,
  L: LiveIndexWriterConfig,
  B: IndexWriterBase,
  R: Rng + ?Sized,
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
pub(crate) fn add_doc_with_index<D, L, B, R>(
  random: &mut R,
  writer: &IndexWriter<D, L, B>,
  index: i32,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory,
  L: LiveIndexWriterConfig,
  B: IndexWriterBase,
  R: Rng + ?Sized,
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

  match writer.add_document(doc) {
    Ok(_) => Ok(()),
    Err(e) => Err(e),
  }
}

#[derive(Clone, Default)]
pub struct KeepFullyDeletedSegmentsMergePolicy {
  in_: NoMergePolicy,
  merge_fully_deleted_on_full_flush: bool,
}

impl KeepFullyDeletedSegmentsMergePolicy {
  fn with_full_flush_merges() -> Self {
    Self {
      in_: NoMergePolicy::default(),
      merge_fully_deleted_on_full_flush: true,
    }
  }
}

impl From<KeepFullyDeletedSegmentsMergePolicy> for MergePolicyEnum {
  fn from(value: KeepFullyDeletedSegmentsMergePolicy) -> Self {
    MergePolicyEnum::KeepFullyDeletedSegments(value)
  }
}

impl Display for KeepFullyDeletedSegmentsMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "KeepFullyDeletedSegmentsMergePolicy")
  }
}

impl MergePolicy for KeepFullyDeletedSegmentsMergePolicy {
  fn get_base(&self) -> &MergePolicyBase {
    self.in_.get_base()
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    self.in_.get_base_mut()
  }

  fn find_merges<D, MC>(
    &self,
    merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&crate::core::index::index_writer::Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self
      .in_
      .find_merges(merge_trigger, segment_infos, inner, merge_context)
  }

  fn find_forced_merges<D, MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    max_segment_count: usize,
    segments_to_merge: &HashMap<String, Option<bool>>,
    inner: Option<&crate::core::index::index_writer::Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
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

  fn find_forced_deletes_merges<D, MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&crate::core::index::index_writer::Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self
      .in_
      .find_forced_deletes_merges(segment_infos, inner, merge_context)
  }

  fn find_full_flush_merges<D, MC>(
    &self,
    merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&crate::core::index::index_writer::Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    // for test_doc_count()
    if !self.merge_fully_deleted_on_full_flush {
      return self
        .in_
        .find_full_flush_merges(merge_trigger, segment_infos, inner, merge_context);
    }
    // for test_merge_on_commit_keep_fully_deleted_segments()
    let mut fully_deleted_segments = Vec::new();
    for sci in segment_infos.iter() {
      let max_doc = sci.info.max_doc()?;
      if max_doc - sci.get_del_count() == 0 {
        fully_deleted_segments.push(SegmentDocAndID::new(
          sci.info.get_id_key().to_string(),
          max_doc,
        ));
      }
    }

    if fully_deleted_segments.is_empty() {
      return Ok(None);
    }

    let mut spec = MergeSpecificationNoReader::new();
    spec.add(OneMerge::new(fully_deleted_segments)?);
    Ok(Some(spec))
  }

  fn use_compound_file<D, MC>(
    &self,
    infos: &SegmentInfos<D>,
    merged_info: &SegmentCommitInfo<D>,
    merge_context: &MC,
  ) -> Result<bool>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self
      .in_
      .use_compound_file(infos, merged_info, merge_context)
  }

  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self.in_.size(info, merge_context)
  }

  fn keep_fully_deleted_segment<D, F>(&self, _reader_supplier: F) -> Result<bool>
  where
    D: Directory,
    F: Fn() -> Result<Arc<SegmentReader<D>>>,
  {
    Ok(true)
  }
}

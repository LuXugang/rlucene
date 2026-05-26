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
use crate::core::analysis::reader::StringReader;
use crate::core::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::document::Document;
use crate::core::document::field::{Field, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::fields::FieldTokenStreamEnum;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::document::stored_field::StoredField;
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::{TextField, text_field_type};
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader;
use crate::core::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::fields::Fields;
use crate::core::index::flush_policy::ApplyDeletesFlushPolicy;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
#[cfg(feature = "nightly")]
use crate::core::index::index_writer::MAX_STORED_STRING_LENGTH;
use crate::core::index::index_writer::{
  EmptyIndexWriterBase, EventEnum, EventImplTest, EventQueue, IndexWriter, IndexWriterBase,
  WRITE_LOCK_NAME, read_field_infos,
};
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::index_writer_config::{DISABLE_AUTO_FLUSH, IndexWriterConfig};
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::lockable_concurrent_approximate_priority_queue::Lock;
use crate::core::index::merge_policy::{
  MergeContext, MergePolicy, MergePolicyBase, MergePolicyEnum, MergeSpecification,
  MergeSpecificationNoReader, OneMerge,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::postings_enum::{ALL, FREQS, NONE, PostingsEnum};
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::index::term_vectors::TermVectors;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::index::tiered_merge_policy::SegmentDocAndID;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::index::{BytesRef, CODEC_FILE_PATTERN, IndexFileNames};
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::search::term_query::TermQuery;
use crate::core::store::IndexOutput;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::store::{DataOutput, IOContext};
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::bits::Bits;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{LATEST, StringHelper};
use crate::test::core::analysis::canned_token_stream::CannedTokenStream;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::analysis::mock_tokenizer::{MockTokenizer, WHITESPACE};
use crate::test::core::analysis::token;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::store::base_directory_test_case::EXTRA_FILE_NAME;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  create_temp_dir, get_only_leaf_reader, new_directory_shared, new_field, new_fs_directory,
  new_index_writer_config, new_index_writer_config_with_analyzer, new_io_context,
  new_log_merge_policy, new_log_merge_policy_with_merge_factor, new_searcher_with_reader,
  new_string_field, new_text_field, random, random_from_seed, rarely, slow_file_exists,
};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
use rand_xoshiro::rand_core::Rng;
use std::clone::Clone;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64};
use std::sync::{Arc, Barrier, LazyLock};
use std::thread;
use std::vec;

static STORED_TEXT_TYPE: LazyLock<FieldType> = LazyLock::new(|| {
  FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED).expect("should not fail")
});
#[allow(dead_code)]
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
pub(crate) fn add_doc<D, B, R>(
  random: &mut R,
  writer: &IndexWriter<D, B>,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory,
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
pub(crate) fn add_doc_with_index<D, B, R>(
  random: &mut R,
  writer: &IndexWriter<D, B>,
  index: i32,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory,
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

pub(crate) fn assert_no_unreferenced_files<D>(dir: Arc<D>, message: &str) -> Result<()>
where
  D: Directory,
{
  let mut start_files = dir.list_all()?;
  let mut random = random();
  let mock = MockAnalyzer::new(&mut random);
  let writer = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, mock),
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

#[test]
fn test_create_with_reader() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let config = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), config)?;
  let mut field_types = HashMap::new();
  add_doc(&mut random, &writer, &mut field_types)?;
  writer.close()?;
  drop(writer);

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(1, reader.num_docs()?);

  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config.set_open_mode(OpenMode::Create);

  let writer = IndexWriter::new(dir.clone(), config)?;
  assert_eq!(0, writer.get_doc_stats()?.max_doc);

  add_doc(&mut random, &writer, &mut field_types)?;
  writer.close()?;

  assert_eq!(1, reader.num_docs()?);

  let reader2 = directory_reader::open(dir)?;
  assert_eq!(1, reader2.num_docs()?);

  reader.close()?;
  reader2.close()?;

  Ok(())
}

#[test]
fn test_changes_after_close() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let config = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir, config)?;

  let mut field_types = HashMap::new();
  add_doc(&mut random, &writer, &mut field_types)?;

  writer.close()?;
  let err = add_doc(&mut random, &writer, &mut field_types);
  assert!(matches!(err, Err(LuceneError::AlreadyClosed(_))));

  Ok(())
}

#[test]
fn test_index_no_documents() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let config = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), config)?;
  writer.commit()?;
  writer.close()?;
  drop(writer);

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(0, reader.max_doc()?);
  assert_eq!(0, reader.num_docs()?);
  reader.close()?;

  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config.set_open_mode(OpenMode::Append);
  let writer = IndexWriter::new(dir.clone(), config)?;
  writer.commit()?;
  writer.close()?;

  let reader = directory_reader::open(dir)?;
  assert_eq!(0, reader.max_doc()?);
  assert_eq!(0, reader.num_docs()?);
  reader.close()?;

  Ok(())
}

#[test]
fn test_small_ram_buffer() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config
    .set_ram_buffer_size_mb(0.000001)
    .set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 10)?);
  let writer = IndexWriter::new(dir.clone(), config)?;
  let mut field_types = HashMap::new();

  let mut _last_num_segments = get_segment_count(dir.clone())?;
  for j in 0..9 {
    let mut doc = Document::new();
    doc.add(new_field(
      &mut random,
      "field",
      format!("aaa{j}"),
      &STORED_TEXT_TYPE,
      &mut field_types,
    )?);
    writer.add_document(doc)?;
    // Verify that with a tiny RAM buffer we see new segment after every doc
    let num_segments = get_segment_count(dir.clone())?;
    // TODO: memory calculation not implement
    // assert!(num_segments > last_num_segments);
    _last_num_segments = num_segments;
  }
  writer.close()?;
  Ok(())
}

/** Returns how many unique segment names are in the directory. */
fn get_segment_count<D>(dir: Arc<D>) -> Result<usize>
where
  D: Directory,
{
  let mut segments = HashSet::new();
  for file in dir.list_all()? {
    segments.insert(IndexFileNames::parse_segment_name(&file).to_string());
  }
  Ok(segments.len())
}

#[test]
fn test_changing_ram_buffer() -> Result<()> {
  // TODO: memory calculation not implement
  // let mut random = random();
  // let dir = new_directory_shared(&mut random)?;
  // let mock = MockAnalyzer::new(&mut random);
  // let mut writer = IndexWriter::new(
  //   dir.clone(),
  //   new_index_writer_config_with_analyzer(&mut random, mock),
  // )?;
  // writer.get_config_mut().set_max_buffered_docs(10);
  // writer
  //   .get_config_mut()
  //   .set_ram_buffer_size_mb(DISABLE_AUTO_FLUSH as f64);
  // let mut field_types = HashMap::new();
  //
  // let mut last_flush_count = -1;
  // for j in 1..52 {
  //   let mut doc = Document::new();
  //   doc.add(new_field(
  //     &mut random,
  //     "field",
  //     format!("aaa{j}"),
  //     &STORED_TEXT_TYPE,
  //     &mut field_types,
  //   )?);
  //   writer.add_document(doc)?;
  //   // TODO IMPORTANT TestUtil.syncConcurrentMerges未实现
  //   let flush_count = writer.get_flush_count();
  //   if j == 1 {
  //     last_flush_count = flush_count;
  //   } else if j < 10 {
  //     // No new files should be created
  //     assert_eq!(flush_count, last_flush_count);
  //   } else if j == 10 {
  //     assert!(flush_count > last_flush_count);
  //     last_flush_count = flush_count;
  //     writer.get_config_mut().set_ram_buffer_size_mb(0.000001);
  //     writer
  //       .get_config_mut()
  //       .set_max_buffered_docs(DISABLE_AUTO_FLUSH);
  //   } else if j < 20 {
  //     assert!(flush_count > last_flush_count);
  //     last_flush_count = flush_count;
  //   } else if j == 20 {
  //     writer.get_config_mut().set_ram_buffer_size_mb(16.0);
  //     writer
  //       .get_config_mut()
  //       .set_max_buffered_docs(DISABLE_AUTO_FLUSH);
  //     last_flush_count = flush_count;
  //   } else if j < 30 {
  //     assert_eq!(flush_count, last_flush_count);
  //   } else if j == 30 {
  //     writer.get_config_mut().set_ram_buffer_size_mb(0.000001);
  //     writer
  //       .get_config_mut()
  //       .set_max_buffered_docs(DISABLE_AUTO_FLUSH);
  //   } else if j < 40 {
  //     assert!(flush_count > last_flush_count);
  //     last_flush_count = flush_count;
  //   } else if j == 40 {
  //     writer.get_config_mut().set_max_buffered_docs(10);
  //     writer
  //       .get_config_mut()
  //       .set_ram_buffer_size_mb(DISABLE_AUTO_FLUSH as f64);
  //     last_flush_count = flush_count;
  //   } else if j < 50 {
  //     assert_eq!(flush_count, last_flush_count);
  //     writer.get_config_mut().set_max_buffered_docs(10);
  //     writer
  //       .get_config_mut()
  //       .set_ram_buffer_size_mb(DISABLE_AUTO_FLUSH as f64);
  //   } else if j == 50 {
  //     assert!(flush_count > last_flush_count);
  //   }
  // }
  // writer.close()?;
  Ok(())
}

#[test]
fn test_enabling_norms() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config.set_max_buffered_docs(10);
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type.set_omit_norms(true)?;
  let mut field_types = HashMap::new();
  for j in 0..10 {
    let mut doc = Document::new();
    let f = if j != 8 {
      new_field(&mut random, "field", "aaa", &custom_type, &mut field_types)?
    } else {
      new_field(
        &mut random,
        "field",
        "aaa",
        &STORED_TEXT_TYPE,
        &mut field_types,
      )?
    };
    doc.add(f);
    writer.add_document(doc)?;
  }
  writer.close()?;
  drop(writer);

  let search_term = Term::from_text("field", "aaa");

  let reader = directory_reader::open(dir.clone())?;
  let searcher = new_searcher_with_reader(reader)?;
  let hits = searcher.search(TermQuery::new(search_term.clone()), 1000)?;
  assert_eq!(10, hits.score_docs.len());

  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config
    .set_open_mode(OpenMode::Create)
    .set_max_buffered_docs(10);
  let writer = IndexWriter::new(dir.clone(), config)?;

  for j in 0..27 {
    let mut doc = Document::new();
    let f = if j != 26 {
      new_field(&mut random, "field", "aaa", &custom_type, &mut field_types)?
    } else {
      new_field(
        &mut random,
        "field",
        "aaa",
        &STORED_TEXT_TYPE,
        &mut field_types,
      )?
    };
    doc.add(f);
    writer.add_document(doc)?;
  }
  writer.close()?;

  let reader = directory_reader::open(dir.clone())?;
  let searcher = new_searcher_with_reader(reader)?;
  let hits = searcher.search(TermQuery::new(search_term), 1000)?;
  assert_eq!(27, hits.score_docs.len());

  let reader = directory_reader::open(dir)?;
  reader.close()?;

  Ok(())
}

#[test]
fn test_high_freq_term() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config.set_ram_buffer_size_mb(0.01);
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut b = String::with_capacity(1024 * 1024);
  for _ in 0..4096 {
    b.push_str(" a a a a a a a a");
    b.push_str(" a a a a a a a a");
    b.push_str(" a a a a a a a a");
    b.push_str(" a a a a a a a a");
  }
  let mut doc = Document::new();
  let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type.set_store_term_vectors(true)?;
  custom_type.set_store_term_vector_positions(true)?;
  custom_type.set_store_term_vector_offsets(true)?;
  doc.add(Field::new("field", b, custom_type));
  writer.add_document(doc)?;
  writer.close()?;

  let reader = directory_reader::open(dir)?;
  assert_eq!(1, reader.max_doc()?);
  assert_eq!(1, reader.num_docs()?);
  let t = Term::from_text("field", "a");
  assert_eq!(1, reader.doc_freq(&t)?);
  let mut td = TestUtil::docs_with_reader(
    &mut random,
    &reader,
    "field",
    &BytesRef::from_string("a"),
    None,
    FREQS as i32,
  )?
  .expect("term should exist");
  td.next_doc()?;
  assert_eq!(128 * 1024, td.freq()?);
  reader.close()?;

  Ok(())
}

#[test]
fn test_flush_with_no_merging() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config
    .set_max_buffered_docs(2)
    .set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 10)?);
  let writer = IndexWriter::new(dir, config)?;

  let mut doc = Document::new();
  let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type.set_store_term_vectors(true)?;
  custom_type.set_store_term_vector_positions(true)?;
  custom_type.set_store_term_vector_offsets(true)?;
  doc.add(Field::new("field", "aaa", custom_type));
  for _ in 0..19 {
    writer.add_document(doc.clone())?;
  }
  writer.flush_with_apply_merge_deletes(false, true)?;
  assert_eq!(10, writer.get_segment_count());
  writer.close()?;

  Ok(())
}

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
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_variable_schema() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  for i in 0..20 {
    let mock = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    iwc.set_max_buffered_docs(2);
    iwc.set_merge_policy(new_log_merge_policy(&mut random)?);

    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    let contents = "aa bb cc dd ee ff gg hh ii jj kk";

    let custom_type = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;

    if i == 7 {
      doc.add(new_text_field(
        &mut random,
        "content3",
        "",
        Store::No,
        &mut field_types,
      )?);
    } else {
      let field_type = if i % 2 == 0 {
        doc.add(new_field(
          &mut random,
          "content4",
          contents,
          &custom_type,
          &mut field_types,
        )?);
        custom_type.clone()
      } else {
        FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?
      };

      doc.add(new_text_field(
        &mut random,
        "content1",
        contents,
        Store::No,
        &mut field_types,
      )?);

      doc.add(new_field(
        &mut random,
        "content3",
        "",
        &custom_type,
        &mut field_types,
      )?);

      doc.add(new_field(
        &mut random,
        "content5",
        "",
        &field_type,
        &mut field_types,
      )?);
    }

    for _ in 0..4 {
      writer.add_document(doc.clone())?;
    }

    writer.close()?;
    drop(writer);

    if i % 4 == 0 {
      let mock = MockAnalyzer::new(&mut random);
      let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
      let writer = IndexWriter::new(dir.clone(), iwc)?;
      writer.force_merge(1)?;
      writer.close()?;
    }
  }

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

  let terms = LeafReader::terms(&subreader, "")?.unwrap();
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

  let terms = LeafReader::terms(&subreader, "")?.unwrap();
  let mut te = terms.iterator()?;

  assert_eq!(&BytesRef::from_string(""), te.next()?.unwrap().as_ref());
  assert_eq!(&BytesRef::from_string("a"), te.next()?.unwrap().as_ref());
  assert_eq!(&BytesRef::from_string("b"), te.next()?.unwrap().as_ref());
  assert_eq!(&BytesRef::from_string("c"), te.next()?.unwrap().as_ref());
  assert_eq!(None, te.next()?);

  Ok(())
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
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let writer = IndexWriter::new(dir, iwc)?;

  let mut doc = Document::new();
  doc.add(TextField::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(NegativePositionsTokenStream::new()),
  )?);

  let result = writer.add_document(doc);
  assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));

  writer.close()?;
  Ok(())
}

#[test]
fn test_position_increment_gap_empty_field() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  let mut analyzer = MockAnalyzer::new(&mut random);
  analyzer.set_position_increment_gap(100);

  let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  let mut field_types = HashMap::new();

  let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
  custom_type.set_store_term_vectors(true)?;
  custom_type.set_store_term_vector_positions(true)?;

  let mut doc = Document::new();
  doc.add(new_field(
    &mut random,
    "field",
    "",
    &custom_type,
    &mut field_types,
  )?);
  doc.add(new_field(
    &mut random,
    "field",
    "crunch man",
    &custom_type,
    &mut field_types,
  )?);

  w.add_document(doc)?;
  w.close()?;

  let r = directory_reader::open(dir)?;
  let mut term_vectors = r.term_vectors()?;
  let fields = term_vectors.get(0)?.unwrap();
  let tpv = fields.terms("field")?.unwrap();

  let mut terms_enum = tpv.iterator()?;

  assert!(terms_enum.next()?.is_some());
  let mut dp_enum = terms_enum.postings_with_flags(None, ALL as i32)?;
  assert_ne!(NO_MORE_DOCS, dp_enum.next_doc()?);
  assert_eq!(1, dp_enum.freq()?);
  assert_eq!(100, dp_enum.next_position()?);

  assert!(terms_enum.next()?.is_some());
  let mut dp_enum = terms_enum.postings_with_flags(Some(dp_enum), ALL as i32)?;
  assert_ne!(NO_MORE_DOCS, dp_enum.next_doc()?);
  assert_eq!(1, dp_enum.freq()?);
  assert_eq!(101, dp_enum.next_position()?);

  assert!(terms_enum.next()?.is_none());

  Ok(())
}

#[test]
fn test_deadlock() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_max_buffered_docs(2);

  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let mut field_types = HashMap::new();

  let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type.set_store_term_vectors(true)?;
  custom_type.set_store_term_vector_positions(true)?;
  custom_type.set_store_term_vector_offsets(true)?;

  let mut doc = Document::new();
  doc.add(new_field(
    &mut random,
    "content",
    "aaa bbb ccc ddd eee fff ggg hhh iii",
    &custom_type,
    &mut field_types,
  )?);

  writer.add_document(doc.clone())?;
  writer.add_document(doc.clone())?;
  writer.add_document(doc.clone())?;
  writer.commit()?;

  let dir2 = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer2 = IndexWriter::new(dir2.clone(), iwc)?;
  writer2.add_document(doc)?;
  writer2.close()?;

  let _r1 = directory_reader::open(dir2.clone())?;
  // TODO add_indexes_slowly未实现
  // TestUtil::add_indexes_slowly(&mut writer, &r1, &r1)?;
  // writer.close()?;
  //
  // let r3 = directory_reader::open(dir.clone())?;
  // assert_eq!(5, r3.num_docs()?);

  Ok(())
}

#[test]
fn test_thread_interrupt_deadlock() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_index_store_combos() -> Result<()> {
  let mut rng = random();
  let dir = new_directory_shared(&mut rng)?;
  let mock = MockAnalyzer::new(&mut rng);
  let iwc = new_index_writer_config_with_analyzer(&mut rng, mock);
  let writer = IndexWriter::new(dir.clone(), iwc)?;
  let b: Vec<u8> = (0..50).map(|i| i + 77).collect();

  let mut custom_type = FieldType::new();
  custom_type.set_tokenized(true)?;
  custom_type.set_index_options(IndexOptions::Docs)?;

  let custom_type2 = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;

  let r = random_from_seed(rng.random());
  let mut field1 = MockTokenizer::with_default_max_token_length(r, WHITESPACE.clone(), false);
  field1.set_reader(StringReader::new("doc1field1").into())?;
  let r = random_from_seed(rng.random());
  let mut field2 = MockTokenizer::with_default_max_token_length(r, WHITESPACE.clone(), false);
  field2.set_reader(StringReader::new("doc1field2").into())?;
  let mut doc = Document::new();
  doc.add(StoredField::from_binary_with_range(
    "binary",
    b.clone(),
    10,
    17,
  )?);
  doc.add(Field::from_token_stream(
    "binary",
    FieldTokenStreamEnum::custom(field1),
    custom_type.clone(),
  )?);
  doc.add(Field::from_string(
    "string",
    "value",
    FieldType::from_ref(&*text_field_type::TYPE_STORED)?,
  )?);
  doc.add(Field::from_token_stream(
    "string",
    FieldTokenStreamEnum::custom(field2),
    custom_type2.clone(),
  )?);
  writer.add_document(doc)?;

  let r = random_from_seed(rng.random());
  let mut field1 = MockTokenizer::with_default_max_token_length(r, WHITESPACE.clone(), false);
  field1.set_reader(StringReader::new("doc2field1").into())?;
  let r = random_from_seed(rng.random());
  let mut field2 = MockTokenizer::with_default_max_token_length(r, WHITESPACE.clone(), false);
  field2.set_reader(StringReader::new("doc2field2").into())?;
  let mut doc = Document::new();
  doc.add(StoredField::from_binary_with_range(
    "binary",
    b.clone(),
    10,
    17,
  )?);
  doc.add(Field::from_token_stream(
    "binary",
    FieldTokenStreamEnum::custom(field1),
    custom_type.clone(),
  )?);
  doc.add(Field::from_string(
    "string",
    "value",
    FieldType::from_ref(&*text_field_type::TYPE_STORED)?,
  )?);
  doc.add(Field::from_token_stream(
    "string",
    FieldTokenStreamEnum::custom(field2),
    custom_type2.clone(),
  )?);
  writer.add_document(doc)?;

  writer.commit()?;
  let r = random_from_seed(rng.random());
  let mut field1 = MockTokenizer::with_default_max_token_length(r, WHITESPACE.clone(), false);
  field1.set_reader(StringReader::new("doc3field1").into())?;
  let r = random_from_seed(rng.random());
  let mut field2 = MockTokenizer::with_default_max_token_length(r, WHITESPACE.clone(), false);
  field2.set_reader(StringReader::new("doc3field2").into())?;
  let mut doc = Document::new();
  doc.add(StoredField::from_binary_with_range(
    "binary",
    b.clone(),
    10,
    17,
  )?);
  doc.add(Field::from_token_stream(
    "binary",
    FieldTokenStreamEnum::custom(field1),
    custom_type,
  )?);
  doc.add(Field::from_string(
    "string",
    "value",
    FieldType::from_ref(&*text_field_type::TYPE_STORED)?,
  )?);
  doc.add(Field::from_token_stream(
    "string",
    FieldTokenStreamEnum::custom(field2),
    custom_type2,
  )?);
  writer.add_document(doc)?;
  writer.commit()?;
  writer.force_merge(1)?;
  writer.close()?;

  let reader = directory_reader::open(dir)?;
  let mut stored_fields = reader.stored_fields()?;
  let doc2 = stored_fields.document(0)?;
  let f3 = doc2.get_field("binary").expect("binary field should exist");
  let b = f3.binary_value()?.expect("binary value should exist");
  assert_eq!(17, b.length);
  assert_eq!(87, b.bytes[b.offset]);

  for doc_id in 0..3 {
    assert!(
      stored_fields
        .document(doc_id)?
        .get_field("binary")
        .expect("binary field should exist")
        .binary_value()?
        .is_some()
    );
  }

  assert_eq!(
    "value",
    stored_fields.document(0)?.get("string")?.unwrap().as_ref()
  );
  assert_eq!(
    "value",
    stored_fields.document(1)?.get("string")?.unwrap().as_ref()
  );
  assert_eq!(
    "value",
    stored_fields.document(2)?.get("string")?.unwrap().as_ref()
  );

  assert_ne!(
    TestUtil::docs_with_reader(
      &mut rng,
      &reader,
      "binary",
      &BytesRef::from_string("doc1field1"),
      None,
      NONE as i32,
    )?
    .expect("term should exist")
    .next_doc()?,
    NO_MORE_DOCS
  );
  assert_ne!(
    TestUtil::docs_with_reader(
      &mut rng,
      &reader,
      "binary",
      &BytesRef::from_string("doc2field1"),
      None,
      NONE as i32,
    )?
    .expect("term should exist")
    .next_doc()?,
    NO_MORE_DOCS
  );
  assert_ne!(
    TestUtil::docs_with_reader(
      &mut rng,
      &reader,
      "binary",
      &BytesRef::from_string("doc3field1"),
      None,
      NONE as i32,
    )?
    .expect("term should exist")
    .next_doc()?,
    NO_MORE_DOCS
  );
  assert_ne!(
    TestUtil::docs_with_reader(
      &mut rng,
      &reader,
      "string",
      &BytesRef::from_string("doc1field2"),
      None,
      NONE as i32,
    )?
    .expect("term should exist")
    .next_doc()?,
    NO_MORE_DOCS
  );
  assert_ne!(
    TestUtil::docs_with_reader(
      &mut rng,
      &reader,
      "string",
      &BytesRef::from_string("doc2field2"),
      None,
      NONE as i32,
    )?
    .expect("term should exist")
    .next_doc()?,
    NO_MORE_DOCS
  );
  assert_ne!(
    TestUtil::docs_with_reader(
      &mut rng,
      &reader,
      "string",
      &BytesRef::from_string("doc3field2"),
      None,
      NONE as i32,
    )?
    .expect("term should exist")
    .next_doc()?,
    NO_MORE_DOCS
  );

  reader.close()?;
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
  // TODO WindowsFS未实现
  Ok(())
}

#[test]
fn test_delete_unused_files2() -> Result<()> {
  // TODO WindowsFS未实现
  Ok(())
}

#[test]
fn test_empty_fs_dir_with_no_lock() -> Result<()> {
  // TODO NoLockFactory未实现
  Ok(())
}

#[test]
fn test_empty_dir_rollback() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let orig_files = dir.list_all()?;
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config.set_max_buffered_docs(2);
  config.set_merge_policy(new_log_merge_policy(&mut random)?);
  config.set_use_compound_file(false);
  let writer = IndexWriter::new(dir.clone(), config)?;
  let mut files = dir.list_all()?;

  let extra_file_count = files.len() - orig_files.len();
  if extra_file_count == 1 {
    assert!(files.contains(&WRITE_LOCK_NAME.to_string()));
  } else {
    let mut sorted_orig_files = orig_files.clone();
    sorted_orig_files.sort();
    files.sort();
    assert_eq!(sorted_orig_files, files);
  }

  let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type.set_store_term_vectors(true)?;
  custom_type.set_store_term_vector_positions(true)?;
  custom_type.set_store_term_vector_offsets(true)?;

  let mut field_types = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_field(
    &mut random,
    "c",
    "val",
    &custom_type,
    &mut field_types,
  )?);
  writer.add_document(doc)?;

  let mut computed_extra_file_count = 0;
  for file in dir.list_all()? {
    if file == WRITE_LOCK_NAME
      || file.starts_with(IndexFileNames::SEGMENTS)
      || CODEC_FILE_PATTERN.is_match(&file)
    {
      let should_count = match file.rsplit_once('.') {
        None => true,
        Some((_, ext)) => !matches!(ext, "fdm" | "fdt" | "tvm" | "tvd" | "tmp"),
      };
      if should_count {
        computed_extra_file_count += 1;
      }
    }
  }
  assert_eq!(
    extra_file_count, computed_extra_file_count,
    "only the stored and term vector files should exist in the directory"
  );

  let mut doc = Document::new();
  doc.add(new_field(
    &mut random,
    "c",
    "val",
    &custom_type,
    &mut field_types,
  )?);
  writer.add_document(doc)?;

  assert!(
    dir.list_all()?.len() > 5 + extra_file_count,
    "flush should have occurred and files should have been created"
  );

  writer.rollback()?;
  let all_files = dir.list_all()?;
  assert_eq!(
    orig_files.len() + extra_file_count,
    all_files.len(),
    "no files should exist in the directory after rollback"
  );

  writer.close()?;
  let all_files = dir.list_all()?;
  assert_eq!(
    orig_files.len() + extra_file_count,
    all_files.len(),
    "expected a no-op close after IW.rollback()"
  );

  Ok(())
}

#[test]
fn test_no_unwanted_tv_files() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config.set_ram_buffer_size_mb(0.01);
  let mut merge_policy = new_log_merge_policy(&mut random)?;
  merge_policy.get_base_mut().set_no_cfs_ratio(0.0)?;
  config.set_merge_policy(merge_policy);
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut big =
    "alskjhlaksjghlaksjfhalksvjepgjioefgjnsdfjgefgjhelkgjhqewlrkhgwlekgrhwelkgjhwelkgrhwlkejg"
      .to_string();
  big = big.repeat(4);

  let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type.set_omit_norms(true)?;
  let mut custom_type2 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type2.set_tokenized(false)?;
  let mut custom_type3 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type3.set_tokenized(false)?;
  custom_type3.set_omit_norms(true)?;

  for i in 0..2 {
    let text = format!("{i}{big}");
    let mut doc = Document::new();
    doc.add(Field::from_string(
      "id",
      text.clone(),
      custom_type3.clone(),
    )?);
    doc.add(Field::from_string(
      "str",
      text.clone(),
      custom_type2.clone(),
    )?);
    doc.add(Field::from_string(
      "str2",
      text.clone(),
      STORED_TEXT_TYPE.clone(),
    )?);
    doc.add(Field::from_string("str3", text, custom_type.clone())?);
    writer.add_document(doc)?;
  }

  writer.close()?;
  drop(writer);

  TestUtil::check_index(dir.clone())?;

  assert_no_unreferenced_files(dir.clone(), "no tv files")?;

  let reader = directory_reader::open(dir)?;
  let context = get_context(&reader)?;
  for ctx in context.leaves()? {
    assert!(!ctx.reader().get_field_infos()?.has_term_vectors());
  }

  reader.close()?;
  Ok(())
}

#[test]
fn test_wicked_long_term() -> Result<()> {
  // TODO StringSplitAnalyzer未实现
  Ok(())
}

// TODO IMPORTANT IndexReader#do_close需要重新设计
fn test_delete_all_nrt_leftover_files() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  let doc = Document::new();

  for _ in 0..20 {
    for _ in 0..100 {
      w.add_document(doc.clone())?;
    }

    w.commit()?;

    let reader = directory_reader::open_from_writer(&w)?;
    reader.close()?;

    w.delete_all()?;
    w.commit()?;

    // Make sure we accumulate no files except for empty segments_N and segments.gen.
    let files = dir.list_all()?;
    assert!(files.len() <= 2, "unexpected leftover files: {files:?}");
  }

  w.close()?;

  Ok(())
}
#[test]
fn test_nrt_reader_version() -> Result<()> {
  // TODO OpenIFchange未实现
  // let mut random = random();
  //
  // let dir = new_directory_shared(&mut random)?;
  //
  // let mock = MockAnalyzer::new(&mut random);
  // let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  // let mut w = IndexWriter::new(dir.clone(), iwc)?;
  //
  // let mut field_types = HashMap::new();
  //
  // let mut doc = Document::new();
  // doc.add(new_string_field(
  //   &mut random,
  //   "id",
  //   "0",
  //   Store::Yes,
  //   &mut field_types,
  // )?);
  //
  // w.add_document(doc.clone())?;
  //
  // let r = directory_reader::open_from_writer(&w)?;
  // let version = r.get_version();
  // drop(r);
  //
  // w.add_document(doc.clone())?;
  //
  // let r = directory_reader::open_from_writer(&w)?;
  // let version2 = r.get_version();
  // drop(r);
  //
  // assert!(version2 > version);
  //
  // w.delete_documents_with_terms(vec![Term::from_text("id", "0")])?;
  //
  // let r = directory_reader::open_from_writer(&w)?;
  // w.close()?;
  //
  // let version3 = r.get_version();
  // drop(r);
  //
  // assert!(version3 > version2);

  Ok(())
}

#[test]
fn test_whether_delete_all_deletes_write_lock() -> Result<()> {
  // TODO IMPORTANT SimpleFSLockFactory未实现
  Ok(())
}

#[test]
fn test_has_blocks_merge_fully_del_segments() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let new_doc = || -> Result<Document> {
    let mut doc = Document::new();
    doc.add(StringField::from_string("foo", "bar", Store::No)?);
    Ok(doc)
  };

  let docs = vec![new_doc()?, new_doc()?];
  writer.update_documents_with_term(Term::from_text("foo", "bar"), docs.clone())?;
  writer.commit()?;

  if random.random_bool(0.5) {
    writer.update_documents_with_term(Term::from_text("foo", "bar"), docs)?;
    writer.commit()?;
  }

  writer.update_document_with_term(Term::from_text("foo", "bar"), new_doc()?)?;

  if random.random_bool(0.5) {
    writer.force_merge_deletes_with_wait(true)?;
  } else {
    writer.force_merge_with_wait(1, true)?;
  }

  writer.commit()?;

  let reader = directory_reader::open(dir.clone())?;
  let reader = get_context(reader)?;
  let leaves = reader.leaves()?;
  assert_eq!(1, leaves.len());

  assert!(
    !leaves[0].reader().get_metadata()?.get_has_blocks(),
    "hasBlocks should be cleared"
  );

  writer.close()?;

  Ok(())
}

#[test]
fn test_single_docs_do_not_trigger_has_blocks() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_max_buffered_docs(i32::MAX);
  iwc.set_ram_buffer_size_mb(100.0);

  let w = IndexWriter::new(dir.clone(), iwc)?;

  let docs = TestUtil::next_int(&mut random, 1, 100);
  for i in 0..docs {
    let mut doc = Document::new();
    doc.add(StringField::from_string("id", i.to_string(), Store::No)?);
    w.add_documents(vec![doc])?;
  }

  w.commit()?;

  let si = w.clone_segment_infos()?;
  assert_eq!(1, si.size());
  assert!(!si.iter()[0].info.get_has_blocks());

  let mut doc = Document::new();
  doc.add(StringField::from_string("id", "XXX", Store::No)?);

  w.add_documents(vec![doc.clone(), doc])?;
  w.commit()?;

  let si = w.clone_segment_infos()?;
  assert_eq!(2, si.size());

  let infos = si.iter();
  assert!(!infos[0].info.get_has_blocks());
  assert!(infos[1].info.get_has_blocks());

  w.force_merge(1)?;
  w.commit()?;

  let si = w.clone_segment_infos()?;
  assert_eq!(1, si.size());
  assert!(si.iter()[0].info.get_has_blocks());

  w.close()?;

  Ok(())
}

#[test]
fn test_carry_over_has_blocks() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  let mut docs = vec![Document::new()];
  w.update_documents_with_term(Term::from_text("foo", "bar"), docs.clone())?;
  w.commit()?;

  {
    let reader = directory_reader::open(dir.clone())?;
    let reader = get_context(reader)?;
    let leaves = reader.leaves()?;
    let segment_info = leaves[0].reader().get_segment_info();
    assert!(!segment_info.info.get_has_blocks());
  }

  docs.push(Document::new());

  w.update_documents_with_term(Term::from_text("foo", "bar"), docs)?;
  w.commit()?;

  {
    let reader = directory_reader::open(dir.clone())?;
    let reader = get_context(reader)?;
    let leaves = reader.leaves()?;
    assert_eq!(2, leaves.len());

    let segment_info = leaves[0].reader().get_segment_info();
    assert!(!segment_info.info.get_has_blocks(),);

    let segment_info = leaves[1].reader().get_segment_info();
    assert!(segment_info.info.get_has_blocks(),);
  }

  w.force_merge_with_wait(1, true)?;
  w.commit()?;

  {
    let reader = directory_reader::open(dir.clone())?;
    let reader = get_context(reader)?;
    let leaves = reader.leaves()?;
    assert_eq!(1, leaves.len());

    let segment_info = leaves[0].reader().get_segment_info();
    assert!(segment_info.info.get_has_blocks(),);
  }

  w.commit()?;
  w.close()?;

  Ok(())
}

#[test]
fn test_prepare_commit_then_close() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  w.prepare_commit()?;

  let err = w.close();
  assert!(matches!(err, Err(LuceneError::IllegalState(_))));

  w.commit()?;
  w.close()?;
  drop(w);

  let r = directory_reader::open(dir)?;
  assert_eq!(0, r.max_doc()?);

  Ok(())
}

#[test]
fn test_prepare_commit_then_rollback() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let conf = new_index_writer_config_with_analyzer(&mut random, mock);
  let w = IndexWriter::new(dir.clone(), conf)?;

  w.prepare_commit()?;
  w.rollback()?;

  assert!(!directory_reader::index_exists(&dir)?);

  Ok(())
}

#[test]
fn test_prepare_commit_then_rollback2() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let conf = new_index_writer_config_with_analyzer(&mut random, mock);
  let w = IndexWriter::new(dir.clone(), conf)?;

  w.commit()?;
  w.add_document(Document::new())?;
  w.prepare_commit()?;
  w.rollback()?;

  assert!(directory_reader::index_exists(&dir)?);

  let r = directory_reader::open(dir.clone())?;
  assert_eq!(0, r.max_doc()?);

  r.close()?;

  Ok(())
}

#[test]
fn test_dont_invoke_analyzer_for_un_analyzed_fields() -> Result<()> {
  // TODO IMPORTANT  自定义分词器有 bug
  Ok(())
}

#[test]
fn test_other_files() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let iw = IndexWriter::new(dir.clone(), iwc)?;
  iw.add_document(Document::new())?;
  iw.close()?;
  drop(iw);

  {
    // Create my own random file.
    let context = new_io_context(&mut random)?;
    let mut out = dir.create_output("myrandomfile", &context)?;
    out.write_byte(42)?;
    out.close()?;
  }

  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let iw = IndexWriter::new(dir.clone(), iwc)?;
  iw.close()?;

  assert!(slow_file_exists(&dir, "myrandomfile")?);

  Ok(())
}

#[test]
fn test_stopwords_pos_inc_hole() -> Result<()> {
  // TODO IMPORTANT  自定义分词器有 bug
  Ok(())
}

#[test]
fn test_stopwords_pos_inc_hole2() -> Result<()> {
  // TODO IMPORTANT  自定义分词器有 bug
  Ok(())
}

#[test]
fn test_commit_with_user_data_only() -> Result<()> {
  // TODO IMPORTANT get_index_commit未实现
  // let mut random = random();
  //
  // let dir = new_directory_shared(&mut random)?;
  //
  // let iwc = new_index_writer_config(None);
  // let mut writer = IndexWriter::new(dir.clone(), iwc)?;
  //
  // writer.commit()?; // first commit to complete IW create transaction.
  //
  // // This should store the commit data, even though no other changes were made.
  // let mut data = HashMap::new();
  // data.insert("key".to_string(), "value".to_string());
  // writer.set_live_commit_data(data);
  // writer.commit()?;
  //
  // let r = directory_reader::open(dir.clone())?;
  // assert_eq!(
  //   Some(&"value".to_string()),
  //   r.get_index_commit()?.get_user_data().get("key")
  // );
  //
  // // Now check setCommitData and prepareCommit/commit sequence.
  // let mut data = HashMap::new();
  // data.insert("key".to_string(), "value1".to_string());
  // writer.set_live_commit_data(data);
  //
  // writer.prepare_commit()?;
  //
  // let mut data = HashMap::new();
  // data.insert("key".to_string(), "value2".to_string());
  // writer.set_live_commit_data(data);
  //
  // // Should commit the first commitData only, per protocol.
  // writer.commit()?;
  //
  // let r = directory_reader::open(dir.clone())?;
  // assert_eq!(
  //   Some(&"value1".to_string()),
  //   r.get_index_commit()?.get_user_data().get("key")
  // );
  //
  // // Now should commit the second commitData - there was a bug where
  // // IndexWriter.finishCommit overrode the second commitData.
  // writer.commit()?;
  //
  // let r = directory_reader::open(dir.clone())?;
  // assert_eq!(
  //   Some(&"value2".to_string()),
  //   r.get_index_commit()?.get_user_data().get("key"),
  //   "IndexWriter.finishCommit may have overridden the second commitData"
  // );
  //
  // writer.close()?;

  Ok(())
}

fn get_live_commit_data<D, B>(writer: &IndexWriter<D, B>) -> HashMap<String, String>
where
  D: Directory,
  B: IndexWriterBase,
{
  let mut data = HashMap::new();

  if let Some(iter) = writer.get_live_commit_data() {
    for ent in iter {
      data.insert(ent.0.clone(), ent.1.clone());
    }
  }

  data
}

#[test]
fn test_get_commit_data() -> Result<()> {
  let dir = new_directory_shared(&mut random())?;
  let mut random = random();

  let iwc = new_index_writer_config(&mut random);
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  writer.set_live_commit_data(HashMap::from([("key".to_string(), "value".to_string())]));

  assert_eq!(
    Some("value"),
    get_live_commit_data(&writer).get("key").map(String::as_str)
  );

  writer.close()?;
  drop(writer);

  // Validate that it's also visible when opening a new IndexWriter.
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_open_mode(OpenMode::Append);

  let writer = IndexWriter::new(dir.clone(), iwc)?;

  assert_eq!(
    Some("value"),
    get_live_commit_data(&writer).get("key").map(String::as_str)
  );

  writer.close()?;

  Ok(())
}

#[test]
fn test_get_commit_data_from_old_snapshot() -> Result<()> {
  // TODO IMPORTANT SnapshotDeletionPolicy未实现
  Ok(())
}

#[test]
fn test_null_analyzer() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_null_document() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_null_documents() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_iterable_field_throws_exception() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_iterable_throws_exception() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_iterable_throws_exception2() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_corrupt_first_commit() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_has_uncommitted_changes() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  assert!(writer.has_uncommitted_changes());

  let mut doc = Document::new();
  doc.add(TextField::from_string("myfield", "a b c", Store::No)?);
  writer.add_document(doc.clone())?;
  assert!(writer.has_uncommitted_changes());

  writer.commit()?;
  writer.wait_for_merges()?;
  writer.commit()?;
  assert!(!writer.has_uncommitted_changes());

  writer.add_document(doc)?;
  assert!(writer.has_uncommitted_changes());
  writer.commit()?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("id", "xyz", Store::Yes)?);
  writer.add_document(doc.clone())?;
  assert!(writer.has_uncommitted_changes());

  writer.commit()?;
  assert!(!writer.has_uncommitted_changes());
  writer.delete_documents_with_terms(vec![Term::from_text("id", "xyz")])?;
  assert!(writer.has_uncommitted_changes());

  writer.commit()?;
  assert!(!writer.has_uncommitted_changes());
  writer.close()?;
  drop(writer);

  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), iwc)?;
  assert!(!writer.has_uncommitted_changes());
  writer.add_document(doc)?;
  assert!(writer.has_uncommitted_changes());

  writer.close()?;

  Ok(())
}

#[test]
fn test_merge_all_deleted() -> Result<()> {
  // TODO
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

#[test]
fn test_has_uncommitted_changes_after_exception() -> Result<()> {
  let mut random = random();
  let analyzer = MockAnalyzer::new(&mut random);

  let directory = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let iwriter = IndexWriter::new(directory, iwc)?;

  let mut doc = Document::new();
  doc.add(SortedDocValuesField::new(
    "dv",
    BytesRef::from_string("foo!"),
  ));
  doc.add(SortedDocValuesField::new(
    "dv",
    BytesRef::from_string("bar!"),
  ));
  let result = iwriter.add_document(doc);
  assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));

  iwriter.commit()?;
  assert!(!iwriter.has_uncommitted_changes());
  iwriter.close()?;

  Ok(())
}

#[test]
fn test_double_close() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let w = IndexWriter::new(
    dir,
    new_index_writer_config_with_analyzer(&mut random, analyzer),
  )?;

  let mut doc = Document::new();
  doc.add(SortedDocValuesField::new(
    "dv",
    BytesRef::from_string("foo!"),
  ));
  w.add_document(doc)?;
  w.close()?;
  w.close()?;
  Ok(())
}

#[test]
fn test_rollback_then_close() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let w = IndexWriter::new(
    dir,
    new_index_writer_config_with_analyzer(&mut random, analyzer),
  )?;

  let mut doc = Document::new();
  doc.add(SortedDocValuesField::new(
    "dv",
    BytesRef::from_string("foo!"),
  ));
  w.add_document(doc)?;
  w.rollback()?;
  // Close after rollback should have no effect
  w.close()?;

  Ok(())
}

#[test]
fn test_close_then_rollback() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let w = IndexWriter::new(
    dir,
    new_index_writer_config_with_analyzer(&mut random, analyzer),
  )?;

  let mut doc = Document::new();
  doc.add(SortedDocValuesField::new(
    "dv",
    BytesRef::from_string("foo!"),
  ));
  w.add_document(doc)?;
  w.close()?;
  // Rollback after close should have no effect
  w.rollback()?;

  Ok(())
}

#[test]
fn test_close_while_merge_is_running() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_close_during_commit() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_ids() -> Result<()> {
  let mut random = random();
  let d = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let w = IndexWriter::new(
    d.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer),
  )?;
  w.add_document(Document::new())?;
  w.close()?;

  let sis = SegmentInfos::read_latest_commit(d.clone())?;
  let id1 = sis
    .get_id()
    .ok_or_else(|| LuceneError::illegal_state("missing segment infos id"))?;
  assert_eq!(StringHelper::ID_LENGTH, id1.len());

  let id2 = sis.info(0).unwrap().info.get_id();
  let sci_id2 = sis
    .info(0)
    .unwrap()
    .get_id()
    .ok_or_else(|| LuceneError::illegal_state("missing segment commit info id"))?;
  assert_eq!(StringHelper::ID_LENGTH, id2.len());
  assert_eq!(StringHelper::ID_LENGTH, sci_id2.len());
  // TODO IMPORTANT CheckIndex未实现
  //   TestUtil::check_index(d.clone())?;

  let id1 = StringHelper::id_to_string(Some(id1));
  assert_ne!("(null)", id1);

  let mut ids = HashSet::new();
  for i in 0..100000 {
    let id = StringHelper::id_to_string(Some(&StringHelper::random_id()));
    assert!(ids.insert(id.clone()), "id={} i={}", id, i);
  }

  Ok(())
}

#[test]
fn test_empty_norm() -> Result<()> {
  let mut random = random();
  let d = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let w = IndexWriter::new(
    d.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer),
  )?;
  let mut doc = Document::new();
  doc.add(TextField::from_token_stream(
    "foo",
    FieldTokenStreamEnum::custom(CannedTokenStream::new(Vec::new())),
  )?);
  w.add_document(doc)?;
  w.commit()?;
  w.close()?;

  let r = directory_reader::open(d)?;
  let leaf = get_only_leaf_reader(&r)?;
  let mut norms = LeafReader::get_norm_values(&leaf, "foo")?
    .ok_or_else(|| LuceneError::illegal_state("missing norms for field foo"))?;
  assert_eq!(0, norms.next_doc()?);
  assert_eq!(0, norms.long_value()?);
  r.close()?;

  Ok(())
}

#[test]
fn test_many_separate_threads() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_max_buffered_docs(1000);
  let writer = Arc::new(IndexWriter::new(dir.clone(), iwc)?);

  for _ in 0..100 {
    let writer = writer.clone();
    thread::scope(|scope| -> Result<()> {
      let handle = scope.spawn(move || -> Result<()> {
        let mut doc = Document::new();
        doc.add(StringField::from_string("foo", "bar", Store::No)?);
        writer.add_document(doc)?;
        Ok(())
      });
      handle.join().expect("thread panicked")?;
      Ok(())
    })?;
  }
  writer.close()?;

  let reader = directory_reader::open(dir)?;
  assert_eq!(1, get_context(&reader)?.leaves()?.len());
  reader.close()?;
  Ok(())
}

#[test]
fn test_nrt_segments_file() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_nrt_after_commit() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_nrt_after_set_user_data_without_commit() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_nrt_after_set_user_data_with_commit() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_commit_immediately_after_nrt_reopen() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_pending_delete_dv_generation() -> Result<()> {
  let mut random = random();
  let dir = new_fs_directory(&mut random, create_temp_dir()?)?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_use_compound_file(false);
  iwc.set_merge_policy(NoMergePolicy::default());
  iwc.set_max_buffered_docs(2);
  iwc.set_ram_buffer_size_mb(-1.0);
  let mut w = IndexWriter::new(dir.clone(), iwc)?;

  let mut d = Document::new();
  d.add(StringField::from_string("id", "1", Store::Yes)?);
  d.add(NumericDocValuesField::new("nvd", 1));
  w.add_document(d)?;

  let mut d = Document::new();
  d.add(StringField::from_string("id", "2", Store::Yes)?);
  d.add(NumericDocValuesField::new("nvd", 2));
  w.add_document(d)?;
  w.flush()?;

  let mut d = Document::new();
  d.add(StringField::from_string("id", "1", Store::Yes)?);
  d.add(NumericDocValuesField::new("nvd", 1));
  w.update_document_with_term(Term::from_text("id", "1"), d)?;
  w.commit()?;

  let files: HashSet<String> = dir.list_all()?.into_iter().collect();
  let num_iters = 10 + random.random_range(0..50);
  let mut to_close = Vec::new();
  for _ in 0..num_iters {
    if random.random_bool(0.5) {
      let mut d = Document::new();
      d.add(StringField::from_string("id", "1", Store::Yes)?);
      d.add(NumericDocValuesField::new("nvd", 1));
      w.update_document_with_term(Term::from_text("id", "1"), d)?;
    } else if random.random_bool(0.5) {
      w.delete_documents_with_terms(vec![Term::from_text("id", "2")])?;
    } else {
      w.update_numeric_doc_value(Term::from_text("id", "1"), "nvd", 2)?;
    }
    w.prepare_commit()?;
    let mut new_files = dir.list_all()?;
    new_files.retain(|file| !files.contains(file));
    let random_file = new_files[random.random_range(0..new_files.len())].clone();
    to_close.push(dir.open_input(&random_file, &IOContext::default_io_context()?)?);
    w.rollback()?;
    drop(w);

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    iwc.set_use_compound_file(false);
    iwc.set_merge_policy(NoMergePolicy::default());
    iwc.set_max_buffered_docs(2);
    iwc.set_ram_buffer_size_mb(-1.0);
    w = IndexWriter::new(dir.clone(), iwc)?;
    assert!(dir.delete_file(&random_file).is_err());
  }

  drop(to_close);
  w.close()?;

  Ok(())
}

#[test]
fn test_pending_deletions_rollback_with_reader() -> Result<()> {
  let mut random = random();
  let dir = new_fs_directory(&mut random, create_temp_dir()?)?;

  let analyzer = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let mut w = IndexWriter::new(dir.clone(), iwc)?;

  let mut d = Document::new();
  d.add(StringField::from_string("id", "1", Store::Yes)?);
  d.add(NumericDocValuesField::new("numval", 1));
  w.add_document(d.clone())?;
  w.commit()?;
  w.add_document(d.clone())?;
  w.flush()?;
  let reader = directory_reader::open_from_writer(&w)?;
  w.rollback()?;
  drop(w);

  // try-delete superfluous files (some will fail due to open readers)
  let analyzer = MockAnalyzer::new(&mut random);
  let iwc2 = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let writer = IndexWriter::new(dir.clone(), iwc2)?;
  writer.close()?;
  drop(writer);

  // test that we can index on top of pending deletions
  let analyzer = MockAnalyzer::new(&mut random);
  let iwc3 = new_index_writer_config_with_analyzer(&mut random, analyzer);
  w = IndexWriter::new(dir.clone(), iwc3)?;
  w.add_document(d)?;
  w.commit()?;

  reader.close()?;
  w.close()?;

  Ok(())
}

#[test]
fn test_with_pending_deletions() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_pending_deletes_already_written_files() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_leftover_temp_files() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let writer = IndexWriter::new(dir.clone(), iwc)?;
  writer.close()?;
  drop(writer);

  let io_context = IOContext::default_io_context()?;
  let temp_name = {
    let mut out = dir.create_temp_output("_0", "bkd", &io_context)?;
    let temp_name = out.get_name().to_string();
    out.close()?;
    temp_name
  };

  let analyzer = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  assert!(
    dir.open_input(&temp_name, &io_context).is_err(),
    "did not hit exception"
  );
  writer.close()?;
  Ok(())
}

#[cfg(feature = "nightly")]
#[test]
#[ignore = "nightly"]
fn test_massive_field() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let mut b = String::new();
  while b.len() <= MAX_STORED_STRING_LENGTH as usize {
    b.push_str("x ");
  }

  let mut doc = Document::new();
  doc.add(StoredField::from_string("big", b.clone())?);
  let err = writer.add_document(doc);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert_eq!(
    format!(
      "stored field \"big\" is too large ({} characters) to store",
      b.len()
    ),
    err.unwrap_err().to_string()
  );

  let mut doc2 = Document::new();
  doc2.add(StringField::from_string("id", "foo", Store::Yes)?);
  writer.add_document(doc2)?;

  let reader = writer.get_reader(true, true)?;
  assert_eq!(1, reader.num_docs()?);
  reader.close()?;
  writer.close()?;
  Ok(())
}

#[test]
fn test_records_index_created_version() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  writer.commit()?;
  writer.close()?;
  assert_eq!(
    LATEST.major,
    SegmentInfos::read_latest_commit(dir)?.get_index_created_version_major()
  );
  Ok(())
}
#[test]
fn test_flush_largest_writer() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  let iwc = IndexWriterConfig::new();
  let w = IndexWriter::new(dir.clone(), iwc)?;

  let num_docs = index_docs_for_multiple_dwpts(&w, &mut random)?;

  let largest_non_pending_writer = w
    .doc_writer
    .flush_control
    .find_largest_non_pending_writer()
    .unwrap();

  assert!(!largest_non_pending_writer.dwpt.lock().is_flush_pending());

  let num_ram_docs = w.num_ram_docs()?;
  let num_docs_in_dwpt = largest_non_pending_writer.dwpt.lock().get_num_docs_in_ram();

  assert!(w.flush_next_buffer()?);
  assert!(largest_non_pending_writer.dwpt.lock().has_flushed());
  assert_eq!(num_ram_docs - num_docs_in_dwpt, w.num_ram_docs()?);

  // Make sure it's not locked.
  {
    largest_non_pending_writer.lock();
    largest_non_pending_writer.unlock();
  }

  if random.random_bool(0.5) {
    w.commit()?;
  }

  let reader = directory_reader::open_with_writer_deletes(&w, true, true)?;
  assert_eq!(num_docs, reader.num_docs()?);

  w.close()?;

  Ok(())
}

fn index_docs_for_multiple_dwpts<R>(
  writer: &IndexWriter<DirEnum, EmptyIndexWriterBase>,
  random: &mut R,
) -> Result<i32>
where
  R: Rng + ?Sized,
{
  let num_threads = 3;
  let latch = Arc::new(Barrier::new(num_threads));
  let num_docs_per_thread = 10 + random.random_range(0..30);

  thread::scope(|scope| -> Result<()> {
    let mut threads = Vec::new();

    for _ in 0..num_threads {
      let latch = latch.clone();

      threads.push(scope.spawn(move || -> Result<()> {
        latch.wait();

        for _ in 0..num_docs_per_thread {
          let mut doc = Document::new();
          doc.add(StringField::from_string("id", "foo", Store::Yes)?);
          writer.add_document(doc)?;
        }

        Ok(())
      }));
    }

    for handle in threads {
      handle.join().expect("thread panicked")?;
    }

    Ok(())
  })?;

  Ok(num_docs_per_thread * num_threads as i32)
}

#[test]
fn test_never_check_out_on_full_flush() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), IndexWriterConfig::new())?;

  index_docs_for_multiple_dwpts(&w, &mut random)?;

  let largest_non_pending_writer = w
    .doc_writer
    .flush_control
    .find_largest_non_pending_writer()
    .unwrap();

  assert!(!largest_non_pending_writer.dwpt.lock().is_flush_pending());
  assert!(!largest_non_pending_writer.dwpt.lock().has_flushed());

  let thread_pool_size = w.doc_writer.flush_control.per_thread_pool.size();

  {
    let guard = w.doc_writer.guard.lock();
    w.doc_writer
      .flush_control
      .mark_for_full_flush(&w.doc_writer, &guard, &w.config)?;
  }

  let documents_writer_per_thread = w
    .doc_writer
    .flush_control
    .checkout_largest_non_pending_writer(&w.config)?;

  assert!(documents_writer_per_thread.is_none());
  assert_eq!(
    thread_pool_size,
    w.doc_writer.flush_control.num_queued_flushes()
  );

  w.doc_writer
    .flush_control
    .abort_full_flushes(&w.doc_writer, &w.config)?;

  assert!(
    w.doc_writer
      .flush_control
      .checkout_largest_non_pending_writer(&w.config)?
      .is_none(),
    "was aborted"
  );

  assert_eq!(0, w.doc_writer.flush_control.num_queued_flushes());

  w.close()?;

  Ok(())
}

#[test]
fn test_apply_deletes_without_flushes() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut index_writer_config = IndexWriterConfig::new();
  let flush_deletes = Arc::new(AtomicBool::new(false));
  index_writer_config.set_flush_policy(ApplyDeletesFlushPolicy::new(flush_deletes.clone()));
  let w = IndexWriter::new(dir.clone(), index_writer_config)?;

  assert_eq!(0, w.doc_writer.flush_control.get_delete_bytes_used()?);
  w.delete_documents_with_terms(vec![Term::from_text("foo", "bar")])?;
  let mut _bytes_used = w.doc_writer.flush_control.get_delete_bytes_used()?;
  // TODO: memory calculation not implement
  // assert!(bytes_used > 0, "{bytes_used} > 0");
  w.delete_documents_with_terms(vec![Term::from_text("foo", "baz")])?;
  _bytes_used = w.doc_writer.flush_control.get_delete_bytes_used()?;
  // TODO: memory calculation not implement
  // assert!(bytes_used > 0, "{bytes_used} > 0");
  assert_eq!(2, w.doc_writer.get_buffered_delete_terms_size()?);
  assert_eq!(0, w.get_flush_deletes_count());
  flush_deletes.store(true, SeqCst);
  w.delete_documents_with_terms(vec![Term::from_text("foo", "bar")])?;
  assert_eq!(0, w.doc_writer.flush_control.get_delete_bytes_used()?);
  assert_eq!(1, w.get_flush_deletes_count());

  w.close()?;
  Ok(())
}
#[test]
fn test_deletes_applied_on_flush() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();
  {
    let w = IndexWriter::new(dir.clone(), IndexWriterConfig::new())?;
    let mut doc = Document::new();
    doc.add(new_field(
      &mut random,
      "id",
      "1",
      &STORED_TEXT_TYPE,
      &mut field_types,
    )?);
    w.add_document(doc.clone())?;
    w.update_document_with_term(Term::from_text("id", "1"), doc)?;
    let mut _delete_bytes_used = w.doc_writer.flush_control.get_delete_bytes_used()?;
    // TODO: memory calculation not implement
    // assert!(
    //   delete_bytes_used > 0,
    //   "deletedBytesUsed: {delete_bytes_used}"
    // );
    assert_eq!(0, w.get_flush_deletes_count());
    assert!(w.flush_next_buffer()?);
    assert_eq!(1, w.get_flush_deletes_count());
    assert_eq!(0, w.doc_writer.flush_control.get_delete_bytes_used()?);
    w.delete_all()?;
    w.commit()?;
    assert_eq!(2, w.get_flush_deletes_count());
    if random.random_bool(0.5) {
      w.delete_documents_with_terms(vec![Term::from_text("id", "1")])?;
    } else {
      w.update_doc_values(
        Term::from_text("id", "1"),
        vec![NumericDocValuesField::new("foo", 1).into()],
      )?;
    }
    _delete_bytes_used = w.doc_writer.flush_control.get_delete_bytes_used()?;
    // TODO: memory calculation not implement
    // assert!(
    //   delete_bytes_used > 0,
    //   "deletedBytesUsed: {delete_bytes_used}"
    // );
    doc = Document::new();
    doc.add(new_field(
      &mut random,
      "id",
      "5",
      &STORED_TEXT_TYPE,
      &mut field_types,
    )?);
    w.add_document(doc)?;
    assert!(w.flush_next_buffer()?);
    assert_eq!(0, w.doc_writer.flush_control.get_delete_bytes_used()?);
    assert_eq!(3, w.get_flush_deletes_count());
    w.close()?;
  }

  {
    let w = RandomIndexWriter::with_config(&mut random, dir.clone(), IndexWriterConfig::new());
    let num_docs = random.random_range(1..100);
    for i in 0..num_docs {
      let mut doc = Document::new();
      doc.add(new_field(
        &mut random,
        "id",
        i.to_string(),
        &STORED_TEXT_TYPE,
        &mut field_types,
      )?);
      w.add_document(doc)?;
    }
    for i in 0..num_docs {
      if random.random_bool(0.5) {
        let mut doc = Document::new();
        doc.add(new_field(
          &mut random,
          "id",
          i.to_string(),
          &STORED_TEXT_TYPE,
          &mut field_types,
        )?);
        w.update_document_with_term(Term::from_text("id", i.to_string()), doc)?;
      }
    }

    let delete_bytes_used = w.w.doc_writer.flush_control.get_delete_bytes_used()?;
    if delete_bytes_used > 0 {
      assert!(w.w.flush_next_buffer()?);
      assert_eq!(0, w.w.doc_writer.flush_control.get_delete_bytes_used()?);
    }
    w.close()?;
  }

  Ok(())
}

#[test]
fn test_hold_lock_on_largest_writer() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), IndexWriterConfig::new())?;
  let num_docs = index_docs_for_multiple_dwpts(&w, &mut random)?;

  let largest_non_pending_writer = w
    .doc_writer
    .flush_control
    .find_largest_non_pending_writer()
    .unwrap();
  assert!(!largest_non_pending_writer.dwpt.lock().is_flush_pending());
  assert!(!largest_non_pending_writer.dwpt.lock().has_flushed());

  let locked = Arc::new(Barrier::new(3));
  let wait = Arc::new(Barrier::new(2));

  thread::scope(|scope| -> Result<()> {
    let lock_thread = {
      let largest_non_pending_writer = Arc::clone(&largest_non_pending_writer);
      let locked = Arc::clone(&locked);
      let wait = Arc::clone(&wait);
      scope.spawn(move || {
        largest_non_pending_writer.lock();
        locked.wait();
        wait.wait();
        largest_non_pending_writer.unlock();
      })
    };

    let flush_thread = {
      let locked = Arc::clone(&locked);
      let writer = &w;
      scope.spawn(move || -> Result<()> {
        locked.wait();
        assert!(writer.flush_next_buffer()?);
        Ok(())
      })
    };

    locked.wait();
    // Access a synced method to ensure we never lock while we hold the flush control monitor.
    w.doc_writer.flush_control.active_bytes(None);
    wait.wait();

    lock_thread.join().expect("thread panicked");
    flush_thread.join().expect("thread panicked")?;

    Ok(())
  })?;

  assert!(
    largest_non_pending_writer.dwpt.lock().has_flushed(),
    "largest DWPT should be flushed"
  );

  // Make sure it's not locked.
  largest_non_pending_writer.lock();
  largest_non_pending_writer.unlock();

  if random.random_bool(0.5) {
    w.commit()?;
  }

  let reader = directory_reader::open_with_writer_deletes(&w, true, true)?;
  assert_eq!(num_docs, reader.num_docs()?);

  w.close()?;

  Ok(())
}
// TODO IMPORTANT 多线程索引 BUG
fn test_check_pending_flush_post_update() -> Result<()> {
  let mut random = random();

  // TODO IMPORTANT MockDirectoryWrapper未实现
  let dir = new_directory_shared(&mut random)?;
  let mut config = IndexWriterConfig::new();
  config.get_base_mut().check_pending_flush_on_update = false;
  config.set_max_buffered_docs(i32::MAX);
  config.set_ram_buffer_size_mb(DISABLE_AUTO_FLUSH as f64);
  let w = IndexWriter::new(dir.clone(), config)?;
  let done = AtomicBool::new(false);
  let num_threads = 2 + random.random_range(0..3);
  let latch = Barrier::new(num_threads + 1);

  thread::scope(|scope| -> Result<()> {
    let mut threads = Vec::new();
    for _ in 0..num_threads {
      threads.push(scope.spawn(|| -> Result<()> {
        latch.wait();
        let mut num_docs = 0;
        while !done.load(SeqCst) {
          let mut doc = Document::new();
          doc.add(StringField::from_string("id", "foo", Store::Yes)?);
          w.add_document(doc)?;
          if num_docs % 10 == 0 {
            thread::yield_now();
          }
          num_docs += 1;
        }
        Ok(())
      }));
    }
    latch.wait();

    let result = (|| -> Result<()> {
      let num_iters = if rarely(&mut random) {
        1 + random.random_range(0..5)
      } else {
        1
      };
      for _ in 0..num_iters {
        wait_for_docs_in_buffers(&w, std::cmp::min(2, num_threads));
        w.commit()?;
        // TODO IMPORTANT MockDirectoryWrapper未实现, 无法断言flush发生在当前线程且不在indexing线程.
      }
      Ok(())
    })();

    done.store(true, SeqCst);
    for handle in threads {
      handle.join().expect("thread panicked")?;
    }
    result
  })?;
  w.close()?;
  Ok(())
}

fn wait_for_docs_in_buffers<D, B>(w: &IndexWriter<D, B>, buffers_with_docs: usize)
where
  D: Directory,
  B: IndexWriterBase,
{
  // wait until at least N DWPTs have a doc in order to observe who flushes the segments.
  loop {
    let mut num_states_with_docs = 0;
    let per_thread_pool = &w.doc_writer.flush_control.per_thread_pool;
    for (_id, dwpt) in per_thread_pool.iterator() {
      dwpt.lock();
      let num_docs_in_ram = dwpt.dwpt.lock().get_num_docs_in_ram();
      dwpt.unlock();
      if num_docs_in_ram > 1 {
        num_states_with_docs += 1;
      }
    }
    if num_states_with_docs >= buffers_with_docs {
      return;
    }
    thread::yield_now();
  }
}

#[test]
fn test_soft_update_documents() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_soft_updates_concurrently() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_soft_updates_concurrently_mixed_deletes() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_delete_happens_before_while_flush() -> Result<()> {
  // TODO
  Ok(())
}
fn assert_files<D, B>(writer: &IndexWriter<D, B>) -> Result<()>
where
  D: Directory,
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

  let mut config = new_index_writer_config(&mut random);
  config.set_ram_buffer_size_mb(i32::MAX as f64);
  config.set_max_buffered_docs(2);
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

// TODO IMPORTANT PendingSoftDeletes# on_new_reader未实现
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
      w.update_document_with_term(Term::from_text("id", "1"), d)?;
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
  let mut random = random();
  let d = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let w = IndexWriter::new(d, iwc)?;

  let mut doc = Document::new();
  let mut token = token::with_range(Some("bar"), 0, 3)?;

  let mut evil = BytesRef::from_bytes(vec![0u8; 1024]);
  evil.offset = 1000;

  token.sub.token.set_payload(Some(evil));

  doc.add(TextField::from_token_stream(
    "foo",
    FieldTokenStreamEnum::custom(CannedTokenStream::new(vec![token])),
  )?);

  let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| w.add_document(doc)));
  assert!(result.is_err());
  Ok(())
}
// TODO IMPORTANT PendingSoftDeletes# on_new_reader未实现
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
      writer.update_document_with_term(Term::from_text("id", doc_id.to_string()), doc)?;
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
  // TODO IMPORTANT OneMergeWrappingMergePolicy未实现
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

// TODO IMPORTANT 多线程索引BUG
fn test_flush_while_starting_new_threads() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), IndexWriterConfig::new())?;
  w.add_document(Document::new())?;
  assert_eq!(1, w.doc_writer.flush_control.per_thread_pool.size());

  let latch = Barrier::new(2);

  thread::scope(|scope| -> Result<()> {
    let thread = scope.spawn(|| -> Result<()> {
      latch.wait();
      let mut states = Vec::new();
      let result = (|| -> Result<()> {
        for _ in 0..100 {
          let delete_queue = w.doc_writer.flush_control.delete_queue.lock().clone();
          let state = w
            .doc_writer
            .flush_control
            .per_thread_pool
            .get_and_lock(&w, delete_queue)?;
          state.state.delete_queue.get_next_sequence_number();
          states.push(state);
        }
        Ok(())
      })();
      for state in states {
        state.unlock();
      }
      result
    });

    latch.wait();
    {
      let guard = w.doc_writer.guard.lock();
      w.doc_writer
        .flush_control
        .mark_for_full_flush(&w.doc_writer, &guard, &w.config)?;
    }
    thread.join().expect("thread panicked")?;
    w.doc_writer
      .flush_control
      .abort_full_flushes(&w.doc_writer, &w.config)?;

    Ok(())
  })?;

  w.close()?;

  Ok(())
}

#[test]
fn test_refresh_and_rollback_concurrently() -> Result<()> {
  // TODO IMPORTANT 多线程 SearcherManager 未实现
  Ok(())
}

#[test]
fn test_closeable_queue() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  let queue = Arc::new(EventQueue::new());
  let executed = Arc::new(AtomicI32::new(0));

  queue.add(EventEnum::Test(EventImplTest::new(executed.clone())))?;
  queue.add(EventEnum::Test(EventImplTest::new(executed.clone())))?;
  queue.process_events(&writer)?;
  assert_eq!(2, executed.load(SeqCst));
  queue.process_events(&writer)?;
  assert_eq!(2, executed.load(SeqCst));

  queue.add(EventEnum::Test(EventImplTest::new(executed.clone())))?;
  queue.add(EventEnum::Test(EventImplTest::new(executed.clone())))?;

  thread::scope(|scope| -> Result<()> {
    let thread_queue = queue.clone();
    let writer = &writer;
    let t = scope.spawn(move || -> Result<()> {
      match thread_queue.process_events(writer) {
        Ok(_) => Ok(()),
        Err(LuceneError::AlreadyClosed(_)) => Ok(()),
        Err(e) => Err(e),
      }
    });
    queue.close(writer)?;
    t.join().expect("thread panicked")?;
    Ok(())
  })?;

  assert_eq!(4, executed.load(SeqCst));
  let err = queue.process_events(&writer);
  assert!(matches!(err, Err(LuceneError::AlreadyClosed(_))));
  let err = queue.add(EventEnum::Test(EventImplTest::new(executed.clone())));
  assert!(matches!(err, Err(LuceneError::AlreadyClosed(_))));

  writer.close()?;
  Ok(())
}

#[test]
fn test_random_operations() -> Result<()> {
  // TODO IMPORTANT 多线程 SearcherManager未实现
  Ok(())
}

#[test]
fn test_random_operations_with_soft_deletes() -> Result<()> {
  // TODO IMPORTANT 多线程SearcherManager 未实现
  Ok(())
}

#[test]
fn test_max_completed_sequence_number() -> Result<()> {
  // TODO IMPORTANT 多线程 SearcherManager未实现
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
      writer.update_document_with_term(Term::from_text("id", "1"), doc)?;
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
  writer.update_document_with_term(Term::from_text("id", "1"), d)?;
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
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
  let stall_control = &writer.get_docs_writer().flush_control.stall_control;
  stall_control.update_stalled(true);
  let num_threads = random.random_range(0..3) + 1;
  let num_threads_completed = AtomicI64::new(0);

  thread::scope(|scope| -> Result<()> {
    let mut threads = Vec::new();
    for _ in 0..num_threads {
      threads.push(scope.spawn(|| -> Result<()> {
        let mut d = Document::new();
        d.add(StringField::from_string("id", 0.to_string(), Store::Yes)?);
        writer.add_document(d)?;
        num_threads_completed.fetch_add(1, SeqCst);
        Ok(())
      }));
    }

    let result = {
      for _ in 0..10 {
        while stall_control.get_num_waiting() != num_threads {
          // wait for all threads to be stalled again
          assert_eq!(0, writer.get_pending_num_docs());
          assert_eq!(0, num_threads_completed.load(SeqCst));
          thread::yield_now();
        }
      }
      Ok(())
    };

    stall_control.update_stalled(false);
    for thread in threads {
      thread.join().expect("thread panicked")?;
    }
    result
  })?;

  writer.commit()?;
  assert_eq!(num_threads, writer.get_doc_stats()?.max_doc);
  writer.close()?;
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
fn test_index_with_parent_field_is_congruent() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  {
    let mock = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    iwc.set_parent_field("parent");
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    if random.random_bool(0.5) {
      let mut child1 = Document::new();
      child1.add(StringField::from_string("id", 1.to_string(), Store::Yes)?);
      let mut child2 = Document::new();
      child2.add(StringField::from_string("id", 1.to_string(), Store::Yes)?);
      let mut parent = Document::new();
      parent.add(StringField::from_string("id", 1.to_string(), Store::Yes)?);
      writer.add_documents(vec![child1.clone(), child2.clone(), parent.clone()])?;
      writer.flush()?;
      if random.random_bool(0.5) {
        writer.add_documents(vec![child1, child2, parent])?;
      }
    } else {
      writer.add_document(Document::new())?;
    }
    writer.commit()?;
    writer.close()?;
  }

  {
    let mock = MockAnalyzer::new(&mut random);
    let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
    config.set_parent_field("someOtherField");

    let err = IndexWriter::new(dir.clone(), config);
    match err {
      Ok(writer) => {
        writer.close()?;
        panic!("expected IllegalArgument error");
      },
      Err(err) => {
        assert!(matches!(err, LuceneError::IllegalArgument(_)));
        assert_eq!(
          "can't add field [parent] as parent document field; this IndexWriter is configured with [someOtherField] as parent document field",
          err.to_string()
        );
      },
    }
  }

  {
    let mock = MockAnalyzer::new(&mut random);
    let config = new_index_writer_config_with_analyzer(&mut random, mock);

    let err = IndexWriter::new(dir, config);
    match err {
      Ok(writer) => {
        writer.close()?;
        panic!("expected IllegalArgument error");
      },
      Err(err) => {
        assert!(matches!(err, LuceneError::IllegalArgument(_)));
        assert_eq!(
          "can't add field [parent] as parent document field; this IndexWriter has no parent document field configured",
          err.to_string()
        );
      },
    }
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

// Make sure we can flush segment w/ norms, then add empty doc (no norms) and flush
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

struct NegativePositionsTokenStream {
  attrs: Attributes,
  terms: [&'static str; 3],
  upto: usize,
  first: bool,
}

impl NegativePositionsTokenStream {
  fn new() -> Self {
    Self {
      attrs: Attributes::default(),
      terms: ["a", "b", "c"],
      upto: 0,
      first: true,
    }
  }
}

impl TokenStream for NegativePositionsTokenStream {
  fn increment_token(&mut self) -> Result<bool> {
    if self.upto == self.terms.len() {
      return Ok(false);
    }

    self.attrs.clear_attributes();
    self.attrs.append_str(Some(self.terms[self.upto]))?;
    self
      .attrs
      .set_position_increment(if self.first { 0 } else { 1 })?;
    self.first = false;
    self.upto += 1;
    Ok(true)
  }

  fn end(&mut self) -> Result<()> {
    self.default_end()
  }

  fn reset(&mut self) -> Result<()> {
    self.upto = 0;
    self.first = true;
    Ok(())
  }

  fn get_attribute_source(&self) -> &Attributes {
    &self.attrs
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    &mut self.attrs
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

fn assert_hard_live_docs<D, B>(writer: &IndexWriter<D, B>, unique_docs: &HashSet<i32>) -> Result<()>
where
  D: Directory,
  B: IndexWriterBase,
{
  let reader = directory_reader::open_from_writer(writer)?;
  assert_eq!(unique_docs.len() as i32, reader.num_docs()?);
  let context = get_context(&reader)?;
  for ctx in context.leaves()? {
    let sr = ctx.reader();
    if let Some(hard_live_docs) = sr.get_hard_live_docs()? {
      let id = LeafReader::terms(sr, "id")?.unwrap();
      let mut iterator = id.iterator()?;
      let live_docs = sr.get_live_docs()?.unwrap();
      for d_id in unique_docs {
        let must_be_hard_deleted = d_id % 2 == 0;
        if iterator.seek_exact(&BytesRef::from_string(&d_id.to_string()))? {
          let mut postings = iterator.postings(None)?;
          while postings.next_doc()? != NO_MORE_DOCS {
            let doc_id = postings.doc_id() as usize;
            if live_docs.get(doc_id)? {
              assert!(hard_live_docs.get(doc_id)?);
            } else if must_be_hard_deleted {
              assert!(!hard_live_docs.get(doc_id)?);
            } else {
              assert!(hard_live_docs.get(doc_id)?);
            }
          }
        }
      }
    }
  }
  reader.close()?;
  Ok(())
}

fn add_doc_with_field<D, B, R>(
  random: &mut R,
  writer: &mut IndexWriter<D, B>,
  field: &str,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory,
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

  fn find_merges_readers<CR, D>(
    &self,
    readers: Vec<CR>,
  ) -> Result<Option<MergeSpecification<D, CR>>>
  where
    CR: CodecReader,
    D: Directory,
  {
    self.in_.find_merges_readers(readers)
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

  fn max_full_flush_merge_size(&self) -> i64 {
    self.in_.max_full_flush_merge_size()
  }

  fn keep_fully_deleted_segment<D, F>(&self, _reader_supplier: F) -> Result<bool>
  where
    D: Directory,
    F: Fn() -> Result<DefaultLeafReader<D>>,
  {
    Ok(true)
  }

  fn num_deletes_to_merge<D, F>(
    &self,
    info: &SegmentCommitInfo<D>,
    del_count: i32,
    reader_supplier: F,
  ) -> Result<i32>
  where
    D: Directory,
    F: Fn() -> Result<DefaultLeafReader<D>>,
  {
    self
      .in_
      .num_deletes_to_merge(info, del_count, reader_supplier)
  }
}

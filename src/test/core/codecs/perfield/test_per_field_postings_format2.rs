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
use crate::core::document::int_point::IntPoint;
use crate::core::document::string_field::StringField;
use crate::core::document::text_field;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::{DefaultIndexWriter, IndexWriter};
use crate::core::index::index_writer_config::{DISABLE_AUTO_FLUSH, IndexWriterConfig, OpenMode};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::log_doc_merge_policy::LogDocMergePolicy;
use crate::core::index::log_merge_policy::LogMergePolicy;
use crate::core::index::merge_policy::MergePolicy;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::term_query::TermQuery;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::codecs::asserting_codec::{AssertingCodec, AssertingCodecHook};
use crate::test_framework::core::codecs::perfield::test_per_field_postings_format::{
  MergeCalledOnTwoFormatsPostingsAssertingCodec, MergeRecordingPostingsFormatWrapper,
  MockAssertingCodec, SameCodecDifferentInstanceAssertingCodec,
};
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_field, new_index_writer_config_with_analyzer,
  new_searcher_with_reader, new_string_field, new_text_field, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[allow(clippy::empty_docs)]
#[allow(dead_code)] // for quick search
struct TestPerFieldPostingsFormat2;

fn new_writer(
  dir: Arc<DirEnum>,
  mut conf: IndexWriterConfig<DirEnum>,
) -> Result<DefaultIndexWriter<DirEnum>> {
  let mut log_byte_size_merge_policy = LogMergePolicy::<LogDocMergePolicy>::log_doc();
  <LogMergePolicy<LogDocMergePolicy> as MergePolicy<DirEnum>>::get_base_mut(
    &mut log_byte_size_merge_policy,
  )
  .set_no_cfs_ratio(0.0)?; // make sure we use plain
  // files
  conf.set_merge_policy(log_byte_size_merge_policy);

  IndexWriter::new(dir, conf)
}

fn add_docs<R>(
  random: &mut R,
  writer: &DefaultIndexWriter<DirEnum>,
  num_docs: usize,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  R: Rng + ?Sized,
{
  for _ in 0..num_docs {
    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      "content",
      "aaa",
      Store::No,
      field_to_type,
    )?);
    writer.add_document(doc)?;
  }
  Ok(())
}

fn add_docs2<R>(
  random: &mut R,
  writer: &DefaultIndexWriter<DirEnum>,
  num_docs: usize,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  R: Rng + ?Sized,
{
  for _ in 0..num_docs {
    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      "content",
      "bbb",
      Store::No,
      field_to_type,
    )?);
    writer.add_document(doc)?;
  }
  Ok(())
}

fn add_docs3<R>(
  random: &mut R,
  writer: &DefaultIndexWriter<DirEnum>,
  num_docs: usize,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  R: Rng + ?Sized,
{
  for i in 0..num_docs {
    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      "content",
      "ccc",
      Store::No,
      field_to_type,
    )?);
    doc.add(new_string_field(
      random,
      "id",
      i.to_string(),
      Store::Yes,
      field_to_type,
    )?);
    writer.add_document(doc)?;
  }
  Ok(())
}

/*
 * Test that heterogeneous index segments are merge successfully
 */
#[test]
fn test_merge_unused_per_field_codec() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwconf = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwconf
    .set_open_mode(OpenMode::Create)
    .set_codec(AssertingCodec::with_hook(AssertingCodecHook::MockPostings(
      MockAssertingCodec::new(),
    )));
  let writer = new_writer(dir.clone(), iwconf)?;
  let mut field_to_type = HashMap::new();
  add_docs(&mut random, &writer, 10, &mut field_to_type)?;
  writer.commit()?;
  add_docs3(&mut random, &writer, 10, &mut field_to_type)?;
  writer.commit()?;
  add_docs2(&mut random, &writer, 10, &mut field_to_type)?;
  writer.commit()?;
  assert_eq!(30, writer.get_doc_stats()?.max_doc);
  TestUtil::check_index(&mut random, dir.as_ref())?;
  writer.force_merge(1)?;
  assert_eq!(30, writer.get_doc_stats()?.max_doc);
  writer.close()?;
  dir.close()
}

#[test]
fn test_change_codec_and_merge() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  if cfg!(feature = "test_log_verbose") {
    println!("TEST: make new index");
  }
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwconf = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwconf
    .set_open_mode(OpenMode::Create)
    .set_codec(AssertingCodec::with_hook(AssertingCodecHook::MockPostings(
      MockAssertingCodec::new(),
    )));
  iwconf.set_max_buffered_docs(DISABLE_AUTO_FLUSH);
  let writer = new_writer(dir.clone(), iwconf)?;
  let mut field_to_type = HashMap::new();

  add_docs(&mut random, &writer, 10, &mut field_to_type)?;
  writer.commit()?;
  assert_query(Term::from_text("content", "aaa"), dir.clone(), 10)?;
  if cfg!(feature = "test_log_verbose") {
    println!("TEST: addDocs3");
  }
  add_docs3(&mut random, &writer, 10, &mut field_to_type)?;
  writer.commit()?;
  writer.close()?;

  assert_query(Term::from_text("content", "ccc"), dir.clone(), 10)?;
  assert_query(Term::from_text("content", "aaa"), dir.clone(), 10)?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwconf = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwconf
    .set_open_mode(OpenMode::Append)
    .set_codec(AssertingCodec::with_hook(AssertingCodecHook::MockPostings(
      MockAssertingCodec::new(),
    )));
  iwconf.set_max_buffered_docs(DISABLE_AUTO_FLUSH);

  iwconf.set_codec(AssertingCodec::with_hook(AssertingCodecHook::MockPostings(
    MockAssertingCodec::new(),
  ))); // uses standard for field content
  let writer = new_writer(dir.clone(), iwconf)?;
  // swap in new codec for currently written segments
  if cfg!(feature = "test_log_verbose") {
    println!("TEST: add docs w/ Standard codec for content field");
  }
  add_docs2(&mut random, &writer, 10, &mut field_to_type)?;
  writer.commit()?;
  assert_eq!(30, writer.get_doc_stats()?.max_doc);
  assert_query(Term::from_text("content", "bbb"), dir.clone(), 10)?;
  assert_query(Term::from_text("content", "ccc"), dir.clone(), 10)?; // //
  assert_query(Term::from_text("content", "aaa"), dir.clone(), 10)?;

  if cfg!(feature = "test_log_verbose") {
    println!("TEST: add more docs w/ new codec");
  }
  add_docs2(&mut random, &writer, 10, &mut field_to_type)?;
  writer.commit()?;
  assert_query(Term::from_text("content", "ccc"), dir.clone(), 10)?;
  assert_query(Term::from_text("content", "bbb"), dir.clone(), 20)?;
  assert_query(Term::from_text("content", "aaa"), dir.clone(), 10)?;
  assert_eq!(40, writer.get_doc_stats()?.max_doc);

  if cfg!(feature = "test_log_verbose") {
    println!("TEST: now optimize");
  }
  writer.force_merge(1)?;
  assert_eq!(40, writer.get_doc_stats()?.max_doc);
  writer.close()?;
  assert_query(Term::from_text("content", "ccc"), dir.clone(), 10)?;
  assert_query(Term::from_text("content", "bbb"), dir.clone(), 20)?;
  assert_query(Term::from_text("content", "aaa"), dir.clone(), 10)?;

  dir.close()
}

fn assert_query(t: Term, dir: Arc<DirEnum>, num: usize) -> Result<()> {
  if cfg!(feature = "test_log_verbose") {
    println!("\nTEST: assertQuery {t}");
  }
  let reader = directory_reader::open(dir)?;
  let searcher = new_searcher_with_reader(reader)?;
  let search = searcher.search(TermQuery::new(t), num + 10)?;
  assert_eq!(num, search.total_hits.value());
  searcher.reader_context.reader().close()
}

/*
 * Test per field codec support - adding fields with random codecs
 */
#[test]
fn test_stress_per_field_codec() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let docs_per_round = 97;
  let num_rounds = at_least(&mut random, 1);
  for i in 0..num_rounds {
    let num = TestUtil::next_int(&mut random, 30, 60);
    let analyzer = MockAnalyzer::new(&mut random);
    let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
    config.set_open_mode(OpenMode::CreateOrAppend);
    let writer = new_writer(dir.clone(), config)?;
    let mut field_to_type = HashMap::new();
    for _ in 0..docs_per_round {
      let mut doc = Document::new();
      for k in 0..num {
        let mut custom_type = FieldType::from_ref(&*text_field::TYPE_NOT_STORED)?;
        custom_type.set_tokenized(random.random_bool(0.5))?;
        custom_type.set_omit_norms(random.random_bool(0.5))?;
        let value = TestUtil::random_realistic_unicode_string_with_len(&mut random, 128);
        let field = new_field(
          &mut random,
          k.to_string(),
          value,
          &custom_type,
          &mut field_to_type,
        )?;
        doc.add(field);
      }
      writer.add_document(doc)?;
    }
    if random.random_bool(0.5) {
      writer.force_merge(1)?;
    }
    writer.commit()?;
    assert_eq!((i + 1) * docs_per_round, writer.get_doc_stats()?.max_doc);
    writer.close()?;
  }
  dir.close()
}

#[test]
fn test_same_codec_different_instance() -> Result<()> {
  let codec = AssertingCodec::with_hook(AssertingCodecHook::SameCodecDifferentInstance(
    SameCodecDifferentInstanceAssertingCodec::new(),
  ));
  do_test_mixed_postings(codec)
}

#[test]
fn test_same_codec_different_params() -> Result<()> {
  // LuceneVarGapFixedInterval has not been migrated to Rust. A different postings format would not
  // preserve the Java test's format and parameter behavior.
  test_not_required_in_rust_lucene!();
}

fn do_test_mixed_postings(codec: AssertingCodec) -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwc.set_codec(codec);
  let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);
  let mut ft = FieldType::from_ref(&*text_field::TYPE_NOT_STORED)?;
  // turn on vectors for the checkindex cross-check
  ft.set_store_term_vectors(true)?;
  ft.set_store_term_vector_offsets(true)?;
  ft.set_store_term_vector_positions(true)?;
  let mut field_to_type = HashMap::new();
  for _ in 0..100 {
    let mut doc = Document::new();
    let id = random.random_range(0..50).to_string();
    let date = random.random_range(0..100).to_string();
    doc.add(new_field(&mut random, "id", id, &ft, &mut field_to_type)?);
    doc.add(new_field(
      &mut random,
      "date",
      date,
      &ft,
      &mut field_to_type,
    )?);
    iw.add_document(&mut random, doc)?;
  }
  iw.close(&mut random)?;
  dir.close() // checkindex
}

#[test]
fn test_merge_called_on_two_formats() -> Result<()> {
  let mut random = random();
  let pf1 = MergeRecordingPostingsFormatWrapper::new(TestUtil::get_default_postings_format());
  let pf2 = MergeRecordingPostingsFormatWrapper::new(TestUtil::get_default_postings_format());

  let mut iwc = IndexWriterConfig::new()?;
  iwc.set_codec(AssertingCodec::with_hook(
    AssertingCodecHook::MergeCalledOnTwoFormatsPostings(
      MergeCalledOnTwoFormatsPostingsAssertingCodec::new(pf1.clone().into(), pf2.clone().into()),
    ),
  ));

  let directory = new_directory_shared(&mut random)?;

  let iwriter = IndexWriter::new(directory.clone(), iwc)?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("f1", "val1", Store::No)?);
  doc.add(StringField::from_string("f2", "val2", Store::Yes)?);
  doc.add(IntPoint::new("f3", [3])?); // Points are not indexed as postings and should not appear in the merge fields
  doc.add(StringField::from_string("f4", "val4", Store::No)?);
  iwriter.add_document(doc)?;
  iwriter.commit()?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("f1", "val5", Store::No)?);
  doc.add(StringField::from_string("f2", "val6", Store::Yes)?);
  doc.add(IntPoint::new("f3", [7])?);
  doc.add(StringField::from_string("f4", "val8", Store::No)?);
  iwriter.add_document(doc)?;
  iwriter.commit()?;

  iwriter.force_merge_with_wait(1, true)?;
  iwriter.close()?;

  assert_eq!(1, pf1.nb_merge_calls());
  assert_eq!(
    HashSet::from(["f1".to_string(), "f2".to_string()]),
    pf1.field_names().into_iter().collect()
  );
  assert_eq!(1, pf2.nb_merge_calls());
  assert_eq!(vec!["f4".to_string()], pf2.field_names());

  directory.close()
}

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
use crate::core::document::text_field::TextField;
use crate::core::index::composite_reader::{CompositeReader, get_context};
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_reader::{Identity, IndexReader};
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::log_merge_policy::LogMergePolicy;
use crate::core::index::multi_doc_values::MultiDocValues;
use crate::core::index::no_deletion_policy::NoDeletionPolicy;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::postings_enum::{ALL, PostingsEnum};
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::index::{directory_reader, field_infos, multi_bits, multi_terms};
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::term_query::TermQuery;
use crate::core::store::buffered_checksum_index_input::BufferedChecksumIndexInput;
use crate::core::store::directory::Directory;
use crate::core::store::{ByteBuffersDirectory, IOContext};
use crate::core::util::HasIdentity;
use crate::core::util::bits::Bits;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, new_index_writer_config_with_analyzer,
  new_log_merge_policy, new_log_merge_policy_with_merge_factor, new_string_field, random,
};
use rand_chacha::rand_core::Rng;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::io::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[allow(dead_code)] // for quick search
struct TestDirectoryReaderReopen;

#[test]
fn test_reopen() -> Result<()> {
  let mut random = random();
  let dir1 = new_directory_shared(&mut random)?;

  let iw = create_index(&mut random, dir1.clone(), false)?;
  let test = TestReopen { dir: dir1.clone() };
  perform_default_tests(&mut random, &test, iw)?;

  let dir2 = new_directory_shared(&mut random)?;

  let iw = create_index(&mut random, dir2.clone(), true)?;
  let test = TestReopen { dir: dir2.clone() };
  perform_default_tests(&mut random, &test, iw)?;

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
  D: Directory + 'static,
{
  let analyzer = MockAnalyzer::new(random);
  let mut config = new_index_writer_config_with_analyzer(random, analyzer)?;
  config.set_open_mode(OpenMode::Create);
  config.set_merge_scheduler(SerialMergeScheduler::new());
  config.set_merge_policy(new_log_merge_policy(random)?);
  let iwriter = IndexWriter::new(dir.clone(), config)?;
  iwriter.commit()?;
  let mut reader = directory_reader::open(dir.clone())?;

  let m = 3;
  let mut custom_type = FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
  custom_type.set_tokenized(false)?;
  let mut custom_type2 = FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
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
      if let Some(v) = directory_reader::open_if_changed(&reader, &iwriter)? {
        reader.close()?;
        reader = v;
      }
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

struct ReaderCouple<D>
where
  D: Directory,
{
  new_reader: Option<StandardDirectoryReaderType<D>>,
  refreshed_reader: RefreshedReader<D>,
}
#[allow(clippy::large_enum_variant)]
enum RefreshedReader<D>
where
  D: Directory,
{
  Same,
  New(StandardDirectoryReaderType<D>),
}

struct TestReopen<D>
where
  D: Directory,
{
  dir: Arc<D>,
}

impl<D> TestReopen<D>
where
  D: Directory + 'static,
{
  fn open_reader(&self) -> Result<StandardDirectoryReaderType<D>> {
    directory_reader::open(self.dir.clone())
  }

  fn modify_index<R>(&self, random: &mut R, i: i32, iw: IndexWriter<D>) -> Result<IndexWriter<D>>
  where
    R: Rng + ?Sized,
  {
    modify_index(random, i, self.dir.clone(), iw)
  }
}

fn perform_default_tests<R, D>(
  random: &mut R,
  test: &TestReopen<D>,
  iw: IndexWriter<D>,
) -> Result<()>
where
  D: Directory + 'static,
  R: Rng + ?Sized,
{
  let mut index1 = test.open_reader()?;
  let mut index2 = test.open_reader()?;

  assert_index_equals(&index1, &index2)?;

  // verify that reopen() does not return a new reader instance
  // in case the index has no changes
  let (couple, iw) = refresh_reader(random, &index2, false, iw)?;
  match couple.refreshed_reader {
    RefreshedReader::Same => {},
    RefreshedReader::New(_) => panic!(
      "New DirectoryReader instance created during refresh even though index had no changes."
    ),
  }

  let (couple, iw) = refresh_reader_with_test(random, &index2, Some(test), 0, true, iw)?;
  index1.close()?;
  index1 = couple.new_reader.unwrap();

  let index2_refreshed = match couple.refreshed_reader {
    RefreshedReader::New(reader) => reader,
    RefreshedReader::Same => panic!("No new DirectoryReader instance created during refresh."),
  };
  index2.close()?;

  // test if refreshed reader and newly opened reader return equal results
  assert_index_equals(&index1, &index2_refreshed)?;

  index2_refreshed.close()?;
  assert_reader_closed(&index2, true);
  assert_reader_closed(&index2_refreshed, true);

  index2 = test.open_reader()?;
  let mut writer = iw;
  for i in 1..4 {
    index1.close()?;
    let (couple, iw) = refresh_reader_with_test(random, &index2, Some(test), i, true, writer)?;
    writer = iw;
    // refresh DirectoryReader
    index2.close()?;

    index2 = match couple.refreshed_reader {
      RefreshedReader::New(reader) => reader,
      RefreshedReader::Same => panic!("No new DirectoryReader instance created during refresh."),
    };
    index1 = couple.new_reader.unwrap();
    assert_index_equals(&index1, &index2)?;
  }

  index1.close()?;
  index2.close()?;
  assert_reader_closed(&index1, true);
  assert_reader_closed(&index2, true);
  Ok(())
}

fn refresh_reader<R, D>(
  random: &mut R,
  reader: &StandardDirectoryReaderType<D>,
  has_changes: bool,
  iw: IndexWriter<D>,
) -> Result<(ReaderCouple<D>, IndexWriter<D>)>
where
  D: Directory + 'static,
  R: Rng + ?Sized,
{
  refresh_reader_with_test(random, reader, None, -1, has_changes, iw)
}

fn refresh_reader_with_test<R, D>(
  random: &mut R,
  reader: &StandardDirectoryReaderType<D>,
  test: Option<&TestReopen<D>>,
  modify: i32,
  has_changes: bool,
  mut iw: IndexWriter<D>,
) -> Result<(ReaderCouple<D>, IndexWriter<D>)>
where
  D: Directory + 'static,
  R: Rng + ?Sized,
{
  let mut r = None;
  if let Some(test) = test {
    iw = test.modify_index(random, modify, iw)?;
    r = Some(test.open_reader()?);
  }

  let refreshed_reader = match directory_reader::open_if_changed(reader, &iw) {
    Ok(Some(refreshed)) => RefreshedReader::New(refreshed),
    Ok(None) => RefreshedReader::Same,
    Err(err) => {
      if let Some(reader) = r.as_ref() {
        let _ = reader.close();
      }
      return Err(err);
    },
  };

  if has_changes {
    if matches!(refreshed_reader, RefreshedReader::Same) {
      panic!("No new DirectoryReader instance created during refresh.");
    }
  } else if matches!(refreshed_reader, RefreshedReader::New(_)) {
    panic!("New DirectoryReader instance created during refresh even though index had no changes.");
  }

  Ok((
    ReaderCouple {
      new_reader: r,
      refreshed_reader,
    },
    iw,
  ))
}

fn create_index<R, D>(random: &mut R, dir: Arc<D>, multi_segment: bool) -> Result<IndexWriter<D>>
where
  R: rand::Rng + ?Sized,
  D: Directory + 'static,
{
  let analyzer = MockAnalyzer::new(random);
  let mut config = new_index_writer_config_with_analyzer(random, analyzer)?;
  config.set_merge_policy(LogMergePolicy::log_doc());
  let writer = IndexWriter::new(dir.clone(), config)?;

  for i in 0..100 {
    writer.add_document(create_document(i, 4)?)?;
    if multi_segment && (i % 10) == 0 {
      writer.commit()?;
    }
  }

  if !multi_segment {
    writer.force_merge(1)?;
  }
  writer.close()?;

  let r = directory_reader::open(dir.clone())?;
  if multi_segment {
    assert!(get_context(&r)?.leaves()?.len() > 1);
  } else {
    assert_eq!(1, get_context(&r)?.leaves()?.len());
  }
  r.close()?;

  Ok(writer)
}

fn modify_index<D, R>(
  random: &mut R,
  i: i32,
  dir: Arc<D>,
  iw: IndexWriter<D>,
) -> Result<IndexWriter<D>>
where
  D: Directory + 'static,
  R: Rng + ?Sized,
{
  let iw = match i {
    0 => {
      drop(iw);
      let analyzer = MockAnalyzer::new(random);
      let writer = IndexWriter::new(
        dir,
        new_index_writer_config_with_analyzer(random, analyzer)?,
      )?;
      writer.delete_documents_with_terms(vec![Term::from_text("field2", "a11")])?;
      writer.delete_documents_with_terms(vec![Term::from_text("field2", "b30")])?;
      writer.close()?;
      writer
    },
    1 => {
      drop(iw);
      let analyzer = MockAnalyzer::new(random);
      let writer = IndexWriter::new(
        dir,
        new_index_writer_config_with_analyzer(random, analyzer)?,
      )?;
      writer.force_merge(1)?;
      writer.close()?;
      writer
    },
    2 => {
      drop(iw);
      let analyzer = MockAnalyzer::new(random);
      let writer = IndexWriter::new(
        dir,
        new_index_writer_config_with_analyzer(random, analyzer)?,
      )?;
      writer.add_document(create_document(101, 4)?)?;
      writer.force_merge(1)?;
      writer.add_document(create_document(102, 4)?)?;
      writer.add_document(create_document(103, 4)?)?;
      writer.close()?;
      writer
    },
    3 => {
      drop(iw);
      let analyzer = MockAnalyzer::new(random);
      let writer = IndexWriter::new(
        dir,
        new_index_writer_config_with_analyzer(random, analyzer)?,
      )?;
      writer.add_document(create_document(101, 4)?)?;
      writer.close()?;
      writer
    },
    _ => iw,
  };
  Ok(iw)
}

fn assert_reader_closed<D>(reader: &StandardDirectoryReaderType<D>, _check_sub_readers: bool)
where
  D: Directory,
{
  assert_eq!(0, reader.get_ref_count());
  // TODO IMPORTANT StandardDirectoryReader::do_close未实现
  // if check_sub_readers {
  //   for sub_reader in reader.get_sequential_sub_readers() {
  //     assert_eq!(0, sub_reader.get_ref_count());
  //   }
  // }
}

fn assert_index_equals<D>(
  index1: &StandardDirectoryReaderType<D>,
  index2: &StandardDirectoryReaderType<D>,
) -> Result<()>
where
  D: Directory,
{
  assert_eq!(index1.num_docs()?, index2.num_docs()?);
  assert_eq!(index1.max_doc()?, index2.max_doc()?);
  assert_eq!(index1.has_deletions()?, index2.has_deletions()?);
  assert_eq!(
    get_context(index1)?.leaves()?.len() == 1,
    get_context(index2)?.leaves()?.len() == 1
  );

  let field_infos1 = field_infos::get_merged_field_infos(index1)?;
  let field_infos2 = field_infos::get_merged_field_infos(index2)?;
  assert_eq!(field_infos1.size(), field_infos2.size());
  for (field_info1, field_info2) in field_infos1.iter().zip(field_infos2.iter()) {
    assert_eq!(field_info1.name, field_info2.name);
  }

  for field_info in field_infos1.iter() {
    let cur_field = &field_info.name;
    let mut norms1 = MultiDocValues::get_norm_values(index1, cur_field)?;
    let mut norms2 = MultiDocValues::get_norm_values(index2, cur_field)?;
    if norms1.is_some() && norms2.is_some() {
      #[allow(clippy::unnecessary_unwrap)]
      let norms1 = norms1.as_mut().unwrap();
      #[allow(clippy::unnecessary_unwrap)]
      let norms2 = norms2.as_mut().unwrap();
      loop {
        let doc_id = norms1.next_doc()?;
        assert_eq!(doc_id, norms2.next_doc()?);
        if doc_id == NO_MORE_DOCS {
          break;
        }
        assert_eq!(norms1.long_value()?, norms2.long_value()?);
      }
    } else {
      assert!(norms1.is_none());
      assert!(norms2.is_none());
    }
  }

  let live_docs1 = multi_bits::get_live_docs(index1)?;
  let live_docs2 = multi_bits::get_live_docs(index2)?;
  for i in 0..index1.max_doc()? {
    assert_eq!(
      live_docs1
        .as_ref()
        .is_none_or(|live_docs| !live_docs.get(i as usize).expect("")),
      live_docs2
        .as_ref()
        .is_none_or(|live_docs| !live_docs.get(i as usize).expect("")),
      "Doc {} only deleted in one index.",
      i
    );
  }

  let mut stored_fields1 = index1.stored_fields()?;
  let mut stored_fields2 = index2.stored_fields()?;
  for i in 0..index1.max_doc()? {
    if live_docs1
      .as_ref()
      .is_none_or(|live_docs| live_docs.get(i as usize).expect(""))
    {
      let doc1 = stored_fields1.document(i)?;
      let doc2 = stored_fields2.document(i)?;
      assert_eq!(doc1.get_fields().len(), doc2.get_fields().len());
      for (field1, field2) in doc1.get_fields().iter().zip(doc2.get_fields().iter()) {
        assert_eq!(field1.name(), field2.name());
        assert_eq!(
          field1.string_value()?.map(|value| value.into_owned()),
          field2.string_value()?.map(|value| value.into_owned())
        );
      }
    }
  }

  let mut fields1: Vec<_> = field_infos::get_indexed_fields(index1)?
    .into_iter()
    .collect();
  let mut fields2: Vec<_> = field_infos::get_indexed_fields(index2)?
    .into_iter()
    .collect();
  fields1.sort();
  fields2.sort();
  let mut fenum2 = fields2.iter();
  for field1 in fields1 {
    assert_eq!(&field1, fenum2.next().unwrap());
    let terms1 = multi_terms::get_terms(index1, &field1)?;
    if terms1.is_none() {
      assert!(multi_terms::get_terms(index2, &field1)?.is_none());
      continue;
    }
    let terms1 = terms1.unwrap();
    let mut enum1 = terms1.iterator()?;

    let terms2 = multi_terms::get_terms(index2, &field1)?;
    assert!(terms2.is_some());
    let terms2 = terms2.unwrap();
    let mut enum2 = terms2.iterator()?;

    while enum1.next()?.is_some() {
      assert_eq!(enum1.term()?, enum2.next()?.unwrap());
      let mut tp1 = enum1.postings_with_flags(None, ALL as i32)?;
      let mut tp2 = enum2.postings_with_flags(None, ALL as i32)?;

      while tp1.next_doc()? != NO_MORE_DOCS {
        assert_ne!(NO_MORE_DOCS, tp2.next_doc()?);
        assert_eq!(tp1.doc_id(), tp2.doc_id());
        let freq = tp1.freq()?;
        assert_eq!(freq, tp2.freq()?);
        for _ in 0..freq {
          assert_eq!(tp1.next_position()?, tp2.next_position()?);
        }
      }
    }
  }
  assert!(fenum2.next().is_none());
  Ok(())
}

fn create_document(n: i32, num_fields: i32) -> Result<Document> {
  let mut value = format!("a{n}");
  let mut doc = Document::new();
  let mut custom_type2 = FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
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
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_to_type = HashMap::new();
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwc.set_index_deletion_policy(NoDeletionPolicy);
  iwc.set_max_buffered_docs(-1);
  iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 10)?);
  let writer = IndexWriter::new(dir.clone(), iwc)?;
  for i in 0..4 {
    let mut doc = Document::new();
    doc.add(new_string_field(
      &mut random,
      "id",
      i.to_string(),
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;
    let mut data = HashMap::new();
    data.insert("index".to_string(), i.to_string());
    writer.set_live_commit_data(data);
    writer.commit()?;
  }
  for i in 0..4 {
    writer.delete_documents_with_terms(vec![Term::from_text("id", i.to_string())])?;
    let mut data = HashMap::new();
    data.insert("index".to_string(), (4 + i).to_string());
    writer.set_live_commit_data(data);
    writer.commit()?;
  }
  writer.close()?;

  let mut r = directory_reader::open(dir.clone())?;
  assert_eq!(0, r.num_docs()?);

  let commits = directory_reader::list_commits(dir.clone())?;
  for commit in &commits {
    let r2 = directory_reader::open_if_changed_with_commit(&r, Some(commit), &writer)?.unwrap();

    let s = commit.get_user_data();
    let v = if s.is_empty() {
      // First commit created by IW
      -1
    } else {
      s.get("index")
        .ok_or_else(|| LuceneError::illegal_state("missing commit index"))?
        .parse::<i32>()
        .map_err(|err| LuceneError::illegal_state(err.to_string()))?
    };
    if v < 4 {
      assert_eq!(1 + v, r2.num_docs()?);
    } else {
      assert_eq!(7 - v, r2.num_docs()?);
    }
    r.close()?;
    r = r2;
  }
  r.close()?;
  Ok(())
}

#[test]
fn test_open_if_changed_nrt_to_commit() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_to_type = HashMap::new();
  // Can't use RIW because it randomly commits:
  let analyzer = MockAnalyzer::new(&mut random);
  let w = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer)?,
  )?;
  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "field",
    "value",
    Store::No,
    &mut field_to_type,
  )?);
  w.add_document(doc.clone())?;
  w.commit()?;
  let commits = directory_reader::list_commits(dir.clone())?;
  assert_eq!(1, commits.len());
  w.add_document(doc)?;
  let r = directory_reader::open_from_writer(&w)?;

  assert_eq!(2, r.num_docs()?);
  let r2 = directory_reader::open_if_changed_with_commit(&r, Some(&commits[0]), &w)?.unwrap();
  r.close()?;
  assert_eq!(1, r2.num_docs()?);
  w.close()?;
  r2.close()?;
  Ok(())
}

#[test]
fn test_over_dec_ref_during_reopen() -> Result<()> {
  let mut random = random();
  let dir = Arc::new(FailOnLiveDocsDirectory::new(ByteBuffersDirectory::new()));

  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwc.set_merge_policy(NoMergePolicy::default());
  let w = IndexWriter::new(dir.clone(), iwc)?;
  let mut doc = Document::new();
  doc.add(StringField::from_string("id", "id", Store::No)?);
  w.add_document(doc)?;
  doc = Document::new();
  doc.add(StringField::from_string("id", "id2", Store::No)?);
  w.add_document(doc)?;
  w.commit()?;

  // Open reader w/ one segment w/ 2 docs:
  let r = directory_reader::open(dir.clone())?;

  // Delete 1 doc from the segment:
  // System.out.println("TEST: now delete");
  w.delete_documents_with_terms(vec![Term::from_text("id", "id")])?;
  // System.out.println("TEST: now commit");
  w.commit()?;

  // Fail when reopen tries to open the live docs file:
  dir.set_fail_on_live_docs(true);

  // Now reopen:
  // System.out.println("TEST: now reopen");
  match directory_reader::open_if_changed(&r, &w) {
    Ok(_) => panic!("expected FakeIOException"),
    Err(LuceneError::IoWithPath { source, .. }) => {
      assert!(
        source
          .get_ref()
          .is_some_and(|source| source.is::<FakeIOException>()),
        "expected FakeIOException, got {source}"
      );
    },
    Err(err) => return Err(err),
  }

  let s = IndexSearcher::from_cr(r)?;
  assert_eq!(1, s.count(TermQuery::new(Term::from_text("id", "id")))?);

  s.get_index_reader().close()?;
  w.close()?;
  Ok(())
}

#[test]
fn test_npe_after_invalid_reindex1() -> Result<()> {
  let mut random = random();
  let dir = Arc::new(ByteBuffersDirectory::new());

  let analyzer = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
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
    new_index_writer_config_with_analyzer(&mut random, analyzer)?,
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

  let err = directory_reader::open_if_changed(&r, &w);
  assert!(matches!(err, Err(LuceneError::IllegalState(_))));

  r.close()?;
  Ok(())
}

#[test]
fn test_npe_after_invalid_reindex2() -> Result<()> {
  let mut random = random();
  let dir = Arc::new(ByteBuffersDirectory::new());

  let analyzer = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
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
    new_index_writer_config_with_analyzer(&mut random, analyzer)?,
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

  let err = directory_reader::open_if_changed(&r, &w);
  assert!(matches!(err, Err(LuceneError::IllegalState(_))));

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
  let dir = Arc::new(ByteBuffersDirectory::new());
  let analyzer = MockAnalyzer::new(&mut random);
  let mut w = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer)?,
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
  let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
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
  w.close()?;
  let err = directory_reader::open_if_changed(&r, &w);
  assert!(matches!(err, Err(LuceneError::IllegalState(_))));

  r.close()?;
  Ok(())
}

#[test]
fn test_reuse_unchanged_leaf_reader_on_dv_update() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut index_writer_config = new_index_writer_config(&mut random)?;
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
  let mut new_reader = directory_reader::open_if_changed(&reader, &writer)?.unwrap();
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

  new_reader = directory_reader::open_if_changed(&reader, &writer)?.unwrap();
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

#[derive(Debug)]
struct FakeIOException;

impl Display for FakeIOException {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "fake IOException")
  }
}

impl std::error::Error for FakeIOException {}

struct FailOnLiveDocsDirectory<D>
where
  D: Directory,
{
  delegate: D,
  id: Identity,
  fail_on_live_docs: AtomicBool,
  failed: AtomicBool,
}

impl<D> FailOnLiveDocsDirectory<D>
where
  D: Directory,
{
  fn new(delegate: D) -> Self {
    Self {
      delegate,
      id: Identity::new(),
      fail_on_live_docs: AtomicBool::new(false),
      failed: AtomicBool::new(false),
    }
  }

  fn set_fail_on_live_docs(&self, value: bool) {
    self.fail_on_live_docs.store(value, Ordering::SeqCst);
    if value {
      self.failed.store(false, Ordering::SeqCst);
    }
  }

  fn maybe_fail_live_docs(&self, name: &str) -> Result<()> {
    if name.ends_with(".liv")
      && self.fail_on_live_docs.load(Ordering::SeqCst)
      && !self.failed.swap(true, Ordering::SeqCst)
    {
      return Err(LuceneError::io(Error::other(FakeIOException)));
    }
    Ok(())
  }
}

impl<D> Display for FailOnLiveDocsDirectory<D>
where
  D: Directory,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "FailOnLiveDocsDirectory({})", self.delegate)
  }
}

impl<D> Closeable for FailOnLiveDocsDirectory<D>
where
  D: Directory,
{
  fn close(&mut self) -> Result<()> {
    self.delegate.close()
  }
}

impl<D> HasIdentity for FailOnLiveDocsDirectory<D>
where
  D: Directory,
{
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl<D> Directory for FailOnLiveDocsDirectory<D>
where
  D: Directory,
{
  fn list_all(&self) -> Result<Vec<String>> {
    self.delegate.list_all()
  }

  fn delete_file(&self, name: &str) -> Result<()> {
    self.delegate.delete_file(name)
  }

  fn file_length(&self, name: &str) -> Result<usize> {
    self.delegate.file_length(name)
  }

  type IndexOutput = D::IndexOutput;

  fn create_output(&self, name: &str, context: &IOContext) -> Result<Self::IndexOutput> {
    self.delegate.create_output(name, context)
  }

  fn create_temp_output(
    &self,
    prefix: &str,
    suffix: &str,
    context: &IOContext,
  ) -> Result<Self::IndexOutput> {
    self.delegate.create_temp_output(prefix, suffix, context)
  }

  fn sync(&self, names: &[String]) -> Result<()> {
    self.delegate.sync(names)
  }

  fn sync_metadata(&self) -> Result<()> {
    self.delegate.sync_metadata()
  }

  fn rename(&self, source: &str, dest: &str) -> Result<()> {
    self.delegate.rename(source, dest)
  }

  type IndexInput = D::IndexInput;

  fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInput> {
    self.maybe_fail_live_docs(name)?;
    self.delegate.open_input(name, context)
  }

  fn open_checksum_input(
    &self,
    name: &str,
  ) -> Result<BufferedChecksumIndexInput<Self::IndexInput>> {
    self.maybe_fail_live_docs(name)?;
    self.delegate.open_checksum_input(name)
  }

  type Lock = D::Lock;

  fn obtain_lock(&self, name: &str) -> Result<Self::Lock> {
    self.delegate.obtain_lock(name)
  }

  fn copy_from(&self, from: &impl Directory, src: &str, dest: &str, ctx: &IOContext) -> Result<()> {
    self.delegate.copy_from(from, src, dest, ctx)
  }

  fn delete_files_ignoring_exceptions(&self, files: &[String]) {
    self.delegate.delete_files_ignoring_exceptions(files)
  }

  fn get_pending_deletions(&self) -> Result<HashSet<String>> {
    self.delegate.get_pending_deletions()
  }

  #[cfg(debug_assertions)]
  fn is_fs_directory(&self) -> bool {
    self.delegate.is_fs_directory()
  }

  fn ensure_open(&self) -> Result<()> {
    self.delegate.ensure_open()
  }
}

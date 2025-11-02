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
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::{TextField, text};
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::term_query::TermQuery;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::store::base_directory_test_case::EXTRA_FILE_NAME;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    new_directory, new_field, new_index_writer_config, new_text_field, random,
};
use once_cell::sync::Lazy;
use rand::Rng;
use std::clone::Clone;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::vec;

static STORED_TEXT_TYPE: Lazy<FieldType> =
    Lazy::new(|| FieldType::from_ref(&text::TYPE_NOT_STORED.clone()).expect("should not fail"));
pub(crate) struct TestIndexWriter;
#[test]
fn test_doc_count() -> Result<()> {
    let mut random = random();
    let dir = Arc::new(new_directory(&mut random)?);
    let writer = IndexWriter::new(dir, new_index_writer_config(&mut random))?;

    let mut doc = Document::new();
    let field1 = TextField::with_string("content", "aaa", Store::Yes)?;
    doc.add(field1);
    writer.add_document(doc)?;

    // let mut doc = Document::new();
    // let field1 = TextField::with_string("content", "aaasdf", Store::Yes)?;
    // doc.add(field1);
    // writer.add_document(doc)?;

    let mut doc = Document::new();
    let field1 = TextField::with_string("content1", "aaa", Store::Yes)?;
    doc.add(field1);
    writer.add_document(doc)?;

    writer.commit()?;
    let reader = Arc::new(directory_reader_util::open_with_writer(&writer)?);
    let irc = get_context(reader)?;
    let mut index_searcher = IndexSearcher::new(irc)?;
    let term_query = TermQuery::new(Term::from_text("content1", "aaa"));
    let v = index_searcher.search(term_query, 10)?;
    assert_eq!(v.score_docs.len(), 1);
    assert_eq!(v.score_docs[0].doc, 1);
    let doc_stats = writer.get_doc_stats()?;
    assert_eq!(2, doc_stats.max_doc);
    assert_eq!(2, doc_stats.num_docs);
    writer.close()?;
    Ok(())
}
// Make sure we can flush segment w/ norms, then add empty doc (no norms) and flush
#[test]
fn test_empty_doc_after_flushing_real_doc() -> Result<()> {
    let mut random = random();
    let dir = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

    let mut doc = Document::new();

    let mut custom_type = FieldType::from_ref(&*text::TYPE_STORED)?;
    custom_type.set_store_term_vectors(true)?;
    custom_type.set_store_term_vector_positions(true)?;
    custom_type.set_store_term_vector_offsets(true)?;

    doc.add(new_field("field", "aaa", &custom_type)?);
    writer.add_document(doc)?;
    writer.commit()?;
    if cfg!(feature = "test_log_verbose") {
        println!("TEST: now add empty doc");
    }
    let empty_doc = Document::new();
    writer.add_document(empty_doc)?;
    writer.close()?;
    let reader = directory_reader_util::open(dir.clone())?;
    assert_eq!(2, reader.num_docs()?);

    Ok(())
}
#[test]
fn test_bad_segment() -> Result<()> {
    let mut random = random();
    let dir = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

    let mut doc = Document::new();
    let mut custom_type = FieldType::from_ref(&*text::TYPE_NOT_STORED)?;
    custom_type.set_store_term_vectors(true)?;
    doc.add(new_field("tvtest", "", &custom_type)?);

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
    let dir = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

    let mut doc = Document::new();
    let text = " a".repeat(10_000) + " x";
    doc.add(new_text_field("field", &text, Store::No)?);
    writer.add_document(doc)?;
    writer.close()?;

    let reader = directory_reader_util::open(dir.clone())?;
    let t = Term::from_text("field", "x");
    assert_eq!(1, reader.doc_freq(&t)?);
    Ok(())
}
#[test]
fn test_empty_field_name() -> Result<()> {
    let mut random = random();
    let dir = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

    let mut doc = Document::new();
    doc.add(new_text_field("", "a b c", Store::No)?);
    writer.add_document(doc)?;
    writer.close()?;

    Ok(())
}
#[test]
fn test_empty_field_name_terms() -> Result<()> {
    // TODO
    Ok(())
}
#[test]
fn test_empty_field_name_with_empty_term() -> Result<()> {
    // TODO
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
    let dir = Arc::new(new_directory(&mut random)?);
    let mock_index_writer = MockIndexWriter::new();
    let writer = IndexWriter::with_sub(
        dir.clone(),
        new_index_writer_config(&mut random),
        Some(mock_index_writer),
    )?;

    let mut doc = Document::new();
    let custom_type = FieldType::from_ref(&*text::TYPE_STORED)?;
    doc.add(new_field("field", "a field", &custom_type)?);
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

    let reader = directory_reader_util::open(dir.clone())?;
    assert_eq!(0, reader.num_docs()?);

    Ok(())
}
#[test]
fn test_negative_positions() -> Result<()> {
    // TODO
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
    let dir = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现 MockAnalyzer
    let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
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
fn test_delete_same_term_across_fields() -> Result<()> {
    let mut random = random();
    let dir = Arc::new(new_directory(&mut random)?);

    // TODO: 未实现 MockAnalyzer
    let iwc = new_index_writer_config(&mut random);
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(TextField::with_string("a", "foo", Store::No)?);
    writer.add_document(doc)?;

    writer.delete_documents_with_terms(vec![
        Term::from_text("a", "xxx"),
        Term::from_text("b", "foo"),
    ])?;

    let reader = directory_reader_util::open_with_writer(&writer)?;
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
    let dir = Arc::new(new_directory(&mut random)?);

    let config = new_index_writer_config(&mut random);
    // TODO: 没有定义flush条件
    let writer = IndexWriter::new(dir.clone(), config)?;

    let mut d = Document::new();
    d.add(StringField::with_string("id", "doc-0", Store::Yes)?);
    writer.add_document(d)?;
    writer.flush()?;

    let mut d = Document::new();
    d.add(StringField::with_string("id", "doc-1", Store::Yes)?);
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
    let dir = Arc::new(new_directory(&mut random)?);

    // TODO: 没有定义flush条件
    let config = new_index_writer_config(&mut random);
    let writer = IndexWriter::new(dir.clone(), config)?;

    let mut d = Document::new();
    d.add(StringField::with_string("id", "doc-0", Store::Yes)?);
    writer.add_document(d)?;

    let mut d = Document::new();
    d.add(StringField::with_string("id", "doc-1", Store::Yes)?);
    writer.add_document(d)?;

    let reader = Arc::new(directory_reader_util::open_with_writer(&writer)?);
    let context = get_context(reader)?;
    let segment_reader = context.leaves()?.first().unwrap().reader();
    let segment_info = segment_reader.get_segment_info();
    let original_info_id = segment_reader.get_original_segment_info_id();
    let clone_segment_infos = writer.clone_segment_infos()?;
    let original_info = clone_segment_infos.info(original_info_id).unwrap();

    assert_eq!(0, original_info.get_del_count());
    assert_eq!(0, segment_info.get_del_count());

    writer.delete_documents_with_terms(vec![Term::from_text("id", "doc-0")])?;
    writer.commit()?;
    let clone_segment_infos = writer.clone_segment_infos()?;
    let original_info = clone_segment_infos.info(original_info_id).unwrap();
    assert_eq!(0, segment_info.get_del_count());
    assert_eq!(1, original_info.get_del_count());

    assert!(Arc::ptr_eq(&original_info.info, &segment_info.info));

    writer.close()?;
    Ok(())
}

#[test]
fn test_pending_num_docs() -> Result<()> {
    let mut random = random();
    let dir = Arc::new(new_directory(&mut random)?);

    let num_docs = random.random_range(0..100);

    {
        let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
        for i in 0..num_docs {
            let mut d = Document::new();
            d.add(StringField::with_string("id", i.to_string(), Store::Yes)?);
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

pub(crate) fn add_doc<D, L, B>(writer: &IndexWriter<D, L, B>) -> Result<()>
where
    D: Directory,
    L: LiveIndexWriterConfig,
    B: IndexWriterBase,
{
    let mut doc = Document::new();
    doc.add(new_text_field("content", "aaa", Store::No)?);
    let _ = writer.add_document(doc)?;
    Ok(())
}
pub(crate) fn add_doc_with_index<D, L, B>(writer: &IndexWriter<D, L, B>, index: i32) -> Result<()>
where
    D: Directory,
    L: LiveIndexWriterConfig,
    B: IndexWriterBase,
{
    let mut doc = Document::new();
    doc.add(new_field(
        "content",
        format!("aaa {}", index),
        &STORED_TEXT_TYPE,
    )?);
    // doc.add(new_field("id", index.to_string(), &STORED_TEXT_TYPE)?);

    match writer.add_document(doc) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

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
use crate::core::document::text_field::{TextField, text};
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::term_query::TermQuery;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    new_directory, new_field, new_index_writer_config, new_text_field, random,
};
use once_cell::sync::Lazy;
use std::clone::Clone;
use std::sync::Arc;

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

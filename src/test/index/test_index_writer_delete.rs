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
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::term_query::TermQuery;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    new_directory, new_index_writer_config, new_searcher_with_reader, new_string_field,
    new_text_field, random,
};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestIndexWriterDelete;
#[test]
fn test_simple_case() -> Result<()> {
    let mut random = random();

    let keywords = ["1", "2"];
    let unindexed = ["Netherlands", "Italy"];
    let unstored = ["Amsterdam has lots of bridges", "Venice has lots of canals"];
    let text = ["Amsterdam", "Venice"];

    let dir = Arc::new(new_directory(&mut random)?);
    // TODO: MockAnalyzer 未实现
    let iwc = new_index_writer_config(&mut random);
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut custom1 = FieldType::new();
    custom1.set_stored(true)?;
    custom1.freeze();

    for i in 0..keywords.len() {
        let mut doc = Document::new();
        doc.add(StringField::with_string("id", keywords[i], Store::Yes)?);
        doc.add(Field::new("country", custom1.clone(), unindexed[i]));
        doc.add(TextField::with_string("contents", unstored[i], Store::No)?);
        doc.add(TextField::with_string("city", text[i], Store::Yes)?);
        writer.add_document(doc)?;
    }

    // TODO: force_merge 未实现
    // writer.force_merge(1)?;
    writer.commit()?;

    let term = Term::from_text("city", "Amsterdam");
    let mut hit_count = get_hit_count(dir.clone(), term.clone())?;
    assert_eq!(1, hit_count);

    writer.delete_documents_with_terms(vec![term.clone()])?;
    writer.commit()?;

    hit_count = get_hit_count(dir.clone(), term)?;
    assert_eq!(0, hit_count);

    writer.close()?;
    Ok(())
}
#[test]
fn test_non_ram_delete() -> Result<()> {
    let mut random = random();

    let dir = Arc::new(new_directory(&mut random)?);

    // TODO: MockAnalyzer 未实现
    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_max_buffered_docs(2);

    let mut modifier = IndexWriter::new(dir.clone(), iwc)?;

    let mut id = 0;
    let value = 100;

    for _ in 0..7 {
        id += 1;
        add_doc(&mut modifier, id, value)?;
    }

    modifier.commit()?;

    assert_eq!(0, modifier.get_num_buffered_documents());
    assert!(modifier.get_segment_count() > 0);

    modifier.commit()?;

    {
        let reader = directory_reader_util::open(dir.clone())?;
        assert_eq!(7, reader.num_docs()?);
    }

    modifier.delete_documents_with_terms(vec![Term::from_text("value", value.to_string())])?;

    modifier.commit()?;

    {
        let reader = directory_reader_util::open(dir.clone())?;
        assert_eq!(0, reader.num_docs()?);
    }

    modifier.close()?;
    Ok(())
}

fn update_doc<D, L, B>(modifier: &mut IndexWriter<D, L, B>, id: i32, value: i32) -> Result<()>
where
    D: Directory,
    L: LiveIndexWriterConfig,
    B: IndexWriterBase,
{
    let mut doc = Document::new();

    doc.add(new_text_field("content", "aaa", Store::No)?);
    doc.add(new_string_field("id", id.to_string(), Store::Yes)?);
    doc.add(new_string_field("value", value.to_string(), Store::No)?);
    doc.add(NumericDocValuesField::new("dv", value as i64));

    modifier.update_documents_with_term(Term::from_text("id", id.to_string()), doc)?;
    Ok(())
}

fn add_doc<D, L, B>(modifier: &mut IndexWriter<D, L, B>, id: i32, value: i32) -> Result<()>
where
    D: Directory,
    L: LiveIndexWriterConfig,
    B: IndexWriterBase,
{
    let mut doc = Document::new();

    doc.add(new_text_field("content", "aaa", Store::No)?);
    doc.add(new_string_field("id", id.to_string(), Store::Yes)?);
    doc.add(new_string_field("value", value.to_string(), Store::No)?);
    doc.add(NumericDocValuesField::new("dv", value as i64));

    modifier.add_document(doc)?;
    Ok(())
}

fn get_hit_count<D: Directory>(dir: Arc<D>, term: Term) -> Result<i64> {
    let reader = Arc::new(directory_reader_util::open(dir)?);
    let mut searcher = new_searcher_with_reader(reader.clone())?;
    let top_docs = searcher.search(TermQuery::new(term.clone()), 1000)?;
    Ok(top_docs.total_hits.value() as i64)
}

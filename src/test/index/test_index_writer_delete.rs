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
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::log_merge_policy::LogMergePolicy;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::term::Term;
use crate::core::search::term_query::TermQuery;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_index_writer_config, new_searcher_with_reader, new_string_field,
    new_text_field, random,
};
use std::collections::HashMap;
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

    let dir = new_directory_shared(&mut random)?;
    // TODO: MockAnalyzer 未实现
    let iwc = new_index_writer_config(&mut random);
    let modifier = IndexWriter::new(dir.clone(), iwc)?;

    let mut custom1 = FieldType::new();
    custom1.set_stored(true)?;
    custom1.freeze();

    for i in 0..keywords.len() {
        let mut doc = Document::new();
        doc.add(StringField::from_string("id", keywords[i], Store::Yes)?);
        doc.add(Field::new("country", unindexed[i], custom1.clone()));
        doc.add(TextField::from_string("contents", unstored[i], Store::No)?);
        doc.add(TextField::from_string("city", text[i], Store::Yes)?);
        modifier.add_document(doc)?;
    }

    modifier.force_merge(1)?;
    modifier.commit()?;

    let term = Term::from_text("city", "Amsterdam");
    let mut hit_count = get_hit_count(dir.clone(), term.clone())?;
    assert_eq!(1, hit_count);

    modifier.delete_documents_with_terms(vec![term.clone()])?;
    modifier.commit()?;

    hit_count = get_hit_count(dir.clone(), term)?;
    assert_eq!(0, hit_count);

    modifier.close()?;
    Ok(())
}
#[test]
fn test_non_ram_delete() -> Result<()> {
    let mut random = random();

    let dir = new_directory_shared(&mut random)?;
    let mut field_types = HashMap::new();

    // TODO: MockAnalyzer 未实现
    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_max_buffered_docs(2);

    let mut modifier = IndexWriter::new(dir.clone(), iwc)?;

    let mut id = 0;
    let value = 100;

    for _ in 0..7 {
        id += 1;
        add_doc(&mut random, &mut modifier, id, value, &mut field_types)?;
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
#[test]
fn test_ram_deletes() -> Result<()> {
    // TODO: FrozenBufferedUpdates#apply_query_deletes未实现
    Ok(())
}
#[test]
fn test_both_deletes() -> Result<()> {
    let mut random = random();

    let dir = new_directory_shared(&mut random)?;
    let mut field_types = HashMap::new();

    // TODO: MockAnalyzer 未实现
    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_max_buffered_docs(100);

    let mut writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut id = 0;
    let mut value = 100;

    // First 5 docs, value=100
    for _ in 0..5 {
        id += 1;
        add_doc(&mut random, &mut writer, id, value, &mut field_types)?;
    }

    value = 200;
    for _ in 0..5 {
        id += 1;
        add_doc(&mut random, &mut writer, id, value, &mut field_types)?;
    }

    writer.commit()?;

    for _ in 0..5 {
        id += 1;
        add_doc(&mut random, &mut writer, id, value, &mut field_types)?;
    }

    writer.delete_documents_with_terms(vec![Term::from_text("value", value.to_string())])?;
    writer.commit()?;

    {
        let reader = directory_reader_util::open(dir.clone())?;
        assert_eq!(5, reader.num_docs()?);
    }

    writer.close()?;
    Ok(())
}
#[test]
fn test_batch_deletes() -> Result<()> {
    let mut random = random();

    let dir = new_directory_shared(&mut random)?;
    let mut field_types = HashMap::new();

    // TODO: MockAnalyzer 未实现
    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_max_buffered_docs(2);

    let mut modifier = IndexWriter::new(dir.clone(), iwc)?;

    let mut id = 0;
    let value = 100;

    for _ in 0..7 {
        id += 1;
        add_doc(&mut random, &mut modifier, id, value, &mut field_types)?;
    }

    modifier.commit()?;

    {
        let reader = directory_reader_util::open(dir.clone())?;
        assert_eq!(7, reader.num_docs()?);
    }

    id = 0;

    modifier.delete_documents_with_terms(vec![
        Term::from_text("id", (id + 1).to_string()),
        Term::from_text("id", (id + 2).to_string()),
    ])?;
    id += 2;

    modifier.commit()?;

    {
        let reader = directory_reader_util::open(dir.clone())?;
        assert_eq!(5, reader.num_docs()?);
    }

    let mut terms = Vec::new();
    for _ in 0..3 {
        id += 1;
        terms.push(Term::from_text("id", id.to_string()));
    }

    modifier.delete_documents_with_terms(terms)?;
    modifier.commit()?;

    {
        let reader = directory_reader_util::open(dir.clone())?;
        assert_eq!(2, reader.num_docs()?);
    }
    modifier.close()?;
    Ok(())
}
#[test]
fn test_delete_all_simple() -> Result<()> {
    // TODO delete_all未实现
    Ok(())
}
#[test]
fn test_delete_all_no_dead_lock() -> Result<()> {
    // TODO 多线程未实现
    Ok(())
}
#[test]
fn test_delete_all_rollback() -> Result<()> {
    // TODO delete_all未实现
    Ok(())
}
#[test]
fn test_delete_all_nrt() -> Result<()> {
    // TODO delete_all未实现
    Ok(())
}
#[test]
fn test_delete_all_repeated() -> Result<()> {
    // TODO delete_all未实现
    Ok(())
}
fn update_doc<D, L, B, R>(
    random: &mut R,
    modifier: &mut IndexWriter<D, L, B>,
    id: i32,
    value: i32,
    field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
    D: Directory,
    L: LiveIndexWriterConfig,
    B: IndexWriterBase,
    R: rand::Rng + ?Sized,
{
    let mut doc = Document::new();

    doc.add(new_text_field(
        random,
        "content",
        "aaa",
        Store::No,
        field_types,
    )?);
    doc.add(new_string_field(
        random,
        "id",
        id.to_string(),
        Store::Yes,
        field_types,
    )?);
    doc.add(new_string_field(
        random,
        "value",
        value.to_string(),
        Store::No,
        field_types,
    )?);
    doc.add(NumericDocValuesField::new("dv", value as i64));

    modifier.update_documents_with_term(Term::from_text("id", id.to_string()), doc)?;
    Ok(())
}

fn add_doc<D, L, B, R>(
    random: &mut R,
    modifier: &mut IndexWriter<D, L, B>,
    id: i32,
    value: i32,
    field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
    D: Directory,
    L: LiveIndexWriterConfig,
    B: IndexWriterBase,
    R: rand::Rng + ?Sized,
{
    let mut doc = Document::new();

    doc.add(new_text_field(
        random,
        "content",
        "aaa",
        Store::No,
        field_types,
    )?);
    doc.add(new_string_field(
        random,
        "id",
        id.to_string(),
        Store::Yes,
        field_types,
    )?);
    doc.add(new_string_field(
        random,
        "value",
        value.to_string(),
        Store::No,
        field_types,
    )?);
    doc.add(NumericDocValuesField::new("dv", value as i64));

    modifier.add_document(doc)?;
    Ok(())
}

fn get_hit_count<D: Directory + 'static>(dir: Arc<D>, term: Term) -> Result<i64> {
    let reader = directory_reader_util::open(dir)?;
    let searcher = new_searcher_with_reader(reader)?;
    let top_docs = searcher.search(TermQuery::new(term.clone()), 1000)?;
    Ok(top_docs.total_hits.value() as i64)
}
#[test]
fn test_deletes_on_disk_full() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_updates_on_disk_full() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_error_after_apply_deletes() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_error_in_docs_writer_add() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_delete_null_query() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_delete_all_slowly() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_indexing_then_deleting() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_flush_pushed_deletes_by_ram() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_apply_deletes_on_flush() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_deletes_check_index_output() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_try_delete_document() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_nrt_is_current_after_delete() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_only_deletes_triggers_merge_on_close() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_only_deletes_triggers_merge_on_get_reader() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_only_deletes_triggers_merge_on_flush() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    // TODO: 未实现MockAnalyzer
    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_max_buffered_docs(2);
    let mut mp = LogMergePolicy::log_doc();
    mp.set_min_merge_docs(1);
    iwc.set_merge_policy(mp);
    iwc.set_merge_scheduler(SerialMergeScheduler::new());

    let w = IndexWriter::new(dir.clone(), iwc)?;
    let mut field_types = HashMap::new();

    for i in 0..38 {
        let mut doc = Document::new();
        doc.add(new_string_field(
            &mut random,
            "id",
            i.to_string(),
            Store::No,
            &mut field_types,
        )?);
        w.add_document(doc)?;
    }
    w.commit()?;

    // Deleting 18 out of the 20 docs in the first segment make it the same "level" as the other 9
    // which should cause a merge to kick off:
    for i in 0..18 {
        w.delete_documents_with_terms(vec![Term::from_text("id", i.to_string())])?;
    }

    let _ = directory_reader_util::open_from_writer(&w)?;
    let reader = directory_reader_util::open_from_writer(&w)?;
    let reader = get_context(reader)?;
    assert_eq!(1, reader.leaves()?.len());
    w.close()?;
    Ok(())
}

#[test]
fn test_only_deletes_delete_all_docs() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    // TODO: 未实现MockAnalyzer
    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_max_buffered_docs(2);

    let mut mp = LogMergePolicy::log_doc();
    mp.set_min_merge_docs(1);
    iwc.set_merge_policy(mp);

    iwc.set_merge_scheduler(SerialMergeScheduler::new());

    let w = IndexWriter::new(dir.clone(), iwc)?;
    let mut field_types = HashMap::new();
    for i in 0..38 {
        let mut doc = Document::new();
        doc.add(new_string_field(
            &mut random,
            "id",
            i.to_string(),
            Store::No,
            &mut field_types,
        )?);
        w.add_document(doc)?;
    }
    w.commit()?;

    for i in 0..38 {
        w.delete_documents_with_terms(vec![Term::from_text("id", i.to_string())])?;
    }

    let r = directory_reader_util::open_from_writer(&w)?;
    assert_eq!(0, r.max_doc()?);
    let reader = get_context(r)?;
    assert_eq!(0, reader.leaves()?.len());
    w.close()?;
    Ok(())
}
#[test]
fn test_merging_after_delete_all() -> Result<()> {
    // TODO
    Ok(())
}

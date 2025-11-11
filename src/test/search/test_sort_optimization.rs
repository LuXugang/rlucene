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
use crate::core::document::long_field::LongPoint;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::sort::Sort;
use crate::core::search::field_doc::FieldDoc;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::sort_field::{SortField, SortFieldType, SortFiledBase};
use crate::core::search::top_docs::TopDocsLike;
use crate::core::search::top_field_collector_manager::TopFieldCollectorManager;
use crate::core::search::total_hits::Relation;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    at_least, new_directory, new_searcher, random,
};
use rand::Rng;
use std::rc::Rc;
use std::sync::Arc;

#[allow(dead_code)]
struct TestSortOptimization;

#[test]
fn test_long_sort_optimization() -> Result<()> {
    let mut random = random();
    let dir = Arc::new(new_directory(&mut random)?);

    let config = IndexWriterConfig::new();
    let writer = IndexWriter::new(dir.clone(), config)?;

    // let num_docs = at_least(&mut random, 10_000);
    let num_docs = 11112;
    for i in 0..num_docs {
        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("my_field", i as i64));
        doc.add(LongPoint::new("my_field", vec![i as i64])?);
        writer.add_document(doc)?;
        if i == 7000 {
            writer.flush()?;
        }
    }

    let reader = Arc::new(directory_reader_util::open_with_writer(&writer)?);
    writer.close()?;
    // TODO：这里应该使用new_searcher的另一个变体
    let mut searcher = new_searcher(reader)?;
    let sort_field = SortField::new(Some("my_field"), SortFieldType::Long)?;
    let sort = Rc::new(Sort::with_fields(vec![sort_field.into()])?);
    let num_hits = 3;
    let total_hits_threshold = 3;
    // simple sort
    {
        let collector_manager =
            TopFieldCollectorManager::new(sort.clone(), num_hits, total_hits_threshold)?;

        let top_docs =
            searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager)?;

        assert_eq!(top_docs.score_docs().len(), num_hits as usize);

        for i in 0..num_hits {
            let field_doc = &top_docs.score_docs()[i as usize];
            let fields = &field_doc.fields()?[0];
            let value = *fields.as_i64().expect("should be i64");
            assert_eq!(i as i64, value);
        }

        assert_eq!(
            top_docs.total_hits().relation,
            Relation::GreaterThanOrEqualTo
        );

        assert_non_competitive_hits_are_skipped(
            top_docs.total_hits().value as i64,
            num_docs as i64,
        )?;
    }
    // paging sort with after
    {
        let after_value: i64 = 2;
        let after = FieldDoc::with_fields(2, f32::NAN, vec![after_value.into()]);
        let collector_manager = TopFieldCollectorManager::with_after(
            sort.clone(),
            num_hits,
            Some(after),
            total_hits_threshold,
        )?;

        let top_docs =
            searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager)?;

        assert_eq!(top_docs.score_docs().len(), num_hits as usize);

        for i in 0..num_hits {
            let field_doc = &top_docs.score_docs()[i as usize];
            let fields = &field_doc.fields()?[0];
            let value = *fields.as_i64().expect("should be i64");
            assert_eq!(after_value + 1 + i as i64, value);
        }

        assert_eq!(
            top_docs.total_hits().relation,
            Relation::GreaterThanOrEqualTo
        );

        assert_non_competitive_hits_are_skipped(
            top_docs.total_hits().value as i64,
            num_docs as i64,
        )?;
    }
    // test that if there is the secondary sort on _score, scores are filled correctly
    {
        let sort_field = SortField::new(Some("my_field"), SortFieldType::Long)?;
        let sort = Rc::new(Sort::with_fields(vec![
            sort_field.into(),
            SortField::get_field_score()?.into(),
        ])?);

        let collector_manager =
            TopFieldCollectorManager::new(sort.clone(), num_hits, total_hits_threshold)?;

        let top_docs =
            searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager)?;

        assert_eq!(top_docs.score_docs().len(), num_hits as usize);

        for i in 0..num_hits {
            let field_doc = &top_docs.score_docs()[i as usize];
            let fields = field_doc.fields()?;

            let long_val = *fields[0].as_i64().expect("should be i64");
            assert_eq!(i as i64, long_val);

            let score = *fields[1].as_f32().expect("should be f32");
            assert!((score - 1.0).abs() < 0.001);
        }

        assert_eq!(
            top_docs.total_hits().relation,
            Relation::GreaterThanOrEqualTo
        );

        assert_non_competitive_hits_are_skipped(
            top_docs.total_hits().value as i64,
            num_docs as i64,
        )?;
    }
    // test that if numeric field is a secondary sort, no optimization is run
    {
        let sort_field = SortField::new(Some("my_field"), SortFieldType::Long)?;
        let sort = Rc::new(Sort::with_fields(vec![
            SortField::get_field_score()?.into(),
            sort_field.into(),
        ])?);

        let collector_manager =
            TopFieldCollectorManager::new(sort.clone(), num_hits, total_hits_threshold)?;

        let top_docs =
            searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager)?;

        assert_eq!(top_docs.score_docs().len(), num_hits as usize);
        // assert that all documents were collected => optimization was not run
        assert_eq!(top_docs.total_hits().value as i32, num_docs);
    }

    Ok(())
}
/// test that even if a field is not indexed with points, optimized sort still works as expected,
/// although no optimization will be run
#[test]
fn test_long_sort_optimization_on_field_not_indexed_with_points() -> Result<()> {
    let mut random = random();
    let dir = Arc::new(new_directory(&mut random)?);
    let writer = IndexWriter::new(dir.clone(), IndexWriterConfig::new())?;

    let num_docs = at_least(&mut random, 100);
    // "my_field" is not indexed with points
    for i in 0..num_docs {
        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("my_field", i as i64));
        writer.add_document(doc)?;
    }

    let reader = Arc::new(directory_reader_util::open_with_writer(&writer)?);
    writer.close()?;

    // single-threaded so totalHits is deterministic
    // TODO: 这里应该使用new_searcher的另一个变体
    let mut searcher = new_searcher(reader)?;
    let sort_field = SortField::new(Some("my_field"), SortFieldType::Long)?;
    let sort = Rc::new(Sort::with_fields(vec![sort_field.into()])?);
    let num_hits = 3;
    let total_hits_threshold = 3;

    let collector_manager =
        TopFieldCollectorManager::new(sort.clone(), num_hits, total_hits_threshold)?;
    let top_docs = searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager)?;

    // sort still works and returns expected number of docs
    assert_eq!(top_docs.score_docs().len(), num_hits as usize);

    // returns expected values
    for i in 0..num_hits {
        let field_doc = &top_docs.score_docs()[i as usize];
        let fields = field_doc.fields()?;
        let long_val = *fields[0].as_i64().expect("should be i64");
        assert_eq!(i as i64, long_val);
    }

    // assert that all documents were collected => optimization was not run
    assert_eq!(top_docs.total_hits().value as i32, num_docs);

    Ok(())
}
#[test]
fn test_sort_optimization_with_missing_values() -> Result<()> {
    let mut random = random();
    let dir = Arc::new(new_directory(&mut random)?);

    let config = IndexWriterConfig::new();
    let writer = IndexWriter::new(dir.clone(), config)?;

    let num_docs = at_least(&mut random, 10_000);
    for i in 0..num_docs {
        let mut doc = Document::new();
        // miss values on every 500th document
        if i % 500 != 0 {
            doc.add(NumericDocValuesField::new("my_field", i as i64));
            doc.add(LongPoint::new("my_field", vec![i as i64])?);
        }
        writer.add_document(doc)?;
        if i == 7000 {
            writer.flush()?;
        }
    }

    let reader = Arc::new(directory_reader_util::open_with_writer(&writer)?);
    writer.close()?;
    // TODO: 这里应该使用new_searcher的另一个变体
    let mut searcher = new_searcher(reader)?;
    let num_hits = 3;
    let total_hits_threshold = 3;

    {
        let mut sort_field = SortField::new(Some("my_field"), SortFieldType::Long)?;
        sort_field.set_missing_value(0i64)?;
        let sort = Rc::new(Sort::with_fields(vec![sort_field.into()])?);
        let collector_manager =
            TopFieldCollectorManager::new(sort.clone(), num_hits, total_hits_threshold)?;
        let top_docs =
            searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager)?;
        assert_eq!(top_docs.score_docs().len(), num_hits as usize);
        assert_non_competitive_hits_are_skipped(
            top_docs.total_hits().value as i64,
            num_docs as i64,
        )?;
    }

    {
        let mut sf1 = SortField::new(Some("my_field1"), SortFieldType::Long)?;
        let mut sf2 = SortField::new(Some("my_field2"), SortFieldType::Long)?;
        sf1.set_missing_value(0i64)?;
        sf2.set_missing_value(0i64)?;
        let sort = Rc::new(Sort::with_fields(vec![sf1.into(), sf2.into()])?);
        let collector_manager =
            TopFieldCollectorManager::new(sort.clone(), num_hits, total_hits_threshold)?;
        let top_docs =
            searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager)?;
        assert_eq!(top_docs.score_docs().len(), num_hits as usize);
        assert_eq!(top_docs.total_hits().value as i32, num_docs as i32);
    }

    {
        let mut sort_field = SortField::new(Some("my_field"), SortFieldType::Long)?;
        sort_field.set_missing_value(100i64)?;
        let sort = Rc::new(Sort::with_fields(vec![sort_field.into()])?);
        let collector_manager =
            TopFieldCollectorManager::new(sort.clone(), num_hits, total_hits_threshold)?;
        let top_docs =
            searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager)?;
        assert_eq!(top_docs.score_docs().len(), num_hits as usize);
        assert_non_competitive_hits_are_skipped(
            top_docs.total_hits().value as i64,
            num_docs as i64,
        )?;
    }

    {
        let after_value = i64::MAX;
        let after = FieldDoc::with_fields(
            10 + random.random_range(0..1000),
            f32::NAN,
            vec![after_value.into()],
        );
        let mut sort_field = SortField::new(Some("my_field"), SortFieldType::Long)?;
        sort_field.set_missing_value(i64::MAX)?;
        let sort = Rc::new(Sort::with_fields(vec![sort_field.into()])?);
        let collector_manager = TopFieldCollectorManager::with_after(
            sort.clone(),
            num_hits,
            Some(after),
            total_hits_threshold,
        )?;
        let top_docs =
            searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager)?;
        assert_eq!(top_docs.score_docs().len(), num_hits as usize);
        assert_non_competitive_hits_are_skipped(
            top_docs.total_hits().value as i64,
            num_docs as i64,
        )?;
    }

    {
        let after_value = i64::MAX;
        let after = FieldDoc::with_fields(
            10 + random.random_range(0..1000),
            f32::NAN,
            vec![after_value.into()],
        );
        let mut sort_field = SortField::with_reverse(Some("my_field"), SortFieldType::Long, true)?;
        sort_field.set_missing_value(i64::MAX);
        let sort = Rc::new(Sort::with_fields(vec![sort_field.into()])?);
        let collector_manager = TopFieldCollectorManager::with_after(
            sort.clone(),
            num_hits,
            Some(after),
            total_hits_threshold,
        )?;
        let top_docs =
            searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager)?;
        assert_eq!(top_docs.score_docs().len(), num_hits as usize);
        assert_non_competitive_hits_are_skipped(
            top_docs.total_hits().value as i64,
            num_docs as i64,
        )?;
    }

    {
        let after_value: i64 = 3;
        let after = FieldDoc::with_fields(3, f32::NAN, vec![after_value.into()]);
        let mut sort_field = SortField::new(Some("my_field"), SortFieldType::Long)?;
        sort_field.set_missing_value(2i64)?;
        let sort = Rc::new(Sort::with_fields(vec![sort_field.into()])?);
        let collector_manager = TopFieldCollectorManager::with_after(
            sort.clone(),
            num_hits,
            Some(after),
            total_hits_threshold,
        )?;
        let top_docs =
            searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager)?;
        assert_eq!(top_docs.score_docs().len(), num_hits as usize);

        for i in 0..num_hits {
            let field_doc = &top_docs.score_docs()[i as usize];
            let fields = &field_doc.fields()?[0];
            let value = *fields.as_i64().expect("should be i64");
            assert_eq!(after_value + 1 + i as i64, value);
        }

        assert_eq!(
            top_docs.total_hits().relation,
            Relation::GreaterThanOrEqualTo
        );

        let expected_skipped = (7001 - 512 - 1) + (num_docs - 7001);
        assert_non_competitive_hits_are_skipped(
            top_docs.total_hits().value as i64,
            num_docs as i64 - expected_skipped as i64 + 1,
        )?;
    }

    Ok(())
}
#[test]
fn test_numeric_doc_values_optimization_with_missing_values() -> Result<()> {
    let mut random = random();
    let dir = Arc::new(new_directory(&mut random)?);

    let config = IndexWriterConfig::new();
    let writer = IndexWriter::new(dir.clone(), config)?;

    let num_docs = at_least(&mut random, 10_000);
    let miss_values_num_docs = num_docs / 2;

    for i in 0..num_docs {
        let mut doc = Document::new();
        if i > miss_values_num_docs {
            doc.add(NumericDocValuesField::new("my_field", i as i64));
            doc.add(LongPoint::new("my_field", vec![i as i64])?);
        }
        writer.add_document(doc)?;
    }

    let reader = Arc::new(directory_reader_util::open_with_writer(&writer)?);
    writer.close()?;
    // TODO: 这里应该使用new_searcher的另一个变体
    let mut searcher = new_searcher(reader)?;
    let num_hits = 3;
    let total_hits_threshold = 3;

    let top_docs1;
    let top_docs2;

    {
        let mut sort_field = SortField::with_reverse(Some("my_field"), SortFieldType::Long, true)?;
        sort_field.set_missing_value(0i64)?;
        let sort = Rc::new(Sort::with_fields(vec![sort_field.into()])?);

        let collector_manager =
            TopFieldCollectorManager::new(sort.clone(), num_hits, total_hits_threshold)?;
        top_docs1 =
            searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager)?;
        assert_non_competitive_hits_are_skipped(
            top_docs1.total_hits().value as i64,
            num_docs as i64,
        )?;
    }

    {
        let mut sort_field = SortField::with_reverse(Some("my_field"), SortFieldType::Long, true)?;
        sort_field.set_missing_value(0i64)?;
        sort_field.set_optimize_sort_with_points(false);
        let sort = Rc::new(Sort::with_fields(vec![sort_field.into()])?);

        let collector_manager =
            TopFieldCollectorManager::new(sort.clone(), num_hits, total_hits_threshold)?;
        top_docs2 =
            searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager)?;

        assert_eq!(top_docs1.score_docs().len(), top_docs2.score_docs().len());
        assert_eq!(top_docs1.score_docs().len(), num_hits as usize);

        for i in 0..num_hits {
            let fd1 = &top_docs1.score_docs()[i as usize];
            let fd2 = &top_docs2.score_docs()[i as usize];
            let v1 = fd1.fields()?[0].as_i64().unwrap();
            let v2 = fd2.fields()?[0].as_i64().unwrap();
            assert_eq!(v1, v2);
            assert_eq!(fd1.doc(), fd2.doc());
        }

        assert!(top_docs1.total_hits().value < top_docs2.total_hits().value);
    }

    {
        let mut sf1 = SortField::with_reverse(Some("my_field"), SortFieldType::Long, true)?;
        let mut sf2 = SortField::with_reverse(Some("other"), SortFieldType::Long, true)?;
        sf1.set_missing_value(0i64)?;
        sf2.set_missing_value(0i64)?;
        let sort = Rc::new(Sort::with_fields(vec![sf1.into(), sf2.into()])?);

        let collector_manager =
            TopFieldCollectorManager::new(sort.clone(), num_hits, total_hits_threshold)?;
        let top_docs =
            searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager)?;
        assert_eq!(top_docs.total_hits().value as i32, num_docs as i32);
    }

    Ok(())
}

fn assert_non_competitive_hits_are_skipped(collected_hits: i64, num_docs: i64) -> Result<()> {
    if collected_hits >= num_docs {
        return Err(LuceneError::illegal_state(format!(
            "Expected some non-competitive hits are skipped; got collected_hits={} num_docs={}",
            collected_hits, num_docs
        )));
    }
    Ok(())
}

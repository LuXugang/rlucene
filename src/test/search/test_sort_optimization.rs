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
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::sort_field::{SortField, SortFieldType};
use crate::core::search::top_docs::TopDocsLike;
use crate::core::search::top_field_collector_manager::TopFieldCollectorManager;
use crate::core::search::total_hits::Relation;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    at_least, new_directory, new_searcher, random,
};
use std::rc::Rc;
use std::sync::Arc;

struct TestSortOptimization;

fn test_long_sort_optimization() -> Result<()> {
    let mut random = random();
    let dir = Arc::new(new_directory(&mut random)?);

    let config = IndexWriterConfig::new();
    let writer = IndexWriter::new(dir.clone(), config)?;

    let num_docs = at_least(&mut random, 10_000);
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
            searcher.search_with_collector_manager(MatchAllDocsQuery, &collector_manager, None)?;

        assert_eq!(top_docs.score_docs().len(), num_hits as usize);

        for i in 0..num_hits {
            let field_doc = &top_docs.score_docs()[i as usize];
            let fields = &field_doc.fields()?[0];
            let value = fields.as_i32().expect("should be i32");
            assert_eq!(i, *value);
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

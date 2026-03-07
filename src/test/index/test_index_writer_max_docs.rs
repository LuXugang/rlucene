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
use crate::core::document::field::Store::No;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::term::Term;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::SortField;
use crate::core::search::sort_field::SortFieldType::Doc;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::util::error::lucene_error::Result;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    create_temp_dir_with_prefix, new_fs_directory, new_index_writer_config, new_string_field,
    random,
};
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
struct TestIndexWriterMaxDocs;
#[test]
fn test_exactly_at_true_limit() -> Result<()> {
    // let max_docs = MAX_DOCS;
    // TODO IMPORTANT 这个在Java Lucene需要执行特别长的时间, 我们这里置为一个较小的值, 以便测试能够在合理的时间内完成. 在Java Lucene中, 这个值被设置为MAX_DOCS, 也就是2^31 - 1.
    let max_docs = 1000000;
    let mut random = random();

    let dir = new_fs_directory(&mut random, create_temp_dir_with_prefix("2BDocs3")?)?;

    let iwc = new_index_writer_config(&mut random);
    let iw = IndexWriter::new(dir.clone(), iwc)?;

    let mut field_types = HashMap::new();
    let mut doc = Document::new();
    doc.add(new_string_field(
        &mut random,
        "field",
        "text",
        No,
        &mut field_types,
    )?);

    for _i in 0..max_docs {
        iw.add_document(doc.clone())?;
    }

    iw.commit()?;

    // first unoptimized, then optimized
    for _iter in 0..2 {
        let ir = directory_reader_util::open(dir.clone())?;
        assert_eq!(max_docs, ir.max_doc()?);
        assert_eq!(max_docs, ir.num_docs()?);

        let searcher = IndexSearcher::from_cr(ir)?;
        let collector_manager =
            TopScoreDocCollectorManager::with_after(10, None, i32::MAX as usize)?;

        let hits = searcher.search_with_collector_manager(
            TermQuery::new(Term::from_text("field", "text")),
            &collector_manager,
        )?;
        assert_eq!(max_docs as usize, hits.total_hits.value);

        // sort by docID reversed
        let sort = Sort::with_fields(vec![SortField::with_reverse::<String>(None, Doc, true)?])?;
        let hits2 = searcher.search_with_sort(
            TermQuery::new(Term::from_text("field", "text")),
            10,
            sort,
        )?;

        assert_eq!(max_docs as usize, hits2.total_hits().value);
        assert_eq!(10, hits2.score_docs().len());
        assert_eq!(max_docs - 1, hits2.score_docs()[0].doc());

        iw.force_merge(1)?;
    }

    iw.close()?;
    Ok(())
}

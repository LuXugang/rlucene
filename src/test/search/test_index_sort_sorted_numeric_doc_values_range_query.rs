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
use crate::core::document::long_point::LongPoint;
use crate::core::document::sorted_numeric_doc_values_field::{
    SortedNumericDocValuesField, sorted_numeric_doc_values_field_util,
};
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::query_timeout::QueryTimeout;
use crate::core::index::sort::Sort;
use crate::core::search::QueryCache;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::Query;
use crate::core::search::query_caching_policy::QueryCachingPolicy;
use crate::core::search::score_doc::ScoreDocLike;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::sort_field::{SortFieldType, SortFiledBase};
use crate::core::search::sorted_numeric_sort_field::SortedNumericSortField;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::error::lucene_error::Result;
use crate::test::index::random_index_writer::RandomIndexWriter;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    at_least, new_directory, new_searcher_with_reader, random,
};
use crate::test::util::test_util::TestUtil;
use rand::Rng;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestIndexSortSortedNumericDocValuesRangeQuery;
#[test]
fn test_same_hits_as_point_range_query() -> Result<()> {
    let mut random = random();
    let iters = at_least(&mut random, 10);

    for _iter in 0..iters {
        let dir = Arc::new(new_directory(&mut random)?);
        // TODO: 未实现MockAnalyzer
        let mut iwc = IndexWriterConfig::new();

        let reverse = random.random_bool(0.5);
        let mut sort_field =
            SortedNumericSortField::with_reverse("dv", SortFieldType::Long, reverse)?;

        let enable_missing_value = random.random_bool(0.5);
        if enable_missing_value {
            let missing_value = if random.random_bool(0.5) {
                TestUtil::next_long(&mut random, -100, 10000)
            } else if random.random_bool(0.5) {
                i64::MIN
            } else {
                i64::MAX
            };
            sort_field.set_missing_value(missing_value)?;
        }

        let sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(sort)?;

        let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

        let num_docs = at_least(&mut random, 100);
        for _i in 0..num_docs {
            let mut doc = Document::new();
            let num_values = TestUtil::next_int(&mut random, 0, 1);

            for _ in 0..num_values {
                let value = TestUtil::next_long(&mut random, -100, 10000);
                doc.add(SortedNumericDocValuesField::new("dv", value));
                doc.add(LongPoint::new("idx", vec![value])?);
            }

            iw.add_document(doc)?;
        }

        // TODO delete by query 未实现
        // Optional delete
        // if random.random_bool(0.5) {
        //     iw.delete_documents_query(LongPoint::new_range_query("idx", vec![0], vec![10])?)?;
        // }

        let reader = Arc::new(iw.get_reader()?);
        let mut searcher = new_searcher_with_reader(reader.clone())?;
        iw.close()?;

        for _i in 0..100 {
            let min = if random.random_bool(0.5) {
                i64::MIN
            } else {
                TestUtil::next_long(&mut random, -100, 10000)
            };
            let max = if random.random_bool(0.5) {
                i64::MAX
            } else {
                TestUtil::next_long(&mut random, -100, 10000)
            };

            let q1 = LongPoint::new_range_query("idx", vec![min], vec![max])?;
            let q2 = sorted_numeric_doc_values_field_util::new_slow_range_query("dv", min, max);

            assert_same_hits(&mut searcher, q1, q2, false)?;
        }
    }

    Ok(())
}

fn assert_same_hits<S, IRC, QT, QCP, QC, T1, T2>(
    searcher: &mut IndexSearcher<IRC, S, QT, QCP, QC>,
    q1: T1,
    q2: T2,
    scores: bool,
) -> Result<()>
where
    IRC: IndexReaderContext,
    S: Similarity,
    QT: QueryTimeout,
    QCP: QueryCachingPolicy,
    QC: QueryCache<IRC::LeafReader>,
    T1: Into<Query>,
    T2: Into<Query>,
{
    let irc = searcher.get_top_reader_context();
    let max_doc = irc.reader().max_doc()?;

    let sort = if scores {
        Arc::new(Sort::get_relevance()?)
    } else {
        Arc::new(Sort::get_index_order()?)
    };

    let td1 = searcher.search_with_sort(q1, max_doc, sort.clone())?;
    let td2 = searcher.search_with_sort(q2, max_doc, sort)?;
    assert_eq!(td1.total_hits().value(), td2.total_hits().value());

    for i in 0..td1.score_docs().len() {
        let sd1 = &td1.score_docs()[i];
        let sd2 = &td2.score_docs()[i];

        assert_eq!(sd1.doc(), sd2.doc());

        if scores {
            let diff = (sd1.score() - sd2.score()).abs();
            assert!(diff <= 1e-6, "score diff={} idx={}", diff, i);
        }
    }

    Ok(())
}
// TODO IndexSortSortedNumericDocValuesRangeQuery 未实现还有很多测试未实现

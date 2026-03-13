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
use crate::core::document::long_point::LongPoint;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::BytesRef;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::field_exists_query::FieldExistsQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::score_doc::ScoreDocLike;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::{SortField, SortFieldType};
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::numeric_utils::NumericUtils;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::query_utils::QueryUtils;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    at_least, new_bytes_ref_from_bytes, new_bytes_ref_from_string, new_directory_shared,
    new_searcher_with_reader, new_searcher_with_wrap, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestDocValuesQueries;

#[test]
fn test_duel_point_range_sorted_numeric_range_query() -> Result<()> {
    do_test_duel_point_range_numeric_range_query(true, 1, false)
}

#[test]
fn test_duel_point_range_sorted_numeric_range_with_slipper_query() -> Result<()> {
    do_test_duel_point_range_numeric_range_query(true, 1, true)
}

#[test]
fn test_duel_point_range_multivalued_sorted_numeric_range_query() -> Result<()> {
    do_test_duel_point_range_numeric_range_query(true, 3, false)
}

#[test]
fn test_duel_point_range_multivalued_sorted_numeric_range_with_skipper_query() -> Result<()> {
    do_test_duel_point_range_numeric_range_query(true, 3, true)
}

#[test]
fn test_duel_point_range_numeric_range_query() -> Result<()> {
    do_test_duel_point_range_numeric_range_query(false, 1, false)
}

#[test]
fn test_duel_point_range_numeric_range_with_skipper_query() -> Result<()> {
    do_test_duel_point_range_numeric_range_query(false, 1, true)
}

#[test]
fn test_duel_point_numeric_sorted_with_skipper_range_query() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mut config = IndexWriterConfig::new();
    let reverse = random.random_bool(0.5);
    config.set_index_sort(Sort::with_fields(vec![SortField::with_reverse(
        Some("dv"),
        SortFieldType::Long,
        reverse,
    )?])?)?;
    let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), config);

    let num_docs = at_least(&mut random, 1000);
    for _ in 0..num_docs {
        let value = TestUtil::next_long(&mut random, -100, 10000);
        let mut doc = Document::new();

        doc.add(NumericDocValuesField::indexed_field("dv", value));
        doc.add(LongPoint::new("idx", vec![value])?);
        iw.add_document(doc)?;
    }

    let reader = iw.get_reader()?;
    let searcher = new_searcher_with_wrap(reader, false)?;
    iw.close()?;

    for _ in 0..100 {
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

        let q1 = LongPoint::new_range_query("idx", min, max)?;
        let q2 = NumericDocValuesField::new_slow_range_query("dv", min, max);

        assert_same_matches(&searcher, q1, q2, false)?;
    }

    Ok(())
}
fn do_test_duel_point_range_numeric_range_query(
    sorted_numeric: bool,
    max_values_per_doc: i32,
    skipper: bool,
) -> Result<()> {
    let mut random = random();
    let iters = at_least(&mut random, 10);

    for _ in 0..iters {
        let dir = new_directory_shared(&mut random)?;

        let iw = if sorted_numeric || random.random_bool(0.5) {
            RandomIndexWriter::new(&mut random, dir.clone())
        } else {
            let mut config = IndexWriterConfig::new();
            let reverse = random.random_bool(0.5);
            config.set_index_sort(Sort::with_fields(vec![SortField::with_reverse(
                Some("dv"),
                SortFieldType::Long,
                reverse,
            )?])?)?;
            RandomIndexWriter::with_config(&mut random, dir.clone(), config)
        };

        let num_docs = at_least(&mut random, 100);

        for _ in 0..num_docs {
            let mut doc = Document::new();
            let num_values = TestUtil::next_int(&mut random, 0, max_values_per_doc);

            for _ in 0..num_values {
                let value = TestUtil::next_long(&mut random, -100, 10000);

                if sorted_numeric {
                    if skipper {
                        doc.add(SortedNumericDocValuesField::indexed_field("dv", value));
                    } else {
                        doc.add(SortedNumericDocValuesField::new("dv", value));
                    }
                } else if skipper {
                    doc.add(NumericDocValuesField::indexed_field("dv", value));
                } else {
                    doc.add(NumericDocValuesField::new("dv", value));
                }

                doc.add(LongPoint::new("idx", vec![value])?);
            }

            iw.add_document(doc)?;
        }

        // TODO delete by query 未实现
        // if random.random_bool(0.5) {
        //     let del_query = LongPoint::new_range_query("idx", vec![0], vec![10])?;
        //     iw.delete_documents(del_query)?;
        // }

        let reader = iw.get_reader()?;
        let searcher = new_searcher_with_wrap(reader, false)?;
        iw.close()?;

        for _ in 0..100 {
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

            let q1 = LongPoint::new_range_query("idx", min, max)?;

            let q2 = if sorted_numeric {
                SortedNumericDocValuesField::new_slow_range_query("dv", min, max)
            } else {
                NumericDocValuesField::new_slow_range_query("dv", min, max)
            };

            assert_same_matches(&searcher, q1, q2, false)?;
        }
    }

    Ok(())
}
fn do_test_duel_point_range_sorted_range_query(
    sorted_set: bool,
    max_values_per_doc: i32,
    skipper: bool,
) -> Result<()> {
    let mut random = random();
    let iters = at_least(&mut random, 10);

    for _ in 0..iters {
        let dir = new_directory_shared(&mut random)?;

        let iw = if sorted_set || random.random_bool(0.5) {
            RandomIndexWriter::new(&mut random, dir.clone())
        } else {
            let mut config = IndexWriterConfig::new();
            let reverse = random.random_bool(0.5);
            config.set_index_sort(Sort::with_fields(vec![SortField::with_reverse(
                Some("dv"),
                SortFieldType::String,
                reverse,
            )?])?)?;
            RandomIndexWriter::with_config(&mut random, dir.clone(), config)
        };

        let num_docs = at_least(&mut random, 100);

        for _ in 0..num_docs {
            let mut doc = Document::new();
            let num_values = TestUtil::next_int(&mut random, 0, max_values_per_doc);

            for _ in 0..num_values {
                let value = TestUtil::next_long(&mut random, -100, 10000);

                let mut encoded = vec![0u8; 8];
                LongPoint::encode_dimension(value, &mut encoded, 0);

                if sorted_set {
                    if skipper {
                        doc.add(SortedSetDocValuesField::indexed_field(
                            "dv",
                            new_bytes_ref_from_bytes(&mut random, encoded.as_ref())?,
                        ));
                    } else {
                        doc.add(SortedSetDocValuesField::new(
                            "dv",
                            new_bytes_ref_from_bytes(&mut random, encoded.as_ref())?,
                        ));
                    }
                } else if skipper {
                    doc.add(SortedDocValuesField::indexed_field(
                        "dv",
                        new_bytes_ref_from_bytes(&mut random, encoded.as_ref())?,
                    ));
                } else {
                    doc.add(SortedDocValuesField::new(
                        "dv",
                        new_bytes_ref_from_bytes(&mut random, encoded.as_ref())?,
                    ));
                }

                doc.add(LongPoint::new("idx", vec![value])?);
            }

            iw.add_document(doc)?;
        }

        // TODO delete by query 未实现
        // if random.random_bool(0.5) {
        //     let del_query = LongPoint::new_range_query("idx", vec![0], vec![10])?;
        //     iw.ded(del_query)?;
        // }

        let reader = iw.get_reader()?;
        let searcher = new_searcher_with_wrap(reader, false)?;
        iw.close()?;

        for _ in 0..100 {
            let mut min = if random.random_bool(0.5) {
                i64::MIN
            } else {
                TestUtil::next_long(&mut random, -100, 10000)
            };
            let mut max = if random.random_bool(0.5) {
                i64::MAX
            } else {
                TestUtil::next_long(&mut random, -100, 10000)
            };

            // encoded boundaries
            let mut encoded_min = vec![0u8; 8];
            let mut encoded_max = vec![0u8; 8];
            LongPoint::encode_dimension(min, encoded_min.as_mut(), 0);
            LongPoint::encode_dimension(max, encoded_max.as_mut(), 0);

            let mut include_min = true;
            let mut include_max = true;

            if random.random_bool(0.5) {
                include_min = false;
                min += 1;
            }

            if random.random_bool(0.5) {
                include_max = false;
                max -= 1;
            }

            let q1 = LongPoint::new_range_query("idx", min, max)?;

            let q2 = if sorted_set {
                SortedSetDocValuesField::new_slow_range_query(
                    "dv",
                    if min == i64::MIN && random.random_bool(0.5) {
                        None
                    } else {
                        Some(new_bytes_ref_from_bytes(&mut random, encoded_min.as_ref())?)
                    },
                    if max == i64::MAX && random.random_bool(0.5) {
                        None
                    } else {
                        Some(new_bytes_ref_from_bytes(&mut random, encoded_max.as_ref())?)
                    },
                    include_min,
                    include_max,
                )
            } else {
                SortedDocValuesField::new_slow_range_query(
                    "dv",
                    if min == i64::MIN && random.random_bool(0.5) {
                        None
                    } else {
                        Some(new_bytes_ref_from_bytes(&mut random, encoded_min.as_ref())?)
                    },
                    if max == i64::MAX && random.random_bool(0.5) {
                        None
                    } else {
                        Some(new_bytes_ref_from_bytes(&mut random, encoded_max.as_ref())?)
                    },
                    include_min,
                    include_max,
                )
            };

            assert_same_matches(&searcher, q1, q2, false)?;
        }
    }

    Ok(())
}

#[test]
fn test_duel_point_range_sorted_set_range_query() -> Result<()> {
    do_test_duel_point_range_sorted_range_query(true, 1, false)
}

#[test]
fn test_duel_point_range_sorted_set_range_skipper_query() -> Result<()> {
    do_test_duel_point_range_sorted_range_query(true, 1, true)
}

#[test]
fn test_duel_point_range_multivalued_sorted_set_range_query() -> Result<()> {
    do_test_duel_point_range_sorted_range_query(true, 3, false)
}

#[test]
fn test_duel_point_range_multivalued_sorted_set_range_skipper_query() -> Result<()> {
    do_test_duel_point_range_sorted_range_query(true, 3, true)
}

#[test]
fn test_duel_point_range_sorted_range_query() -> Result<()> {
    do_test_duel_point_range_sorted_range_query(false, 1, false)
}

#[test]
fn test_duel_point_range_sorted_range_skipper_query() -> Result<()> {
    do_test_duel_point_range_sorted_range_query(false, 1, true)
}
#[test]
fn test_duel_point_sorted_set_sorted_with_skipper_range_query() -> Result<()> {
    let mut random = random();

    let dir = new_directory_shared(&mut random)?;

    let mut config = IndexWriterConfig::new();
    let reverse = random.random_bool(0.5);
    config.set_index_sort(Sort::with_fields(vec![SortField::with_reverse(
        Some("dv"),
        SortFieldType::String,
        reverse,
    )?])?)?;
    let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), config);

    let num_docs = at_least(&mut random, 1000);
    for _ in 0..num_docs {
        let value = TestUtil::next_long(&mut random, -100, 10000);

        // encode value → BytesRef
        let mut encoded = vec![0u8; 8];
        LongPoint::encode_dimension(value, &mut encoded, 0);

        let mut doc = Document::new();
        doc.add(SortedDocValuesField::indexed_field(
            "dv",
            new_bytes_ref_from_bytes(&mut random, encoded.as_ref())?,
        ));

        doc.add(LongPoint::new("idx", vec![value])?);

        iw.add_document(doc)?;
    }

    let reader = iw.get_reader()?;
    let searcher = new_searcher_with_wrap(reader, false)?;
    iw.close()?;

    for _ in 0..100 {
        let mut min = if random.random_bool(0.5) {
            i64::MIN
        } else {
            TestUtil::next_long(&mut random, -100, 10000)
        };

        let mut max = if random.random_bool(0.5) {
            i64::MAX
        } else {
            TestUtil::next_long(&mut random, -100, 10000)
        };

        let mut encoded_min = vec![0u8; 8];
        let mut encoded_max = vec![0u8; 8];
        LongPoint::encode_dimension(min, encoded_min.as_mut(), 0);
        LongPoint::encode_dimension(max, encoded_max.as_mut(), 0);

        let mut include_min = true;
        let mut include_max = true;

        if random.random_bool(0.5) {
            include_min = false;
            min += 1;
        }

        if random.random_bool(0.5) {
            include_max = false;
            max -= 1;
        }

        let q1 = LongPoint::new_range_query("idx", min, max)?;

        let q2 = SortedDocValuesField::new_slow_range_query(
            "dv",
            if min == i64::MIN && random.random_bool(0.5) {
                None
            } else {
                Some(new_bytes_ref_from_bytes(&mut random, encoded_min.as_ref())?)
            },
            if max == i64::MAX && random.random_bool(0.5) {
                None
            } else {
                Some(new_bytes_ref_from_bytes(&mut random, encoded_max.as_ref())?)
            },
            include_min,
            include_max,
        );

        assert_same_matches(&searcher, q1, q2, false)?;
    }

    Ok(())
}

fn assert_same_matches<IRC, T1, T2>(
    searcher: &IndexSearcher<IRC>,
    q1: T1,
    q2: T2,
    scores: bool,
) -> Result<()>
where
    IRC: IndexReaderContext,
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

    let td1 = searcher.search_with_sort(q1, max_doc.try_convert()?, sort.clone())?;
    let td2 = searcher.search_with_sort(q2, max_doc.try_convert()?, sort)?;
    assert_eq!(td1.total_hits().value(), td2.total_hits().value());

    for i in 0..td1.score_docs().len() {
        let sd1 = &td1.score_docs()[i];
        let sd2 = &td2.score_docs()[i];

        assert_eq!(sd1.doc(), sd2.doc());

        if scores {
            let sd1_score = sd1.score();
            let sd2_score = sd2.score();
            if sd1_score.is_nan() && sd2_score.is_nan() {
                // true
                continue;
            } else {
                let diff = (sd1_score - sd2_score).abs();
                assert!(diff <= 1e-6, "score diff={} idx={}", diff, i);
            }
        }
    }

    Ok(())
}

#[test]
fn test_equals() -> Result<()> {
    let mut random = random();
    let q1 = SortedNumericDocValuesField::new_slow_range_query("foo", 3, 5);

    QueryUtils::check_equal(
        &q1,
        &SortedNumericDocValuesField::new_slow_range_query("foo", 3, 5),
    );

    QueryUtils::check_unequal(
        &q1,
        &SortedNumericDocValuesField::new_slow_range_query("foo", 3, 6),
    );

    QueryUtils::check_unequal(
        &q1,
        &SortedNumericDocValuesField::new_slow_range_query("foo", 4, 5),
    );

    QueryUtils::check_unequal(
        &q1,
        &SortedNumericDocValuesField::new_slow_range_query("bar", 3, 5),
    );

    let q2 = SortedSetDocValuesField::new_slow_range_query(
        "foo",
        Some(new_bytes_ref_from_string(&mut random, "bar")?),
        Some(new_bytes_ref_from_string(&mut random, "baz")?),
        true,
        true,
    );

    QueryUtils::check_equal(
        &q2,
        &SortedSetDocValuesField::new_slow_range_query(
            "foo",
            Some(new_bytes_ref_from_string(&mut random, "bar")?),
            Some(new_bytes_ref_from_string(&mut random, "baz")?),
            true,
            true,
        ),
    );

    QueryUtils::check_unequal(
        &q2,
        &SortedSetDocValuesField::new_slow_range_query(
            "foo",
            Some(new_bytes_ref_from_string(&mut random, "baz")?),
            Some(new_bytes_ref_from_string(&mut random, "baz")?),
            true,
            true,
        ),
    );

    QueryUtils::check_unequal(
        &q2,
        &SortedSetDocValuesField::new_slow_range_query(
            "foo",
            Some(new_bytes_ref_from_string(&mut random, "bar")?),
            Some(new_bytes_ref_from_string(&mut random, "bar")?),
            true,
            true,
        ),
    );

    QueryUtils::check_unequal(
        &q2,
        &SortedSetDocValuesField::new_slow_range_query(
            "quux",
            Some(new_bytes_ref_from_string(&mut random, "bar")?),
            Some(new_bytes_ref_from_string(&mut random, "baz")?),
            true,
            true,
        ),
    );
    Ok(())
}

#[test]
fn test_to_string() -> Result<()> {
    let mut random = random();
    let q1 = SortedNumericDocValuesField::new_slow_range_query("foo", 3, 5);

    assert_eq!("foo:[3 TO 5]", q1.as_string("")?);
    assert_eq!("[3 TO 5]", q1.as_string("foo")?);
    assert_eq!("foo:[3 TO 5]", q1.as_string("bar")?);

    let q2 = SortedSetDocValuesField::new_slow_range_query(
        "foo",
        Some(new_bytes_ref_from_string(&mut random, "bar")?),
        Some(new_bytes_ref_from_string(&mut random, "baz")?),
        true,
        true,
    );
    assert_eq!("foo:[[62 61 72] TO [62 61 7a]]", q2.as_string("")?);

    let q2 = SortedSetDocValuesField::new_slow_range_query(
        "foo",
        Some(new_bytes_ref_from_string(&mut random, "bar")?),
        Some(new_bytes_ref_from_string(&mut random, "baz")?),
        false,
        true,
    );
    assert_eq!("foo:{[62 61 72] TO [62 61 7a]]", q2.as_string("")?);

    let q2 = SortedSetDocValuesField::new_slow_range_query(
        "foo",
        Some(new_bytes_ref_from_string(&mut random, "bar")?),
        Some(new_bytes_ref_from_string(&mut random, "baz")?),
        false,
        false,
    );
    assert_eq!("foo:{[62 61 72] TO [62 61 7a]}", q2.as_string("")?);

    let q2 = SortedSetDocValuesField::new_slow_range_query(
        "foo",
        Some(new_bytes_ref_from_string(&mut random, "bar")?),
        None,
        true,
        true,
    );
    assert_eq!("foo:[[62 61 72] TO *}", q2.as_string("")?);

    let q2 = SortedSetDocValuesField::new_slow_range_query(
        "foo",
        None,
        Some(new_bytes_ref_from_string(&mut random, "baz")?),
        true,
        true,
    );
    assert_eq!("foo:{* TO [62 61 7a]]", q2.as_string("")?);
    assert_eq!("{* TO [62 61 7a]]", q2.as_string("foo")?);
    assert_eq!("foo:{* TO [62 61 7a]]", q2.as_string("bar")?);

    Ok(())
}

#[test]
fn test_missing_field() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let iw = RandomIndexWriter::new(&mut random, dir.clone());
    iw.add_document(Document::new())?;
    let reader = iw.get_reader()?;
    iw.close()?;

    let searcher = new_searcher_with_reader(reader)?;
    let leaves = searcher.get_top_reader_context().leaves()?;
    let ctx = &leaves[0];

    let queries: Vec<Query> = vec![
        NumericDocValuesField::new_slow_range_query("foo", 2, 4).into(),
        SortedNumericDocValuesField::new_slow_range_query("foo", 2, 4).into(),
        SortedDocValuesField::new_slow_range_query(
            "foo",
            Some(BytesRef::from_string("abc")),
            Some(BytesRef::from_string("bcd")),
            random.random_bool(0.5),
            random.random_bool(0.5),
        )
        .into(),
        SortedSetDocValuesField::new_slow_range_query(
            "foo",
            Some(BytesRef::from_string("abc")),
            Some(BytesRef::from_string("bcd")),
            random.random_bool(0.5),
            random.random_bool(0.5),
        )
        .into(),
    ];

    for query in queries {
        let rewritten = searcher.rewrite(query)?;
        let w = searcher.create_weight(rewritten, ScoreMode::Complete, 1.0)?;
        assert!(w.scorer(ctx, &searcher)?.is_none());
    }

    Ok(())
}
#[test]
fn test_slow_range_query_rewrite() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let iw = RandomIndexWriter::new(&mut random, dir.clone());
    let reader = iw.get_reader()?;
    iw.close()?;
    let searcher = new_searcher_with_reader(reader)?;

    QueryUtils::check_equal(
        &NumericDocValuesField::new_slow_range_query("foo", 10, 1).rewrite(&searcher)?,
        &MatchNoDocsQuery::new().into(),
    );
    QueryUtils::check_equal(
        &NumericDocValuesField::new_slow_range_query("foo", i64::MIN, i64::MAX)
            .rewrite(&searcher)?,
        &FieldExistsQuery::new("foo").into(),
    );

    Ok(())
}
#[test]
fn test_sorted_numeric_npe() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let iw = RandomIndexWriter::new(&mut random, dir.clone());

    let nums = [
        -1.7147449030215377E-208_f64,
        -1.6887024655302576E-11_f64,
        1.534911516604164E113_f64,
        0.0_f64,
        2.6947996404505155E-166_f64,
        -2.649722021970773E306_f64,
        6.138239235731689E-198_f64,
        2.3967090122610808E111_f64,
    ];

    for &v in nums.iter() {
        let mut doc = Document::default();
        let sortable = NumericUtils::double_to_sortable_long(v);
        doc.add(SortedNumericDocValuesField::new("dv", sortable));
        iw.add_document(doc)?;
    }

    iw.commit()?;

    let reader = iw.get_reader()?;
    let searcher = new_searcher_with_reader(reader)?;
    iw.close()?;

    let lo = NumericUtils::double_to_sortable_long(8.701032080293731E-226_f64);
    let hi = NumericUtils::double_to_sortable_long(2.0801416404385346E-41_f64);

    let max_doc = searcher.get_index_reader().max_doc()?;
    let q1 = SortedNumericDocValuesField::new_slow_range_query("dv", lo, hi);
    searcher.search_with_sort(q1, max_doc.try_convert()?, Sort::get_index_order()?)?;

    let q2 = SortedNumericDocValuesField::new_slow_range_query("dv", hi, lo);
    searcher.search_with_sort(q2, max_doc.try_convert()?, Sort::get_index_order()?)?;

    Ok(())
}
#[test]
fn test_set_equals() -> Result<()> {
    assert_eq!(
        NumericDocValuesField::new_slow_set_query("field", vec![17, 42])?,
        NumericDocValuesField::new_slow_set_query("field", vec![17, 42])?
    );

    assert_eq!(
        NumericDocValuesField::new_slow_set_query("field", vec![17, 42, 32416190071])?,
        NumericDocValuesField::new_slow_set_query("field", vec![17, 32416190071, 42])?
    );

    assert_ne!(
        NumericDocValuesField::new_slow_set_query("field", vec![42])?,
        NumericDocValuesField::new_slow_set_query("field2", vec![42])?
    );

    assert_ne!(
        NumericDocValuesField::new_slow_set_query("field", vec![17, 42])?,
        NumericDocValuesField::new_slow_set_query("field", vec![17, 32416190071])?
    );

    Ok(())
}

#[test]
fn test_duel_set_vs_terms_query() -> Result<()> {
    let mut random = random();
    let iters = at_least(&mut random, 2);

    for _ in 0..iters {
        let mut all_numbers = Vec::new();
        let end = 1 << TestUtil::next_int(&mut random, 1, 10);
        let num_numbers = TestUtil::next_int(&mut random, 1, end);
        for _ in 0..num_numbers {
            all_numbers.push(random.random::<i64>());
        }

        let dir = new_directory_shared(&mut random)?;
        let iw = RandomIndexWriter::new(&mut random, dir.clone());

        let num_docs = at_least(&mut random, 100);
        for _ in 0..num_docs {
            let mut doc = Document::new();
            let number = all_numbers[random.random_range(0..all_numbers.len())];

            doc.add(StringField::from_string(
                "text",
                number.to_string(),
                Store::No,
            )?);
            doc.add(NumericDocValuesField::new("long", number));
            doc.add(SortedNumericDocValuesField::new("twolongs", number));
            doc.add(SortedNumericDocValuesField::new("twolongs", number * 2));

            iw.add_document(doc)?;
        }
        // TODO delete by query 未实现
        // if num_numbers > 1 && random.random_bool(0.5) {
        //     iw.delete_documents_with_terms(
        //         Term::from_text("text", all_numbers[0].to_string()),
        //     )?;
        // }

        iw.commit()?;
        let reader = iw.get_reader()?;
        let searcher = new_searcher_with_reader(reader)?;
        iw.close()?;

        if searcher.get_top_reader_context().reader().num_docs()? == 0 {
            continue;
        }

        for _ in 0..100 {
            let boost = random.random::<f32>() * 10.0;
            let end = 1 << TestUtil::next_int(&mut random, 1, 8);
            let num_query_numbers = TestUtil::next_int(&mut random, 1, end);

            let mut query_numbers = std::collections::HashSet::new();
            let mut query_numbers_x2 = std::collections::HashSet::new();

            for _ in 0..num_query_numbers {
                let number = all_numbers[random.random_range(0..all_numbers.len())];
                query_numbers.insert(number);
                query_numbers_x2.insert(number * 2);
            }

            let query_numbers_array: Vec<i64> = query_numbers.iter().copied().collect();
            let query_numbers_x2_array: Vec<i64> = query_numbers_x2.iter().copied().collect();

            let mut bq = Builder::new();
            for number in &query_numbers {
                bq.add(
                    TermQuery::new(Term::from_text("text", number.to_string())),
                    Occur::Should,
                )?;
            }

            let q1 = BoostQuery::new(ConstantScoreQuery::new(bq.build()), boost)?;

            let q2 = BoostQuery::new(
                NumericDocValuesField::new_slow_set_query("long", query_numbers_array)?,
                boost,
            )?;
            assert_same_matches(&searcher, q1.clone(), q2, true)?;

            let q3 = BoostQuery::new(
                SortedNumericDocValuesField::new_slow_set_query(
                    "twolongs",
                    query_numbers.iter().copied().collect(),
                )?,
                boost,
            )?;
            assert_same_matches(&searcher, q1.clone(), q3, true)?;

            let q4 = BoostQuery::new(
                SortedNumericDocValuesField::new_slow_set_query(
                    "twolongs",
                    query_numbers_x2_array,
                )?,
                boost,
            )?;
            assert_same_matches(&searcher, q1.clone(), q4, true)?;
        }
    }

    Ok(())
}

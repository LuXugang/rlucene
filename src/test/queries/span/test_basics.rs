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
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::query::Query;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::analysis::mock_tokenizer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::check_hits::CheckHits;
use crate::test_framework::core::util::DefaultIndexSearchCR;
use crate::test_framework::core::util::english::English;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_log_merge_policy,
  new_searcher_with_reader, new_text_field, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Tests basic search capabilities.
///
/// Uses a collection of 2000 documents, each the english rendition of their document number.
/// For example, the document numbered 333 has text "three hundred thirty three".
///
/// Tests are each a single query, and its hits are checked to ensure that all and only the
/// correct documents are returned, thus providing end-to-end testing of the indexing and search
/// code.
#[allow(dead_code)]
pub struct TestBasics;

static CONTEXT: LazyLock<DefaultIndexSearchCR> = LazyLock::new(|| {
  let mut random = random();
  TestBasics::set_up(&mut random).expect("failed to initialize TestBasics")
});

impl TestBasics {
  fn set_up(random: &mut impl Rng) -> Result<DefaultIndexSearchCR> {
    let dir = new_directory_shared(random)?;
    let a = MockAnalyzer::with_automaton(random, mock_tokenizer::SIMPLE.clone(), true);
    let mut iwc = new_index_writer_config_with_analyzer(random, a)?;
    iwc.set_max_buffered_docs(TestUtil::next_int(random, 100, 1000));
    iwc.set_merge_policy(new_log_merge_policy(random)?);
    let writer = RandomIndexWriter::with_config(random, dir, iwc);
    let mut field_to_type = HashMap::new();
    for i in 0..2000 {
      let mut doc = Document::new();
      let field = new_text_field(
        random,
        "field",
        English::int_to_english(i),
        Store::Yes,
        &mut field_to_type,
      )?;
      doc.add(field);
      writer.add_document(random, doc)?;
    }
    let reader = writer.get_reader(random)?;
    let searcher = new_searcher_with_reader(reader)?;
    writer.close(random)?;
    Ok(searcher)
  }

  fn check_hits(
    random: &mut impl Rng,
    query: Query,
    searcher: &DefaultIndexSearchCR,
    results: &[i32],
  ) -> Result<()> {
    CheckHits::check_hits(random, query, "field", searcher, results)
  }
}

#[test]
fn test_term() -> Result<()> {
  let mut random = random();
  let searcher = &*CONTEXT;
  let query = TermQuery::new(Term::from_text("field", "seventy"));
  TestBasics::check_hits(
    &mut random,
    query.into(),
    searcher,
    &[
      70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179,
      270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 370, 371, 372, 373, 374, 375, 376, 377,
      378, 379, 470, 471, 472, 473, 474, 475, 476, 477, 478, 479, 570, 571, 572, 573, 574, 575,
      576, 577, 578, 579, 670, 671, 672, 673, 674, 675, 676, 677, 678, 679, 770, 771, 772, 773,
      774, 775, 776, 777, 778, 779, 870, 871, 872, 873, 874, 875, 876, 877, 878, 879, 970, 971,
      972, 973, 974, 975, 976, 977, 978, 979, 1070, 1071, 1072, 1073, 1074, 1075, 1076, 1077, 1078,
      1079, 1170, 1171, 1172, 1173, 1174, 1175, 1176, 1177, 1178, 1179, 1270, 1271, 1272, 1273,
      1274, 1275, 1276, 1277, 1278, 1279, 1370, 1371, 1372, 1373, 1374, 1375, 1376, 1377, 1378,
      1379, 1470, 1471, 1472, 1473, 1474, 1475, 1476, 1477, 1478, 1479, 1570, 1571, 1572, 1573,
      1574, 1575, 1576, 1577, 1578, 1579, 1670, 1671, 1672, 1673, 1674, 1675, 1676, 1677, 1678,
      1679, 1770, 1771, 1772, 1773, 1774, 1775, 1776, 1777, 1778, 1779, 1870, 1871, 1872, 1873,
      1874, 1875, 1876, 1877, 1878, 1879, 1970, 1971, 1972, 1973, 1974, 1975, 1976, 1977, 1978,
      1979,
    ],
  )
}

#[test]
fn test_term2() -> Result<()> {
  let mut random = random();
  let searcher = &*CONTEXT;
  let query = TermQuery::new(Term::from_text("field", "seventish"));
  TestBasics::check_hits(&mut random, query.into(), searcher, &[])
}

#[test]
fn test_phrase() -> Result<()> {
  let mut random = random();
  let searcher = &*CONTEXT;
  let query = PhraseQuery::from_terms_no_slop("field", &["seventy", "seven"])?;
  TestBasics::check_hits(
    &mut random,
    query.into(),
    searcher,
    &[
      77, 177, 277, 377, 477, 577, 677, 777, 877, 977, 1077, 1177, 1277, 1377, 1477, 1577, 1677,
      1777, 1877, 1977,
    ],
  )
}

#[test]
fn test_phrase2() -> Result<()> {
  let mut random = random();
  let searcher = &*CONTEXT;
  let query = PhraseQuery::from_terms_no_slop("field", &["seventish", "seven"])?;
  TestBasics::check_hits(&mut random, query.into(), searcher, &[])
}

#[test]
fn test_boolean() -> Result<()> {
  let mut random = random();
  let searcher = &*CONTEXT;
  let mut bq = BooleanQueryBuilder::new();
  bq.add(
    TermQuery::new(Term::from_text("field", "seventy")),
    Occur::Must,
  )?;
  bq.add(
    TermQuery::new(Term::from_text("field", "seven")),
    Occur::Must,
  )?;
  let query = bq.build();
  TestBasics::check_hits(
    &mut random,
    query.into(),
    searcher,
    &[
      77, 177, 277, 377, 477, 577, 677, 770, 771, 772, 773, 774, 775, 776, 777, 778, 779, 877, 977,
      1077, 1177, 1277, 1377, 1477, 1577, 1677, 1770, 1771, 1772, 1773, 1774, 1775, 1776, 1777,
      1778, 1779, 1877, 1977,
    ],
  )
}

#[test]
fn test_boolean2() -> Result<()> {
  let mut random = random();
  let searcher = &*CONTEXT;
  let mut bq = BooleanQueryBuilder::new();
  bq.add(
    TermQuery::new(Term::from_text("field", "sevento")),
    Occur::Must,
  )?;
  bq.add(
    TermQuery::new(Term::from_text("field", "sevenly")),
    Occur::Must,
  )?;
  let query = bq.build();
  TestBasics::check_hits(&mut random, query.into(), searcher, &[])
}

// TODO: SpanNearQuery 未实现
#[test]
fn test_span_near_exact() -> Result<()> {
  Ok(())
}

// TODO: SpanTermQuery 未实现
#[test]
fn test_span_term_query() -> Result<()> {
  Ok(())
}

// TODO: SpanNearQuery (unordered) 未实现
#[test]
fn test_span_near_unordered() -> Result<()> {
  Ok(())
}

// TODO: SpanNearQuery (ordered) 未实现
#[test]
fn test_span_near_ordered() -> Result<()> {
  Ok(())
}

// TODO: SpanNotQuery 未实现
#[test]
fn test_span_not() -> Result<()> {
  Ok(())
}

// TODO: SpanNotQuery 未实现
#[test]
fn test_span_not_no_overflow_on_large_spans() -> Result<()> {
  Ok(())
}

// TODO: SpanNotQuery 未实现
#[test]
fn test_span_with_multiple_not_single() -> Result<()> {
  Ok(())
}

// TODO: SpanNotQuery 未实现
#[test]
fn test_span_with_multiple_not_many() -> Result<()> {
  Ok(())
}

// TODO: SpanNearQuery + SpanNotQuery 未实现
#[test]
fn test_npe_in_span_near_with_span_not() -> Result<()> {
  Ok(())
}

// TODO: SpanNearQuery + SpanFirstQuery + SpanNotQuery 未实现
#[test]
fn test_npe_in_span_near_in_span_first_in_span_not() -> Result<()> {
  Ok(())
}

// TODO: SpanNotQuery 未实现
#[test]
fn test_span_not_window_one() -> Result<()> {
  Ok(())
}

// TODO: SpanNotQuery 未实现
#[test]
fn test_span_not_window_two_before() -> Result<()> {
  Ok(())
}

// TODO: SpanNotQuery 未实现
#[test]
fn test_span_not_window_neg_post() -> Result<()> {
  Ok(())
}

// TODO: SpanNotQuery 未实现
#[test]
fn test_span_not_window_neg_pre() -> Result<()> {
  Ok(())
}

// TODO: SpanNotQuery 未实现
#[test]
fn test_span_not_window_double_excludes_before() -> Result<()> {
  Ok(())
}

// TODO: SpanFirstQuery 未实现
#[test]
fn test_span_first() -> Result<()> {
  Ok(())
}

// TODO: SpanPositionRangeQuery 未实现
#[test]
fn test_span_position_range() -> Result<()> {
  Ok(())
}

// TODO: SpanOrQuery 未实现
#[test]
fn test_span_or() -> Result<()> {
  Ok(())
}

// TODO: SpanNearQuery 未实现
#[test]
fn test_span_exact_nested() -> Result<()> {
  Ok(())
}

// TODO: SpanNearQuery + SpanOrQuery 未实现
#[test]
fn test_span_near_or() -> Result<()> {
  Ok(())
}

// TODO: SpanNearQuery + SpanOrQuery 未实现
#[test]
fn test_span_complex1() -> Result<()> {
  Ok(())
}

// TODO: SpanTermQuery 未实现（boolean_query 已实现，但内部使用了 SpanTermQuery）
#[test]
fn test_boolean_span_query() -> Result<()> {
  Ok(())
}

// TODO: SpanTermQuery 未实现（DisjunctionMaxQuery 已实现，但内部使用了 SpanTermQuery）
#[test]
fn test_dismax_span_query() -> Result<()> {
  Ok(())
}

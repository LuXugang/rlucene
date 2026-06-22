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
use crate::core::document::field::Store::Yes;
use crate::core::document::field_type::FieldType;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::multi_term_query::{
  CONSTANT_SCORE_BOOLEAN_REWRITE, ConstantScoreBlendedRewrite, ConstantScoreRewrite,
  RewriteMethodEnum,
};
use crate::core::search::prefix_query::PrefixQuery;
use crate::core::search::query::{IntoQuery, Query};
use crate::core::search::term_query::TermQuery;
use crate::core::search::term_range_query::TermRangeQuery;
use crate::core::search::wildcard_query::WildcardQuery;
use crate::core::store::directory::DirEnum;
use crate::core::util::automation::operations::Operations;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::query_utils::QueryUtils;
use crate::test::core::search::test_base_range_filter;
use crate::test::core::search::test_base_range_filter::pad;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_field, new_index_writer_config_with_analyzer, new_log_merge_policy,
  new_searcher, new_text_field, random,
};
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub struct TestMultiTermConstantScore;

const SCORE_COMP_THRESH: f32 = 1e-6f32;
const T: bool = true;
const F: bool = false;

fn constant_score_rewrites() -> [RewriteMethodEnum; 2] {
  [
    ConstantScoreRewrite.into(),
    ConstantScoreBlendedRewrite.into(),
  ]
}

fn set_up() -> Result<(Arc<DirEnum>, StandardDirectoryReaderType<DirEnum>)> {
  let data = [
    Some("A 1 2 3 4 5 6"),
    Some("Z       4 5 6"),
    None,
    Some("B   2   4 5 6"),
    Some("Y     3   5 6"),
    None,
    Some("C     3     6"),
    Some("X       4 5 6"),
  ];

  let mut random = random();
  let small = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer);
  config.set_merge_policy(new_log_merge_policy(&mut random)?);
  let writer = RandomIndexWriter::with_config(&mut random, small.clone(), config);
  let mut field_types = HashMap::new();
  let mut custom_type = FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
  custom_type.set_tokenized(false)?;

  for (i, value) in data.iter().enumerate() {
    let mut doc = Document::new();
    doc.add(new_field(
      &mut random,
      "id",
      i.to_string(),
      &custom_type,
      &mut field_types,
    )?);
    doc.add(new_field(
      &mut random,
      "all",
      "all",
      &custom_type,
      &mut field_types,
    )?);
    if let Some(value) = value {
      doc.add(new_text_field(
        &mut random,
        "data",
        *value,
        Yes,
        &mut field_types,
      )?);
    }
    writer.add_document(doc)?;
  }

  let reader = writer.get_reader()?;
  writer.close()?;
  Ok((small, reader))
}
fn csrq(
  f: &str,
  l: Option<&str>,
  h: Option<&str>,
  il: bool,
  ih: bool,
  method: impl Into<RewriteMethodEnum>,
) -> Result<Query> {
  Ok(TermRangeQuery::new_string_range_with_rewrite(f, l, h, il, ih, method)?.into_query())
}

fn cspq(prefix: Term, method: impl Into<RewriteMethodEnum>) -> Result<Query> {
  Ok(PrefixQuery::with_rewrite(prefix, method)?.into_query())
}

fn cswcq(wild: Term, method: impl Into<RewriteMethodEnum>) -> Result<Query> {
  Ok(
    WildcardQuery::with_rewrite(
      wild,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
      method,
    )?
    .into_query(),
  )
}

#[test]
fn test_basics() -> Result<()> {
  for rw in constant_score_rewrites() {
    QueryUtils::check_from_query(&csrq("data", Some("1"), Some("6"), T, T, rw.clone())?);
    QueryUtils::check_from_query(&csrq("data", Some("A"), Some("Z"), T, T, rw.clone())?);
    QueryUtils::check_unequal(
      &csrq("data", Some("1"), Some("6"), T, T, rw.clone())?,
      &csrq("data", Some("A"), Some("Z"), T, T, rw.clone())?,
    );

    QueryUtils::check_from_query(&cspq(Term::from_text("data", "p*u?"), rw.clone())?);
    QueryUtils::check_unequal(
      &cspq(Term::from_text("data", "pre*"), rw.clone())?,
      &cspq(Term::from_text("data", "pres*"), rw.clone())?,
    );

    QueryUtils::check_from_query(&cswcq(Term::from_text("data", "p"), rw.clone())?);
    QueryUtils::check_unequal(
      &cswcq(Term::from_text("data", "pre*n?t"), rw.clone())?,
      &cswcq(Term::from_text("data", "pr*t?j"), rw.clone())?,
    );
  }
  Ok(())
}

#[test]
fn test_equal_scores() -> Result<()> {
  let (_small, reader) = set_up()?;
  let search = new_searcher(reader, false, false)?;

  let mut result = search
    .search(
      csrq(
        "data",
        Some("1"),
        Some("6"),
        T,
        T,
        ConstantScoreBlendedRewrite,
      )?,
      1000,
    )?
    .score_docs;
  let mut num_hits = result.len();
  assert_eq!(6, num_hits, "wrong number of results");
  let score = result[0].score;
  for (i, hit) in result.iter().enumerate().skip(1) {
    assert!(
      (score - hit.score).abs() <= SCORE_COMP_THRESH,
      "score for {i} was not the same"
    );
  }

  result = search
    .search(
      csrq(
        "data",
        Some("1"),
        Some("6"),
        T,
        T,
        CONSTANT_SCORE_BOOLEAN_REWRITE,
      )?,
      1000,
    )?
    .score_docs;
  num_hits = result.len();
  assert_eq!(6, num_hits, "wrong number of results");
  for (i, hit) in result.iter().enumerate() {
    assert!(
      (score - hit.score).abs() <= SCORE_COMP_THRESH,
      "score for {i} was not the same"
    );
  }

  result = search
    .search(
      csrq("data", Some("1"), Some("6"), T, T, ConstantScoreRewrite)?,
      1000,
    )?
    .score_docs;
  num_hits = result.len();
  assert_eq!(6, num_hits, "wrong number of results");
  for (i, hit) in result.iter().enumerate() {
    assert!(
      (score - hit.score).abs() <= SCORE_COMP_THRESH,
      "score for {i} was not the same"
    );
  }

  Ok(())
}

#[test]
fn test_equal_scores_when_no_hits() -> Result<()> {
  let (_small, reader) = set_up()?;
  let search = new_searcher(reader, false, false)?;
  let dummy_term = TermQuery::new(Term::from_text("data", "1"));

  let mut bq = BooleanQueryBuilder::new();
  bq.add(dummy_term.clone(), Occur::Should)?;
  bq.add(
    csrq(
      "data",
      Some("#"),
      Some("#"),
      T,
      T,
      ConstantScoreBlendedRewrite,
    )?,
    Occur::Should,
  )?;
  let mut result = search.search(bq.build(), 1000)?.score_docs;
  let mut num_hits = result.len();
  assert_eq!(1, num_hits, "wrong number of results");
  let score = result[0].score;
  for (i, hit) in result.iter().enumerate().skip(1) {
    assert!(
      (score - hit.score).abs() <= SCORE_COMP_THRESH,
      "score for {i} was not the same"
    );
  }

  bq = BooleanQueryBuilder::new();
  bq.add(dummy_term.clone(), Occur::Should)?;
  bq.add(
    csrq(
      "data",
      Some("#"),
      Some("#"),
      T,
      T,
      CONSTANT_SCORE_BOOLEAN_REWRITE,
    )?,
    Occur::Should,
  )?;
  result = search.search(bq.build(), 1000)?.score_docs;
  num_hits = result.len();
  assert_eq!(1, num_hits, "wrong number of results");
  for (i, hit) in result.iter().enumerate() {
    assert!(
      (score - hit.score).abs() <= SCORE_COMP_THRESH,
      "score for {i} was not the same"
    );
  }

  bq = BooleanQueryBuilder::new();
  bq.add(dummy_term, Occur::Should)?;
  bq.add(
    csrq("data", Some("#"), Some("#"), T, T, ConstantScoreRewrite)?,
    Occur::Should,
  )?;
  result = search.search(bq.build(), 1000)?.score_docs;
  num_hits = result.len();
  assert_eq!(1, num_hits, "wrong number of results");
  for (i, hit) in result.iter().enumerate() {
    assert!(
      (score - hit.score).abs() <= SCORE_COMP_THRESH,
      "score for {i} was not the same"
    );
  }

  Ok(())
}

#[test]
fn test_boolean_order_un_affected() -> Result<()> {
  let (_small, reader) = set_up()?;
  let search = new_searcher(reader, false, false)?;

  for rw in constant_score_rewrites() {
    let rq = csrq("data", Some("1"), Some("4"), T, T, rw.clone())?;
    let expected = search.search(rq.clone(), 1000)?.score_docs;
    let num_hits = expected.len();

    let mut q = BooleanQueryBuilder::new();
    q.add(rq, Occur::Must)?;
    q.add(
      csrq("data", Some("1"), Some("6"), T, T, rw.clone())?,
      Occur::Must,
    )?;

    let actual = search.search(q.build(), 1000)?.score_docs;
    assert_eq!(num_hits, actual.len(), "wrong number of hits");
    for i in 0..num_hits {
      assert_eq!(
        expected[i].doc, actual[i].doc,
        "mismatch in docid for hit#{i}"
      );
    }
  }

  Ok(())
}
#[test]
fn test_range_query_id() -> Result<()> {
  let mut random = random();
  let (min_id, max_id, _min_r, _max_r, reader, _unsigned_index_reader) =
    test_base_range_filter::set_up(&mut random)?;
  let search = new_searcher(reader, false, false)?;

  let med_id = (max_id - min_id) / 2;

  let min_ip = pad(min_id);
  let max_ip = pad(max_id);
  let med_ip = pad(med_id);

  let num_docs = search.get_index_reader().num_docs()?;

  assert_eq!(1 + max_id - min_id, num_docs, "num of docs");

  for rw in constant_score_rewrites() {
    // test id, bounded on both ends
    let mut result = search
      .search(
        csrq("id", Some(&min_ip), Some(&max_ip), T, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(num_docs as usize, result.len(), "find all");

    result = search
      .search(
        csrq("id", Some(&min_ip), Some(&max_ip), T, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!((num_docs - 1) as usize, result.len(), "all but last");

    result = search
      .search(
        csrq("id", Some(&min_ip), Some(&max_ip), F, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!((num_docs - 1) as usize, result.len(), "all but first");

    result = search
      .search(
        csrq("id", Some(&min_ip), Some(&max_ip), F, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!((num_docs - 2) as usize, result.len(), "all but ends");

    result = search
      .search(
        csrq("id", Some(&med_ip), Some(&max_ip), T, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!((1 + max_id - med_id) as usize, result.len(), "med and up");

    result = search
      .search(
        csrq("id", Some(&min_ip), Some(&med_ip), T, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!((1 + med_id - min_id) as usize, result.len(), "up to med");

    // unbounded id
    result = search
      .search(
        csrq("id", Some(&min_ip), None, T, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(num_docs as usize, result.len(), "min and up");

    result = search
      .search(
        csrq("id", None, Some(&max_ip), F, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(num_docs as usize, result.len(), "max and down");

    result = search
      .search(
        csrq("id", Some(&min_ip), None, F, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!((num_docs - 1) as usize, result.len(), "not min, but up");

    result = search
      .search(
        csrq("id", None, Some(&max_ip), F, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!((num_docs - 1) as usize, result.len(), "not max, but down");

    result = search
      .search(
        csrq("id", Some(&med_ip), Some(&max_ip), T, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(
      (max_id - med_id) as usize,
      result.len(),
      "med and up, not max"
    );

    result = search
      .search(
        csrq("id", Some(&min_ip), Some(&med_ip), F, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(
      (med_id - min_id) as usize,
      result.len(),
      "not min, up to med"
    );

    // very small sets
    result = search
      .search(
        csrq("id", Some(&min_ip), Some(&min_ip), F, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(0, result.len(), "min,min,F,F");

    result = search
      .search(
        csrq("id", Some(&med_ip), Some(&med_ip), F, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(0, result.len(), "med,med,F,F");

    result = search
      .search(
        csrq("id", Some(&max_ip), Some(&max_ip), F, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(0, result.len(), "max,max,F,F");
    result = search
      .search(
        csrq("id", Some(&min_ip), Some(&min_ip), T, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(1, result.len(), "min,min,T,T");

    result = search
      .search(
        csrq("id", None, Some(&min_ip), F, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(1, result.len(), "nul,min,F,T");
    result = search
      .search(
        csrq("id", Some(&max_ip), Some(&max_ip), T, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(1, result.len(), "max,max,T,T");

    result = search
      .search(
        csrq("id", Some(&max_ip), None, T, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(1, result.len(), "max,nul,T,T");
    result = search
      .search(
        csrq("id", Some(&med_ip), Some(&med_ip), T, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(1, result.len(), "med,med,T,T");
  }

  Ok(())
}
#[test]
fn test_range_query_rand() -> Result<()> {
  let mut random = random();
  let (min_id, max_id, min_r, max_r, reader, _unsigned_index_reader) =
    test_base_range_filter::set_up(&mut random)?;
  let search = new_searcher(reader, false, false)?;

  let min_rp = pad(min_r);
  let max_rp = pad(max_r);
  let num_docs = search.get_index_reader().num_docs()?;

  assert_eq!(1 + max_id - min_id, num_docs, "num of docs");

  for rw in constant_score_rewrites() {
    // test extremes, bounded on both ends
    let mut result = search
      .search(
        csrq("rand", Some(&min_rp), Some(&max_rp), T, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(num_docs as usize, result.len(), "find all");

    result = search
      .search(
        csrq("rand", Some(&min_rp), Some(&max_rp), T, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!((num_docs - 1) as usize, result.len(), "all but biggest");

    result = search
      .search(
        csrq("rand", Some(&min_rp), Some(&max_rp), F, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!((num_docs - 1) as usize, result.len(), "all but smallest");

    result = search
      .search(
        csrq("rand", Some(&min_rp), Some(&max_rp), F, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!((num_docs - 2) as usize, result.len(), "all but extremes");

    // unbounded
    result = search
      .search(
        csrq("rand", Some(&min_rp), None, T, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(num_docs as usize, result.len(), "smallest and up");

    result = search
      .search(
        csrq("rand", None, Some(&max_rp), F, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(num_docs as usize, result.len(), "biggest and down");

    result = search
      .search(
        csrq("rand", Some(&min_rp), None, F, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(
      (num_docs - 1) as usize,
      result.len(),
      "not smallest, but up"
    );

    result = search
      .search(
        csrq("rand", None, Some(&max_rp), F, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(
      (num_docs - 1) as usize,
      result.len(),
      "not biggest, but down"
    );

    // very small sets
    result = search
      .search(
        csrq("rand", Some(&min_rp), Some(&min_rp), F, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(0, result.len(), "min,min,F,F");

    result = search
      .search(
        csrq("rand", Some(&max_rp), Some(&max_rp), F, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(0, result.len(), "max,max,F,F");
    result = search
      .search(
        csrq("rand", Some(&min_rp), Some(&min_rp), T, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(1, result.len(), "min,min,T,T");

    result = search
      .search(
        csrq("rand", None, Some(&min_rp), F, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(1, result.len(), "nul,min,F,T");
    result = search
      .search(
        csrq("rand", Some(&max_rp), Some(&max_rp), T, T, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(1, result.len(), "max,max,T,T");

    result = search
      .search(
        csrq("rand", Some(&max_rp), None, T, F, rw.clone())?,
        num_docs as usize,
      )?
      .score_docs;
    assert_eq!(1, result.len(), "max,nul,T,T");
  }

  Ok(())
}

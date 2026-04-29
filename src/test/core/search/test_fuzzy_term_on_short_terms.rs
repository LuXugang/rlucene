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
use crate::analysis::common::analysis_impl::core::whitespace_analyzer::WhitespaceAnalyzer;
use crate::core::analysis::analyzer::AnalyzerEnum;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::index::directory_reader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::fuzzy_query::FuzzyQuery;
use crate::core::search::query::{Query, QueryBase};
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_log_merge_policy,
  new_searcher_with_reader, new_text_field, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand_chacha::rand_core::Rng;
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub struct TestFuzzyTermOnShortTerms;

const FIELD: &str = "field";
#[test]
fn test() -> Result<()> {
  let mut random = random();

  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "abc",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "ab"), 1)?.into(),
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "ab",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "abc"), 1)?.into(),
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "abcde",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "abc"), 2)?.into(),
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "abc",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "abcde"), 2)?.into(),
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "ab",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "a"), 1)?.into(),
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "a",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "ab"), 1)?.into(),
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "abc",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "a"), 2)?.into(),
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "a",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "abc"), 2)?.into(),
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "abcd",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "ab"), 2)?.into(),
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "ab",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "abcd"), 2)?.into(),
    1,
  )?;

  Ok(())
}
fn get_analyzer<R>(_random: &mut R) -> WhitespaceAnalyzer
where
  R: Rng + ?Sized,
{
  // TODO IMPORTANT MockTokenizer未实现
  WhitespaceAnalyzer::new()
}

fn count_hits<A>(
  random: &mut impl Rng,
  analyzer: A,
  docs: &str,
  q: Query,
  expected: i32,
) -> Result<()>
where
  A: Into<AnalyzerEnum>,
{
  let d = get_directory(random, analyzer, docs)?;
  let r = directory_reader::open(d.clone())?;
  let s = new_searcher_with_reader(r)?;
  let total_hits = s.count(q.clone())?;
  assert_eq!(expected, total_hits, "{}", q.as_string("")?);
  Ok(())
}
fn get_directory<R, A>(random: &mut R, analyzer: A, vals: &str) -> Result<Arc<DirEnum>>
where
  R: Rng + ?Sized,
  A: Into<AnalyzerEnum>,
{
  let directory = new_directory_shared(random)?;
  let mock = analyzer;
  let mut iwc = new_index_writer_config_with_analyzer(random, mock);
  iwc.set_max_buffered_docs(TestUtil::next_int(random, 100, 1000));
  iwc.set_merge_policy(new_log_merge_policy(random)?);

  let writer = RandomIndexWriter::with_config(random, directory.clone(), iwc);
  let mut field_to_type = HashMap::new();
  let mut d = Document::new();
  d.add(new_text_field(
    random,
    FIELD,
    vals,
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(d)?;

  writer.close()?;
  Ok(directory)
}

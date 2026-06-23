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
use crate::core::analysis::analyzer::{
  Analyzer, AnalyzerEnum, AnalyzerStoredValue, TokenStreamComponents,
};
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::index::directory_reader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::fuzzy_query::FuzzyQuery;
use crate::core::search::query::{IntoQuery, QueryBase};
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_tokenizer::{MockTokenizer, SIMPLE};
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_log_merge_policy,
  new_searcher_with_reader, new_text_field, random, random_from_seed,
};
use crate::test::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
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
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "ab"), 1)?,
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "ab",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "abc"), 1)?,
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "abcde",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "abc"), 2)?,
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "abc",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "abcde"), 2)?,
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "ab",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "a"), 1)?,
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "a",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "ab"), 1)?,
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "abc",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "a"), 2)?,
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "a",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "abc"), 2)?,
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "abcd",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "ab"), 2)?,
    1,
  )?;
  let a = get_analyzer(&mut random);
  count_hits(
    &mut random,
    a,
    "ab",
    FuzzyQuery::with_max_edits(Term::from_text(FIELD, "abcd"), 2)?,
    1,
  )?;

  Ok(())
}

struct FuzzyTermOnShortTermsAnalyzer {
  stored_value: AnalyzerStoredValue,
  seed: u64,
}

impl Analyzer for FuzzyTermOnShortTermsAnalyzer {
  fn create_components(&self, _field_name: &str) -> Result<TokenStreamComponents> {
    let tokenizer = MockTokenizer::with_default_max_token_length(
      random_from_seed(self.seed),
      SIMPLE.clone(),
      true,
    );
    Ok(TokenStreamComponents::new(
      Box::new(tokenizer) as Box<dyn TokenStream + Send + Sync>,
      None,
    ))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

crate::impl_analyzer_close!(FuzzyTermOnShortTermsAnalyzer);

fn get_analyzer<R>(random: &mut R) -> Box<dyn Analyzer>
where
  R: Rng + ?Sized,
{
  Box::new(FuzzyTermOnShortTermsAnalyzer {
    stored_value: AnalyzerStoredValue::new(),
    seed: random.random(),
  })
}

fn count_hits<A>(
  random: &mut impl Rng,
  analyzer: A,
  docs: &str,
  q: impl IntoQuery,
  expected: i32,
) -> Result<()>
where
  A: Into<AnalyzerEnum>,
{
  let q = q.into_query();
  let d = get_directory(random, analyzer, docs)?;
  let r = directory_reader::open(d.clone())?;
  let s = new_searcher_with_reader(r)?;
  let total_hits = s.count(q.clone())?;
  assert_eq!(expected, total_hits, "{}", q.to_string("")?);
  Ok(())
}
fn get_directory<R, A>(random: &mut R, analyzer: A, vals: &str) -> Result<Arc<DirEnum>>
where
  R: Rng + ?Sized,
  A: Into<AnalyzerEnum>,
{
  let directory = new_directory_shared(random)?;
  let mock = analyzer;
  let mut iwc = new_index_writer_config_with_analyzer(random, mock)?;
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
  writer.add_document(random, d)?;

  writer.close(random)?;
  Ok(directory)
}

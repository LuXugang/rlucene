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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::term::Term;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::ConstantScoreBlendedRewrite;
use crate::core::search::regexp_query::RegexpQuery;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::automaton_provider::{AutomatonProvider, DefaultProvider};
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::DefaultIndexSearchCR;
use crate::test_framework::core::util::lucene_test_case::{
  is_light_mode, new_directory_shared, new_searcher_with_reader, new_text_field, random,
};
use rand::Rng;
use rand::RngExt;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

#[allow(dead_code)] // for quick search
struct TestRegexpQuery;
const FN: &str = "field";

static LIGHT_SEARCHER: LazyLock<Arc<DefaultIndexSearchCR>> = LazyLock::new(|| {
  let mut random = random();
  Arc::new(build_set_up(&mut random).expect("failed to initialize TestRegexpQuery"))
});

fn set_up<R>(random: &mut R) -> Result<Arc<DefaultIndexSearchCR>>
where
  R: Rng + ?Sized,
{
  if is_light_mode() {
    return Ok(LIGHT_SEARCHER.clone());
  }

  Ok(Arc::new(build_set_up(random)?))
}

fn build_set_up<R>(random: &mut R) -> Result<DefaultIndexSearchCR>
where
  R: Rng + ?Sized,
{
  let directory = new_directory_shared(random)?;
  let writer = RandomIndexWriter::new(random, directory.clone())?;
  let mut doc = Document::new();
  let mut field_to_type = HashMap::new();
  doc.add(new_text_field(
    random,
    FN,
    "the quick brown fox jumps over the lazy ??? dog 493432 49344 [foo] 12.3 \\",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(random, doc)?;

  let reader = writer.get_reader(random)?;
  writer.close(random)?;

  let searcher = new_searcher_with_reader(reader)?;

  Ok(searcher)
}
fn new_term(value: &str) -> Term {
  Term::from_text(FN, value)
}

fn regex_query_nr_hits<IRC>(searcher: &IndexSearcher<IRC>, regex: &str) -> Result<i64>
where
  IRC: IndexReaderContext + Sync,
{
  let query = RegexpQuery::new(new_term(regex))?;
  Ok(searcher.count(query)? as i64)
}
fn case_insensitive_regex_query_nr_hits<IRC, R>(
  random: &mut R,
  searcher: &IndexSearcher<IRC>,
  regex: &str,
) -> Result<i64>
where
  R: Rng + ?Sized,
  IRC: IndexReaderContext + Sync,
{
  let query = RegexpQuery::with_all_and_determinization(
    new_term(regex),
    RegExp::ALL,
    RegExp::ASCII_CASE_INSENSITIVE,
    &DefaultProvider,
    Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
    ConstantScoreBlendedRewrite,
    random.random_bool(0.5),
  )?;
  Ok(searcher.count(query)? as i64)
}
#[test]
fn test_regex1() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;
  assert_eq!(1, regex_query_nr_hits(&searcher, "q.[aeiou]c.*")?);
  Ok(())
}
#[test]
fn test_regex2() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;
  assert_eq!(0, regex_query_nr_hits(&searcher, ".[aeiou]c.*")?);
  Ok(())
}

#[test]
fn test_regex3() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;
  assert_eq!(0, regex_query_nr_hits(&searcher, "q.[aeiou]c")?);
  Ok(())
}

#[test]
fn test_numeric_range() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;
  assert_eq!(1, regex_query_nr_hits(&searcher, "<420000-600000>")?);
  assert_eq!(0, regex_query_nr_hits(&searcher, "<493433-600000>")?);
  Ok(())
}
#[test]
fn test_character_classes() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  assert_eq!(0, regex_query_nr_hits(&searcher, "\\d")?);
  assert_eq!(1, regex_query_nr_hits(&searcher, "\\d*")?);
  assert_eq!(1, regex_query_nr_hits(&searcher, "\\d{6}")?);
  assert_eq!(1, regex_query_nr_hits(&searcher, "[a\\d]{6}")?);
  assert_eq!(1, regex_query_nr_hits(&searcher, "\\d{2,7}")?);
  assert_eq!(0, regex_query_nr_hits(&searcher, "\\d{4}")?);
  assert_eq!(0, regex_query_nr_hits(&searcher, "\\dog")?);
  assert_eq!(1, regex_query_nr_hits(&searcher, "493\\d32")?);

  assert_eq!(1, regex_query_nr_hits(&searcher, "\\wox")?);
  assert_eq!(1, regex_query_nr_hits(&searcher, "493\\w32")?);
  assert_eq!(1, regex_query_nr_hits(&searcher, "\\?\\?\\?")?);
  assert_eq!(1, regex_query_nr_hits(&searcher, "\\?\\W\\?")?);
  assert_eq!(1, regex_query_nr_hits(&searcher, "\\?\\S\\?")?);

  assert_eq!(1, regex_query_nr_hits(&searcher, "\\[foo\\]")?);
  assert_eq!(1, regex_query_nr_hits(&searcher, "\\[\\w{3}\\]")?);

  assert_eq!(0, regex_query_nr_hits(&searcher, "\\s.*")?);
  assert_eq!(1, regex_query_nr_hits(&searcher, "\\S*ck")?);
  assert_eq!(1, regex_query_nr_hits(&searcher, "[\\d\\.]{3,10}")?);
  assert_eq!(
    1,
    regex_query_nr_hits(&searcher, "\\d{1,3}(\\.(\\d{1,2}))+")?
  );

  assert_eq!(1, regex_query_nr_hits(&searcher, "\\\\")?);
  assert_eq!(1, regex_query_nr_hits(&searcher, "\\\\.*")?);

  let err = regex_query_nr_hits(&searcher, "\\p").unwrap_err();
  match err {
    LuceneError::IllegalArgument(msg) => {
      assert!(msg.to_string().contains("invalid character class"));
    },
    _ => unreachable!(),
  }

  Ok(())
}
#[test]
fn test_case_insensitive() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  assert_eq!(0, regex_query_nr_hits(&searcher, "Quick")?);
  assert_eq!(
    1,
    case_insensitive_regex_query_nr_hits(&mut random, &searcher, "Quick")?
  );
  Ok(())
}

#[test]
fn test_regex_negated_character_class() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  assert_eq!(1, regex_query_nr_hits(&searcher, "[^a-z]")?);
  assert_eq!(1, regex_query_nr_hits(&searcher, "[^03ad]")?);
  Ok(())
}
#[test]
fn test_custom_provider() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let query = RegexpQuery::with_provider(
    new_term("<quickBrown>"),
    RegExp::ALL,
    &MyProvider,
    Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
  )?;

  let top_docs = searcher.search(query, 5)?;
  assert_eq!(1, top_docs.total_hits.value());

  Ok(())
}
struct MyProvider;
impl AutomatonProvider for MyProvider {
  fn get_automaton(&self, name: &str) -> Result<Option<Automaton>> {
    if name == "quickBrown" {
      Ok(Some(Operations::union_list(&[
        &Automata::make_string("quick")?,
        &Automata::make_string("brown")?,
        &Automata::make_string("bob")?,
      ])?))
    } else {
      Ok(None)
    }
  }
}
/// Test a corner case for backtracking: In this case the term dictionary has 493432 followed by 49344.
/// When backtracking from 49343... to 4934, it's necessary to test that 4934 itself is ok before trying to
/// append more characters.
#[test]
fn test_backtracking() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;
  assert_eq!(1, regex_query_nr_hits(&searcher, "4934[314]")?);
  Ok(())
}

#[test]
fn test_slow_common_suffix() -> Result<()> {
  let mut random = random();
  let _searcher = set_up(&mut random)?;

  let err = RegexpQuery::new(Term::from_text("stringvalue", "(.*a){2000}")).unwrap_err();
  match err {
    LuceneError::TooComplexToDeterminize(_) => {},
    _ => {
      return Err(LuceneError::illegal_state(
        "expected a too-complex-to-determinize error",
      ));
    },
  }
  Ok(())
}

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
use crate::test::support::core::util::lucene_test_case::{
  at_least, is_night_mode, new_directory_shared, new_searcher_with_reader, new_text_field, random,
};

use crate::core::index::BytesRef;
use crate::core::index::multi_terms::get_terms;
use crate::core::index::term::Term;
use crate::core::search::automaton_query::AutomatonQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{
  ConstantScoreBlendedRewrite, ConstantScoreRewrite, MultiTermQuery,
};
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::compiled_automaton::CompiledAutomatonTE;
use crate::core::util::automation::operations::Operations;
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
use crate::test::support::core::util::DefaultIndexSearchCR;
use crate::test::support::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;
use std::collections::HashMap;
use std::rc::Rc;

use crate::core::search::query::{IntoQuery, Query};
use crate::core::search::regexp_query::RegexpQuery;
use crate::core::search::scoring_rewrite::ScoringBooleanRewrite;
use crate::core::search::wildcard_query::WildcardQuery;
use crate::core::util::CoreHelper;
use crate::test::support::core::util::automaton::automaton_test_util::AutomatonTestUtil;

#[allow(dead_code)] // for quick search
struct TestAutomatonQuery;
const FN: &str = "field";

fn set_up<R>(random: &mut R) -> Result<DefaultIndexSearchCR>
where
  R: Rng + ?Sized,
{
  let directory = new_directory_shared(random)?;
  let mut field_to_type = HashMap::new();

  let writer = RandomIndexWriter::new(random, directory)?;

  let mut doc = Document::new();
  let title_field = new_text_field(random, "title", "some title", Store::No, &mut field_to_type)?;
  let mut field = new_text_field(
    random,
    FN,
    "this is document one 2345",
    Store::No,
    &mut field_to_type,
  )?;
  let footer_field = new_text_field(random, "footer", "a footer", Store::No, &mut field_to_type)?;

  doc.add(title_field.clone());
  doc.add(field.clone());
  doc.add(footer_field.clone());
  writer.add_document(random, doc)?;

  doc = Document::new();
  field.set_string_value("some text from doc two a short piece 5678.91")?;
  doc.add(title_field.clone());
  doc.add(field.clone());
  doc.add(footer_field.clone());
  writer.add_document(random, doc.clone())?;

  doc = Document::new();
  field
    .set_string_value("doc three has some different stuff with numbers 1234 5678.9 and letter b")?;
  doc.add(title_field.clone());
  doc.add(field.clone());
  doc.add(footer_field.clone());
  writer.add_document(random, doc.clone())?;

  let reader = writer.get_reader(random)?;
  let searcher = new_searcher_with_reader(reader)?;

  writer.close(random)?;
  Ok(searcher)
}
fn new_term(value: &str) -> Term {
  Term::from_text(FN, value)
}
fn automaton_query_nr_hits<IRC>(
  searcher: &IndexSearcher<IRC>,
  query: AutomatonQuery,
) -> Result<usize>
where
  IRC: IndexReaderContext + Sync,
{
  let top_docs = searcher.search(query, 5)?;
  Ok(top_docs.total_hits().value())
}
fn assert_automaton_hits<IRC>(
  expected: usize,
  automaton: Automaton,
  searcher: &IndexSearcher<IRC>,
) -> Result<()>
where
  IRC: IndexReaderContext + Sync,
{
  assert_eq!(
    expected,
    automaton_query_nr_hits(
      searcher,
      AutomatonQuery::new(
        new_term("bogus"),
        automaton.clone(),
        false,
        ScoringBooleanRewrite,
      )?,
    )?
  );

  assert_eq!(
    expected,
    automaton_query_nr_hits(
      searcher,
      AutomatonQuery::new(
        new_term("bogus"),
        automaton.clone(),
        false,
        ConstantScoreRewrite,
      )?,
    )?
  );

  assert_eq!(
    expected,
    automaton_query_nr_hits(
      searcher,
      AutomatonQuery::new(
        new_term("bogus"),
        automaton.clone(),
        false,
        ConstantScoreBlendedRewrite,
      )?,
    )?
  );

  assert_eq!(
    expected,
    automaton_query_nr_hits(
      searcher,
      AutomatonQuery::new(
        new_term("bogus"),
        automaton,
        false,
        ConstantScoreBlendedRewrite,
      )?,
    )?
  );

  Ok(())
}
#[test]
fn test_automata() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  assert_automaton_hits(0, Automata::make_empty()?, &searcher)?;
  assert_automaton_hits(0, Automata::make_empty_string()?, &searcher)?;
  assert_automaton_hits(2, Automata::make_any_char()?, &searcher)?;
  assert_automaton_hits(3, Automata::make_any_string()?, &searcher)?;
  assert_automaton_hits(2, Automata::make_string("doc")?, &searcher)?;
  assert_automaton_hits(1, Automata::make_char('a' as i32)?, &searcher)?;
  assert_automaton_hits(
    2,
    Automata::make_char_range('a' as i32, 'b' as i32)?,
    &searcher,
  )?;
  assert_automaton_hits(
    2,
    Automata::make_decimal_interval(1233, 2346, 0)?,
    &searcher,
  )?;

  assert_automaton_hits(
    1,
    Operations::determinize(
      &Automata::make_decimal_interval(0, 2000, 0)?,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT,
    )?
    .into_owned(),
    &searcher,
  )?;

  assert_automaton_hits(
    2,
    Operations::union(
      &Automata::make_char('a' as i32)?,
      &Automata::make_char('b' as i32)?,
    )?,
    &searcher,
  )?;

  assert_automaton_hits(
    0,
    Operations::intersection(
      &Automata::make_char('a' as i32)?,
      &Automata::make_char('b' as i32)?,
    )?
    .into_owned(),
    &searcher,
  )?;

  assert_automaton_hits(
    1,
    Operations::minus(
      &Automata::make_char_range('a' as i32, 'b' as i32)?,
      &Automata::make_char('a' as i32)?,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT,
    )?
    .into_owned(),
    &searcher,
  )?;

  Ok(())
}
#[test]
fn test_equals() -> Result<()> {
  let a1 = AutomatonQuery::from_automaton(new_term("foobar"), Automata::make_string("foobar")?)?;
  let a2 = a1.clone();
  let a3 = AutomatonQuery::from_automaton(
    new_term("foobar"),
    Operations::concatenate(
      &Automata::make_string("foo")?,
      &Automata::make_string("bar")?,
    )?,
  )?;
  let a4 = AutomatonQuery::from_automaton(new_term("foobar"), Automata::make_string("different")?)?;
  let a5 = AutomatonQuery::from_automaton(new_term("blah"), Automata::make_string("foobar")?)?;

  let a1: Query = a1.into_query();
  let a2: Query = a2.into_query();
  let a3: Query = a3.into_query();
  let a4: Query = a4.into_query();
  let a5: Query = a5.into_query();

  assert_eq!(
    CoreHelper::calculate_hash(&a1),
    CoreHelper::calculate_hash(&a2)
  );
  assert_eq!(a1, a2);
  assert_eq!(
    CoreHelper::calculate_hash(&a1),
    CoreHelper::calculate_hash(&a3)
  );
  assert_eq!(a1, a3);

  let w1: Query = WildcardQuery::new(new_term("foobar"))?.into_query();
  let w2: Query = RegexpQuery::new(new_term("foobar"))?.into_query();

  assert_ne!(a1, w1);
  assert_ne!(a1, w2);
  assert_ne!(w1, w2);
  assert_ne!(a1, a4);
  assert_ne!(a1, a5);

  Ok(())
}
#[test]
fn test_rewrite_single_term() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let aq = AutomatonQuery::from_automaton(new_term("bogus"), Automata::make_string("piece")?)?;

  let terms = Rc::new(get_terms(searcher.get_index_reader(), FN)?.unwrap());

  let te = aq.get_terms_enum(terms)?;
  assert!(matches!(te, CompiledAutomatonTE::Single(_)));

  assert_eq!(1, automaton_query_nr_hits(&searcher, aq)?);
  Ok(())
}
/// Test that rewriting to a prefix query works as expected, preserves MultiTermQuery semantics.
#[test]
fn test_rewrite_prefix() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let pfx = Automata::make_string("do")?;
  let prefix_automaton = Operations::concatenate(&pfx, &Automata::make_any_string()?)?;

  let aq = AutomatonQuery::from_automaton(new_term("bogus"), prefix_automaton)?;
  assert_eq!(3, automaton_query_nr_hits(&searcher, aq)?);

  Ok(())
}

/// Test handling of the empty language
#[test]
fn test_empty_optimization() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let aq = AutomatonQuery::from_automaton(new_term("bogus"), Automata::make_empty()?)?;

  let terms = Rc::new(get_terms(searcher.get_index_reader(), FN)?.unwrap());
  let te = aq.get_terms_enum(terms)?;
  assert!(matches!(te, CompiledAutomatonTE::Empty(_)));

  assert_eq!(0, automaton_query_nr_hits(&searcher, aq)?);
  Ok(())
}
#[test]
fn test_hash_code_with_threads() -> Result<()> {
  let mut random = random();
  let mut queries = Vec::new();
  for _ in 0..at_least(&mut random, 100) {
    let automaton = AutomatonTestUtil::random_automaton(&mut random)?;
    let automaton =
      Operations::determinize(&automaton, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?.into_owned();
    queries.push(AutomatonQuery::from_automaton(
      Term::from_text("bogus", "bogus"),
      automaton,
    )?);
  }

  let queries = std::sync::Arc::new(queries);
  let num_threads = random.random_range(2..=5);
  let starting_gun = std::sync::Arc::new(std::sync::Barrier::new(num_threads));
  let mut threads = Vec::new();

  for _ in 0..num_threads {
    let queries = std::sync::Arc::clone(&queries);
    let starting_gun = std::sync::Arc::clone(&starting_gun);
    threads.push(std::thread::spawn(move || {
      starting_gun.wait();
      for query in queries.iter() {
        CoreHelper::calculate_hash(query);
      }
    }));
  }

  for thread in threads {
    thread.join().unwrap();
  }
  Ok(())
}
#[test]
fn test_biggish_automaton() -> Result<()> {
  let mut random = random();

  let num_terms: usize = if is_night_mode() { 3000 } else { 500 };

  let mut terms = Vec::new();
  while terms.len() < num_terms {
    let s = TestUtil::random_unicode_string(&mut random);
    terms.push(BytesRef::from_string(&s));
  }

  terms.sort();

  let automaton = Automata::make_string_union(terms.as_ref())?;
  let _aq = AutomatonQuery::from_automaton(Term::from_text("foo", "bar"), automaton)?;

  Ok(())
}

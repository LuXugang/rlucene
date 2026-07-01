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

use crate::test::support::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_reader, new_string_field, new_text_field, random,
};
use std::collections::HashMap;

use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::term::Term;
use crate::core::search::automaton_query::AutomatonQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{
  CONSTANT_SCORE_BOOLEAN_REWRITE, ConstantScoreBlendedRewrite, ConstantScoreRewrite,
  SCORING_BOOLEAN_REWRITE,
};
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
use crate::test::support::core::util::DefaultIndexSearchCR;
use rand::Rng;

pub struct TestAutomatonQueryUnicode;
const FN: &str = "field";

fn set_up<R>(random: &mut R) -> Result<DefaultIndexSearchCR>
where
  R: Rng + ?Sized,
{
  let directory = new_directory_shared(random)?;
  let mut field_to_type = HashMap::new();
  let writer = RandomIndexWriter::new(random, directory)?;

  let title_field = new_text_field(random, "title", "some title", Store::No, &mut field_to_type)?;
  let mut field = new_string_field(random, FN, "", Store::No, &mut field_to_type)?;
  let footer_field = new_text_field(random, "footer", "a footer", Store::No, &mut field_to_type)?;

  let values = [
    "\u{29B05}abcdef",
    "\u{29B06}ghijkl",
    "\u{FB94}mnopqr",
    "\u{FB95}stuvwx",
    "a\u{FFFC}bc",
    "a\u{FFFD}bc",
    "a\u{FFFE}bc",
    "a\u{FB94}bc",
    "bacadaba",
    "\u{FFFD}",
    "\u{FFFD}\u{29B05}",
    "\u{FFFD}\u{FFFD}",
  ];

  for value in values {
    let mut doc = Document::new();
    field.set_string_value(value)?;
    doc.add(title_field.clone());
    doc.add(field.clone());
    doc.add(footer_field.clone());
    writer.add_document(random, doc)?;
  }

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
  Ok(searcher.search(query, 5)?.total_hits().value())
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
        SCORING_BOOLEAN_REWRITE,
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
        ConstantScoreRewrite
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
        CONSTANT_SCORE_BOOLEAN_REWRITE
      )?,
    )?
  );
  Ok(())
}
/// Test that [`AutomatonQuery`] interacts with lucene's sort order correctly.
///
/// This expression matches something either starting with the arabic presentation forms block,
/// or a supplementary character.
#[test]
fn test_sort_order() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  // Matches terms that start with either the Arabic Presentation Forms block or
  // a supplementary character.
  let automaton = RegExp::from_string("((\u{29B05})|\u{FB94}).*")?.to_automaton()?;
  assert_automaton_hits(2, automaton, &searcher)
}

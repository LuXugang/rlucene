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
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::index::BytesRef;
use crate::core::index::filtered_terms_enum::{
  AcceptStatus, FilteredTermsEnum, FilteredTermsEnumBase,
};
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{
  CONSTANT_SCORE_BOOLEAN_REWRITE, ConstantScoreBlendedRewrite, MultiTermQuery, MultiTermQuerySet,
  RewriteMethod,
};
use crate::core::search::query::{IntoQuery, Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::regexp_query::RegexpQuery;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::automation::automaton_provider::DefaultProvider;
use crate::core::util::automation::character_run_automaton::CharacterRunAutomaton;
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::support::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::support::core::analysis::mock_tokenizer;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
use crate::test::support::core::search::check_hits::CheckHits;
pub use crate::test::support::core::search::multi_term::DumbRegexpQuery;
use crate::test::support::core::util::DefaultIndexSearchCRShared;
use crate::test::support::core::util::automaton::automaton_test_util::AutomatonTestUtil;
use crate::test::support::core::util::lucene_test_case::{
  at_least, new_bytes_ref_from_string, new_directory_shared, new_index_writer_config_with_analyzer,
  new_searcher_with_reader, new_string_field, random,
};
use crate::test::support::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;
use rand::prelude::StdRng;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Create an index with random unicode terms Generates random regexps, and validates against a
/// simple impl.
#[allow(dead_code)] // for quick search
pub(crate) trait TestRegexpRandom2 {
  /// Check that the hits are the same as from a very simple RegexpQuery implementation.
  fn assert_same<IRC>(
    &self,
    searcher1: &IndexSearcher<IRC>,
    searcher2: &IndexSearcher<IRC>,
    searcher3: &IndexSearcher<IRC>,
    field_name: &str,
    regexp: String,
  ) -> Result<()>
  where
    IRC: IndexReaderContext + Sync,
  {
    let smart = RegexpQuery::with_flags(Term::from_text(field_name, regexp.clone()), RegExp::NONE)?;
    let nfa_query = RegexpQuery::with_all_and_determinization(
      Term::from_text(field_name, regexp.clone()),
      RegExp::NONE,
      0,
      &DefaultProvider,
      0,
      CONSTANT_SCORE_BOOLEAN_REWRITE,
      false,
    )?;
    let dumb = DumbRegexpQuery::new(Term::from_text(field_name, regexp), RegExp::NONE)?;

    let smart_docs = searcher1.search(smart.clone(), 25)?;
    let dumb_docs = searcher2.search(dumb.clone(), 25)?;
    let nfa_docs = searcher3.search(nfa_query.clone(), 25)?;

    CheckHits::check_equal(
      &smart.into_query(),
      &smart_docs.score_docs,
      &dumb_docs.score_docs,
    )?;
    CheckHits::check_equal(
      &nfa_query.into_query(),
      &nfa_docs.score_docs,
      &dumb_docs.score_docs,
    )
  }

  /// Test a bunch of random regular expressions.
  fn test_regexps<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let (searcher1, searcher2, searcher3, field_name) = set_up(random)?;

    let num = at_least(random, 200);
    for _ in 0..num {
      let regexp = AutomatonTestUtil::random_regexp(random)?;
      self.assert_same(
        &searcher1,
        &searcher2,
        &searcher3,
        field_name.as_str(),
        regexp,
      )?;
    }

    Ok(())
  }
}

fn set_up<R>(
  random: &mut R,
) -> Result<(
  DefaultIndexSearchCRShared,
  DefaultIndexSearchCRShared,
  DefaultIndexSearchCRShared,
  String,
)>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let field_name = if random.random_bool(0.5) {
    "field".to_string()
  } else {
    "".to_string()
  };
  let a = MockAnalyzer::with_automaton(random, mock_tokenizer::KEYWORD.clone(), false);
  let mut config = new_index_writer_config_with_analyzer(random, a)?;
  config.set_max_buffered_docs(TestUtil::next_int(random, 50, 1000));

  let writer = RandomIndexWriter::with_config(random, dir, config);
  let mut field_to_type = HashMap::new();

  let num = at_least(random, 200);
  for _ in 0..num {
    let mut doc = Document::new();
    let value = TestUtil::random_unicode_string(random);
    doc.add(new_string_field(
      random,
      field_name.as_str(),
      &value,
      crate::core::document::field::Store::No,
      &mut field_to_type,
    )?);
    doc.add(SortedDocValuesField::new(
      field_name.as_str(),
      new_bytes_ref_from_string(random, &value)?,
    ));
    writer.add_document(random, doc)?;
  }

  let reader = Arc::new(writer.get_reader(random)?);
  writer.close(random)?;
  Ok((
    new_searcher_with_reader(reader.clone())?,
    new_searcher_with_reader(reader.clone())?,
    new_searcher_with_reader(reader)?,
    field_name,
  ))
}

struct TestRegexpRandom2Impl;
fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestRegexpRandom2Impl, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestRegexpRandom2Impl;
  f(&case, &mut random)
}
impl TestRegexpRandom2 for TestRegexpRandom2Impl {}

#[test]
fn test_regexps() -> Result<()> {
  run_case(|case, random| case.test_regexps(random))
}

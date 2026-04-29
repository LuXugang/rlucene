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
  ConstantScoreBlendedRewrite,  MultiTermQuery, RewriteMethod,
};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::regexp_query::RegexpQuery;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::automation::automaton_provider::DefaultProvider;
use crate::core::util::automation::character_run_automaton::CharacterRunAutomaton;
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::check_hits::CheckHits;
use crate::test::core::util::DefaultIndexSearchCR;
use crate::test::core::util::automaton::automaton_test_util::AutomatonTestUtil;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_directory_shared, new_index_writer_config, new_searcher_with_reader,
  new_string_field, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

/// Create an index with random unicode terms Generates random regexps, and validates against a
/// simple impl.
#[allow(dead_code)] // for quick search
pub struct TestRegexpRandom2;

fn set_up<R>(random: &mut R) -> Result<(DefaultIndexSearchCR, String)>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let field_name = if random.random_bool(0.5) {
    "field".to_string()
  } else {
    "".to_string()
  };
  // TODO IMPORTANT 要使用MockAnalyzer带分词器
  let mut config = new_index_writer_config(random);
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
    writer.add_document(doc)?;
  }

  let reader = writer.get_reader()?;
  writer.close()?;
  Ok((new_searcher_with_reader(reader)?, field_name))
}

/// Check that the hits are the same as from a very simple RegexpQuery implementation.
fn assert_same<IRC>(searcher: &IndexSearcher<IRC>, field_name: &str, regexp: String) -> Result<()>
where
  IRC: IndexReaderContext,
{
  let smart = RegexpQuery::with_flags(Term::from_text(field_name, regexp.clone()), RegExp::NONE)?;
  let nfa_query = RegexpQuery::with_all_and_determinization(
    Term::from_text(field_name, regexp.clone()),
    RegExp::NONE,
    0,
    &DefaultProvider,
    0,
    // TODO IMPORTANT CONSTANT_SCORE_BOOLEAN_REWRITE 未实现
    ConstantScoreBlendedRewrite,
    false,
  )?;
  let dumb = DumbRegexpQuery::new(Term::from_text(field_name, regexp), RegExp::NONE)?;

  let smart_docs = searcher.search(smart.clone(), 25)?;
  let dumb_docs = searcher.search(dumb.clone(), 25)?;
  let nfa_docs = searcher.search(nfa_query.clone(), 25)?;

  CheckHits::check_equal(&smart.into(), &smart_docs.score_docs, &dumb_docs.score_docs)?;
  CheckHits::check_equal(
    &nfa_query.into(),
    &nfa_docs.score_docs,
    &dumb_docs.score_docs,
  )
}

/// Test a bunch of random regular expressions.
#[test]
fn test_regexps() -> Result<()> {
  let mut random = random();
  let (searcher, field_name) = set_up(&mut random)?;

  let num = at_least(&mut random, 200);
  for _ in 0..num {
    let regexp = AutomatonTestUtil::random_regexp(&mut random)?;
    assert_same(&searcher, field_name.as_str(), regexp)?;
  }

  Ok(())
}

/// A simple regexp query that scans through all terms.
#[derive(Clone)]
pub struct DumbRegexpQuery {
  field: String,
  run_automaton: CharacterRunAutomaton,
  id: Identity,
}

impl DumbRegexpQuery {
  pub fn new(term: Term, flags: i32) -> Result<Self> {
    let re = RegExp::parse(&term.text()?, flags, 0)?;
    let automaton = match Operations::determinize(
      &re.to_automaton()?,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT,
    )? {
      Cow::Owned(o) => o,
      Cow::Borrowed(b) => b.clone(),
    };

    Ok(Self {
      field: term.field().to_string(),
      run_automaton: CharacterRunAutomaton::new(automaton)?,
      id: Identity::default(),
    })
  }
}

impl QueryBase for DumbRegexpQuery {
  fn as_string(&self, field: &str) -> Result<String> {
    if self.field == field {
      Ok(self.run_automaton.base.automaton.to_string())
    } else {
      Ok(format!(
        "{}:{}",
        self.field, self.run_automaton.base.automaton
      ))
    }
  }

  fn create_weight<IRC>(
    self,
    _searcher: &IndexSearcher<IRC>,
    _score_mode: &ScoreMode,
    _boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Err(LuceneError::unsupported_operation(""))
  }

  fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    ConstantScoreBlendedRewrite.rewrite(searcher, self)
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
  }
}

impl Debug for DumbRegexpQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.as_string("") {
      Ok(s) => write!(f, "{}", s),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl HasIdentity for DumbRegexpQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl MultiTermQuery for DumbRegexpQuery {
  fn get_field(&self) -> &str {
    &self.field
  }

  type TermsEnum<T>
    = FilteredTermsEnum<T::TermsEnum, SimpleAutomatonTermsEnum>
  where
    T: Terms;

  fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
  where
    T: Terms + Clone,
  {
    let mut terms_enum = FilteredTermsEnum::new(
      terms.iterator()?,
      SimpleAutomatonTermsEnum {
        run_automaton: self.run_automaton.clone(),
      },
    );
    terms_enum.set_initial_seek_term(BytesRef::from(""));
    Ok(terms_enum)
  }

  fn as_query(&self) -> Query {
    self.clone().into()
  }
}

impl Eq for DumbRegexpQuery {}

impl PartialEq for DumbRegexpQuery {
  fn eq(&self, other: &Self) -> bool {
    self.field == other.field && self.run_automaton == other.run_automaton
  }
}

impl Hash for DumbRegexpQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.field.hash(state);
    self.run_automaton.hash(state);
  }
}

pub struct SimpleAutomatonTermsEnum {
  run_automaton: CharacterRunAutomaton,
}

impl FilteredTermsEnumBase for SimpleAutomatonTermsEnum {
  fn accept(&mut self, term: &BytesRef<Vec<u8>>, _ord: i64) -> Result<AcceptStatus> {
    if self.run_automaton.run_str(&term.utf8_to_string()?)? {
      Ok(AcceptStatus::Yes)
    } else {
      Ok(AcceptStatus::No)
    }
  }
}

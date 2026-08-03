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
use crate::core::index::BytesRef;
use crate::core::index::filtered_terms_enum::{
  AcceptStatus, FilteredTermsEnum, FilteredTermsEnumBase,
};
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{
  ConstantScoreBlendedRewrite, MultiTermQuery, MultiTermQuerySet, RewriteMethod, RewriteMethodEnum,
};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::automation::character_run_automaton::CharacterRunAutomaton;
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{HasIdentity, StringHelper};
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct BoostCheckingQuery {
  field: String,
  rewrite_method: RewriteMethodEnum,
  id: Identity,
}

impl BoostCheckingQuery {
  pub fn new<T>(field: &str, rewrite_method: T) -> Self
  where
    T: Into<RewriteMethodEnum>,
  {
    Self {
      field: field.to_string(),
      rewrite_method: rewrite_method.into(),
      id: Identity::default(),
    }
  }
}

impl QueryBase for BoostCheckingQuery {
  fn to_string(&self, _field: &str) -> Result<String> {
    Ok("dummy".to_string())
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
    unreachable!("BoostCheckingQuery must be rewritten before weighting")
  }

  fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    self.rewrite_method.clone().rewrite(searcher, self)
  }

  fn visit<QV>(&self, _visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    Ok(())
  }
}

impl Debug for BoostCheckingQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "dummy")
  }
}

impl HasIdentity for BoostCheckingQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl MultiTermQuery for BoostCheckingQuery {
  fn get_field(&self) -> &str {
    &self.field
  }

  type TermsEnum<T>
    = FilteredTermsEnum<T::TermsEnum, BoostCheckingTermsEnum>
  where
    T: Terms;

  fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
  where
    T: Terms + Clone,
  {
    let mut terms_enum = FilteredTermsEnum::new(terms.iterator()?, BoostCheckingTermsEnum);
    terms_enum.set_initial_seek_term(BytesRef::from(""));
    Ok(terms_enum)
  }

  fn to_query(&self) -> Query {
    MultiTermQuerySet::from(self.clone()).into()
  }
}

impl Eq for BoostCheckingQuery {}

impl PartialEq for BoostCheckingQuery {
  fn eq(&self, other: &Self) -> bool {
    self.field == other.field && self.rewrite_method == other.rewrite_method
  }
}

impl Hash for BoostCheckingQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.field.hash(state);
    self.rewrite_method.hash(state);
  }
}

pub struct BoostCheckingTermsEnum;

impl FilteredTermsEnumBase for BoostCheckingTermsEnum {
  fn accept(&mut self, term: &BytesRef<Vec<u8>>, _ord: i64) -> Result<AcceptStatus> {
    if term.length == 0 {
      return Ok(AcceptStatus::No);
    }

    let c = term.bytes[term.offset] as char;
    if c >= '2' {
      if c <= '7' {
        Ok(AcceptStatus::Yes)
      } else {
        Ok(AcceptStatus::End)
      }
    } else {
      Ok(AcceptStatus::No)
    }
  }
}

impl crate::core::util::accountable::Accountable for BoostCheckingQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}

/// A simple prefix query that scans through all terms.
#[derive(Clone)]
pub struct DumbPrefixQuery {
  field: String,
  prefix: BytesRef<Vec<u8>>,
  id: Identity,
}

impl DumbPrefixQuery {
  pub fn new(term: Term) -> Self {
    Self {
      field: term.field().to_string(),
      prefix: term.bytes().clone(),
      id: Identity::default(),
    }
  }
}

impl QueryBase for DumbPrefixQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    if self.field == field {
      Ok(format!("{}", self.prefix))
    } else {
      Ok(format!("{}:{}", self.field, self.prefix))
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

  fn visit<QV>(&self, _visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    Ok(())
  }
}

impl Debug for DumbPrefixQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.to_string("") {
      Ok(s) => write!(f, "{}", s),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl HasIdentity for DumbPrefixQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl MultiTermQuery for DumbPrefixQuery {
  fn get_field(&self) -> &str {
    &self.field
  }

  type TermsEnum<T>
    = FilteredTermsEnum<T::TermsEnum, SimplePrefixTermsEnum>
  where
    T: Terms;

  fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
  where
    T: Terms + Clone,
  {
    let mut terms_enum = FilteredTermsEnum::new(
      terms.iterator()?,
      SimplePrefixTermsEnum {
        prefix: self.prefix.clone(),
      },
    );
    terms_enum.set_initial_seek_term(BytesRef::from(""));
    Ok(terms_enum)
  }

  fn to_query(&self) -> Query {
    MultiTermQuerySet::from(self.clone()).into()
  }
}

impl Eq for DumbPrefixQuery {}

impl PartialEq for DumbPrefixQuery {
  fn eq(&self, other: &Self) -> bool {
    self.field == other.field && self.prefix == other.prefix
  }
}

impl Hash for DumbPrefixQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.field.hash(state);
    self.prefix.hash(state);
  }
}

pub struct SimplePrefixTermsEnum {
  prefix: BytesRef<Vec<u8>>,
}

impl FilteredTermsEnumBase for SimplePrefixTermsEnum {
  fn accept(&mut self, term: &BytesRef<Vec<u8>>, _ord: i64) -> Result<AcceptStatus> {
    if StringHelper::starts_with_byte_ref(term, &self.prefix) {
      Ok(AcceptStatus::Yes)
    } else {
      Ok(AcceptStatus::No)
    }
  }
}

impl crate::core::util::accountable::Accountable for DumbPrefixQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
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
  fn to_string(&self, field: &str) -> Result<String> {
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

  fn visit<QV>(&self, _visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    Ok(())
  }
}

impl Debug for DumbRegexpQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.to_string("") {
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

  fn to_query(&self) -> Query {
    MultiTermQuerySet::from(self.clone()).into()
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

impl crate::core::util::accountable::Accountable for DumbRegexpQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}

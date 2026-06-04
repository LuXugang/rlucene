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
use crate::core::index::prefix_coded_terms::{
  PrefixCodedTermsArc, PrefixCodedTermsBuilder, TermIteratorArc,
};
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{
  ConstantScoreBlendedRewrite, MultiTermQuery, MultiTermQuerySet, RewriteMethod, RewriteMethodEnum,
};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::accountable::Accountable;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct TermInSetQuery {
  id: Identity,
  field: String,
  term_data: PrefixCodedTermsArc,
  term_data_hash_code: u64,
  rewrite_method: RewriteMethodEnum,
}

impl TermInSetQuery {
  /// Create a new TermInSetQuery that matches documents containing any of the specified terms.
  pub fn new<T>(field: T, terms: Vec<BytesRef<Vec<u8>>>) -> Self
  where
    T: Into<String>,
  {
    let field = field.into();
    let term_data = Self::pack_terms(&field, terms);
    Self::from_term_data(field, term_data)
  }

  /// Create a new TermInSetQuery that matches documents containing any of the specified terms.
  pub fn new_with_rewrite_method<R, T>(
    rewrite_method: R,
    field: T,
    terms: Vec<BytesRef<Vec<u8>>>,
  ) -> Self
  where
    R: Into<RewriteMethodEnum>,
    T: Into<String>,
  {
    let field = field.into();
    let term_data = Self::pack_terms(&field, terms);
    Self::from_rewrite_method_and_term_data(rewrite_method, field, term_data)
  }

  fn from_term_data(field: String, term_data: PrefixCodedTermsArc) -> Self {
    Self::from_rewrite_method_and_term_data(ConstantScoreBlendedRewrite, field, term_data)
  }

  fn from_rewrite_method_and_term_data<R>(
    rewrite_method: R,
    field: String,
    term_data: PrefixCodedTermsArc,
  ) -> Self
  where
    R: Into<RewriteMethodEnum>,
  {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    term_data.hash(&mut hasher);
    let term_data_hash_code = hasher.finish();
    Self {
      id: Identity::default(),
      field,
      term_data,
      term_data_hash_code,
      rewrite_method: rewrite_method.into(),
    }
  }

  fn pack_terms(field: &str, mut terms: Vec<BytesRef<Vec<u8>>>) -> PrefixCodedTermsArc {
    // TODO IMPORTANT 这里需要判断是否已经有序
    terms.sort();
    let mut builder = PrefixCodedTermsBuilder::new();
    let mut previous: Option<BytesRef<Vec<u8>>> = None;
    for term in terms {
      if previous.as_ref().is_some_and(|previous| previous == &term) {
        continue;
      }
      builder
        .add(field.to_string(), &term)
        .expect("prefix coding in-memory terms should not fail");
      previous = Some(BytesRef::deep_copy_of(&term));
    }
    builder.finish().into()
  }

  pub fn get_terms_count(&self) -> i64 {
    self.term_data.size()
  }

  pub fn get_bytes_ref_iterator(&self) -> Result<TermIteratorArc> {
    self.term_data.iterator()
  }

  fn equals_to(&self, other: &TermInSetQuery) -> bool {
    self.term_data_hash_code == other.term_data_hash_code && self.term_data == other.term_data
  }
}

impl HasIdentity for TermInSetQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for TermInSetQuery {
  fn to_string(&self, _field: &str) -> Result<String> {
    let mut builder = String::new();
    builder.push_str(&self.field);
    builder.push_str(":(");

    let mut iterator = self.term_data.iterator()?;
    let mut first = true;
    while let Some(term) = iterator.next()? {
      if !first {
        builder.push(' ');
      }
      first = false;
      let term = term.as_ref();
      builder.push_str(&Term::get_string(term).unwrap_or_else(|_| term.to_string()));
    }
    builder.push(')');
    Ok(builder)
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
    let rewrite_method = self.rewrite_method.clone();
    rewrite_method.rewrite(searcher, self)
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
  }
}

impl Debug for TermInSetQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.to_string("") {
      Ok(s) => write!(f, "{}", s),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl Eq for TermInSetQuery {}

impl PartialEq for TermInSetQuery {
  fn eq(&self, other: &Self) -> bool {
    self.equals_to(other)
  }
}

impl Hash for TermInSetQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.term_data_hash_code.hash(state);
  }
}

impl Accountable for TermInSetQuery {
  fn ram_bytes_used(&self) -> Result<i64> {
    self.term_data.ram_bytes_used()
  }
}

impl MultiTermQuery for TermInSetQuery {
  fn get_field(&self) -> &str {
    &self.field
  }

  type TermsEnum<T>
    = FilteredTermsEnum<T::TermsEnum, SetEnum>
  where
    T: Terms;

  fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
  where
    T: Terms + Clone,
  {
    SetEnum::new(terms.iterator()?, self.term_data.clone())
  }

  fn get_terms_count(&self) -> i64 {
    self.get_terms_count()
  }

  fn to_query(&self) -> Query {
    MultiTermQuerySet::from(self.clone()).into()
  }
}

pub struct SetEnum {
  iterator: TermIteratorArc,
  seek_term: Option<BytesRef<Vec<u8>>>,
}

impl SetEnum {
  pub fn new<TE>(
    terms_enum: TE,
    term_data: PrefixCodedTermsArc,
  ) -> Result<FilteredTermsEnum<TE, Self>>
  where
    TE: TermsEnum,
  {
    let mut iterator = term_data.iterator()?;
    let seek_term = iterator.next()?.map(|term| term.into_owned());
    Ok(FilteredTermsEnum::new(
      terms_enum,
      SetEnum {
        iterator,
        seek_term,
      },
    ))
  }
}

impl FilteredTermsEnumBase for SetEnum {
  fn accept(&mut self, term: &BytesRef<Vec<u8>>, _ord: i64) -> Result<AcceptStatus> {
    let mut cmp = std::cmp::Ordering::Equal;
    while let Some(seek_term) = self.seek_term.as_ref()
      && {
        cmp = seek_term.cmp(term);
        cmp.is_lt()
      }
    {
      self.seek_term = self.iterator.next()?.map(|term| term.into_owned());
    }

    match self.seek_term.as_ref() {
      None => Ok(AcceptStatus::End),
      Some(_) if cmp.is_eq() => Ok(AcceptStatus::YesAndSeek),
      Some(_) => Ok(AcceptStatus::NoAndSeek),
    }
  }

  fn next_seek_term(
    &mut self,
    current: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    if current.is_none() {
      return Ok(self.seek_term.as_ref().map(Cow::Borrowed));
    }
    while self
      .seek_term
      .as_ref()
      .is_some_and(|seek_term| seek_term <= current.unwrap())
    {
      self.seek_term = self.iterator.next()?.map(|term| term.into_owned());
    }
    Ok(self.seek_term.as_ref().map(Cow::Borrowed))
  }
}

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
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::search::automaton_query::AutomatonQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{
  ConstantScoreBlendedRewrite, MultiTermQuery, MultiTermQuerySet, RewriteMethod, RewriteMethodEnum,
};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
/// A [`Query`] that matches documents whose terms fall within a specified range.
///
/// This query matches documents containing terms that fall within the given
/// range according to [`BytesRef`].
///
/// **NOTE**: [`TermRangeQuery`] is significantly slower than point-based ranges
/// see [`PointRangeQuery`](crate::core::search::point_range_query::PointRangeQuery) because it must visit all terms that match the range
/// and merge their matches.
///
/// This query uses the [`ConstantScoreBlendedRewrite`]
/// rewrite method.
#[derive(Clone)]
pub struct TermRangeQuery {
  lower_term: Option<BytesRef<Vec<u8>>>,
  upper_term: Option<BytesRef<Vec<u8>>>,
  include_lower: bool,
  include_upper: bool,
  base: AutomatonQuery,
  id: Identity,
}
impl TermRangeQuery {
  /// Constructs a query selecting all terms greater than or equal to `lower_term`
  /// but less than or equal to `upper_term`.
  ///
  /// If an endpoint is `None`, it is considered "open". Either or both endpoints
  /// may be open. Open endpoints may not be exclusive (it is not possible to
  /// select all but the first or last term without explicitly specifying the
  /// term to exclude).
  ///
  /// # Parameters
  ///
  /// - `field`: The field that holds both lower and upper terms.
  /// - `lower_term`: The term text at the lower end of the range.
  /// - `upper_term`: The term text at the upper end of the range.
  /// - `include_lower`: If `true`, `lower_term` is included in the range.
  /// - `include_upper`: If `true`, `upper_term` is included in the range.
  ///
  /// Uses `CONSTANT_SCORE_BLENDED_REWRITE` as the default rewrite method.
  pub fn new<T>(
    field: T,
    lower_term: Option<BytesRef<Vec<u8>>>,
    upper_term: Option<BytesRef<Vec<u8>>>,
    include_lower: bool,
    include_upper: bool,
  ) -> Result<Self>
  where
    T: Into<String>,
  {
    Self::with_rewrite(
      field,
      lower_term,
      upper_term,
      include_lower,
      include_upper,
      ConstantScoreBlendedRewrite,
    )
  }
  /// Constructs a query selecting all terms greater than or equal to `lower_term`
  /// but less than or equal to `upper_term`.
  ///
  /// If an endpoint is `None`, it is considered "open". Either or both endpoints
  /// may be open. Open endpoints may not be exclusive (it is not possible to
  /// select all but the first or last term without explicitly specifying the
  /// term to exclude).
  ///
  /// # Parameters
  ///
  /// - `field`: The field that holds both lower and upper terms.
  /// - `lower_term`: The term text at the lower end of the range.
  /// - `upper_term`: The term text at the upper end of the range.
  /// - `include_lower`: If `true`, `lower_term` is included in the range.
  /// - `include_upper`: If `true`, `upper_term` is included in the range.
  /// - `rewrite_method`: The rewrite method used when building the final query.
  pub fn with_rewrite<T, R>(
    field: T,
    lower_term: Option<BytesRef<Vec<u8>>>,
    upper_term: Option<BytesRef<Vec<u8>>>,
    include_lower: bool,
    include_upper: bool,
    rewrite_method: R,
  ) -> Result<Self>
  where
    T: Into<String>,
    R: Into<RewriteMethodEnum>,
  {
    let automaton = to_automaton(
      lower_term.as_ref(),
      upper_term.as_ref(),
      include_lower,
      include_upper,
    )?;
    let lower = match lower_term {
      Some(ref lt) => lt.clone(),
      None => BytesRef::default(),
    };
    let base = AutomatonQuery::new(Term::new(field, lower), automaton, true, rewrite_method)?;

    Ok(Self {
      lower_term,
      upper_term,
      include_lower,
      include_upper,
      base,
      id: Identity::default(),
    })
  }
  /// Factory that creates a new [`TermRangeQuery`] using `String` values
  /// for term text.
  ///
  /// Uses [`ConstantScoreBlendedRewrite`] as the default rewrite method.
  pub fn new_string_range<F>(
    field: F,
    lower_term: Option<impl AsRef<str>>,
    upper_term: Option<impl AsRef<str>>,
    include_lower: bool,
    include_upper: bool,
  ) -> Result<Self>
  where
    F: Into<String>,
  {
    Self::new_string_range_with_rewrite(
      field,
      lower_term,
      upper_term,
      include_lower,
      include_upper,
      ConstantScoreBlendedRewrite,
    )
  }

  /// Factory that creates a new [`TermRangeQuery`] using `String` values
  /// for term text.
  pub fn new_string_range_with_rewrite<F, R>(
    field: F,
    lower_term: Option<impl AsRef<str>>,
    upper_term: Option<impl AsRef<str>>,
    include_lower: bool,
    include_upper: bool,
    rewrite_method: R,
  ) -> Result<Self>
  where
    F: Into<String>,
    R: Into<RewriteMethodEnum>,
  {
    let lower = lower_term.map(|s| BytesRef::from_string(s.as_ref()));
    let upper = upper_term.map(|s| BytesRef::from_string(s.as_ref()));

    Self::with_rewrite(
      field,
      lower,
      upper,
      include_lower,
      include_upper,
      rewrite_method,
    )
  }
  /// Returns the lower value of this range query.
  pub fn lower_term(&self) -> Option<&BytesRef<Vec<u8>>> {
    self.lower_term.as_ref()
  }

  /// Returns the upper value of this range query.
  pub fn upper_term(&self) -> Option<&BytesRef<Vec<u8>>> {
    self.upper_term.as_ref()
  }

  /// Returns `true` if the lower endpoint is inclusive.
  pub fn includes_lower(&self) -> bool {
    self.include_lower
  }

  /// Returns `true` if the upper endpoint is inclusive.
  pub fn includes_upper(&self) -> bool {
    self.include_upper
  }
}

impl QueryBase for TermRangeQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    let mut buffer = String::new();

    if self.base.get_field() != field {
      buffer.push_str(self.base.get_field());
      buffer.push(':');
    }

    buffer.push(if self.include_lower { '[' } else { '{' });

    let lower_str = match self.lower_term.as_ref() {
      Some(term) => {
        let s = term.utf8_to_string()?;
        if s == "*" { "\\*".to_string() } else { s }
      },
      None => "*".to_string(),
    };
    buffer.push_str(&lower_str);

    buffer.push_str(" TO ");

    let upper_str = match self.upper_term.as_ref() {
      Some(term) => {
        let s = term.utf8_to_string()?;
        if s == "*" { "\\*".to_string() } else { s }
      },
      None => "*".to_string(),
    };
    buffer.push_str(&upper_str);

    buffer.push(if self.include_upper { ']' } else { '}' });

    Ok(buffer)
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
    let rewrite_method = self.base.rewrite_method.clone();
    rewrite_method.rewrite(searcher, self)
  }

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    self.base.visit_with_query(visitor, self.into())
  }
}

impl Debug for TermRangeQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.to_string("") {
      Ok(s) => write!(f, "{}", s),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl HasIdentity for TermRangeQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl MultiTermQuery for TermRangeQuery {
  fn get_field(&self) -> &str {
    self.base.get_field()
  }

  type TermsEnum<T>
    = <AutomatonQuery as MultiTermQuery>::TermsEnum<T>
  where
    T: Terms;

  fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
  where
    T: Terms + Clone,
  {
    self.base.compiled.get_terms_enum(terms)
  }

  fn to_query(&self) -> Query {
    MultiTermQuerySet::from(self.clone()).into()
  }
}
impl Hash for TermRangeQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: std::hash::Hasher,
  {
    self.base.hash(state);
    if self.include_lower {
      1231.hash(state);
    } else {
      1237.hash(state);
    }

    if self.include_upper {
      1231.hash(state);
    } else {
      1237.hash(state);
    }

    self.lower_term.hash(state);
    self.upper_term.hash(state);
  }
}
impl Eq for TermRangeQuery {}
impl PartialEq for TermRangeQuery {
  fn eq(&self, other: &Self) -> bool {
    self.base == other.base
      && self.include_lower == other.include_lower
      && self.include_upper == other.include_upper
      && self.lower_term == other.lower_term
      && self.upper_term == other.upper_term
  }
}
pub fn to_automaton(
  lower_term: Option<&BytesRef<Vec<u8>>>,
  upper_term: Option<&BytesRef<Vec<u8>>>,
  mut include_lower: bool,
  mut include_upper: bool,
) -> Result<Automaton> {
  if lower_term.is_none() {
    include_lower = true;
  }

  if upper_term.is_none() {
    include_upper = true;
  }

  Automata::make_binary_interval(lower_term, include_lower, upper_term, include_upper)
}

impl crate::core::util::accountable::Accountable for TermRangeQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}

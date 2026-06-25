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
use crate::core::util::automation::operations::Operations;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
/// Implements the wildcard search query. Supported wildcards are `*`, which matches any
/// character sequence (including the empty one), and `?`, which matches any single
/// character. '\' is the escape character.
///
/// Note this query can be slow, as it needs to iterate over many terms. In order to prevent
/// extremely slow WildcardQueries, a Wildcard term should not start with the wildcard `*`
///
/// This query uses the [`ConstantScoreBlendedRewrite`] rewrite method.
///
/// See [`AutomatonQuery`].
#[derive(Clone)]
pub struct WildcardQuery {
  determinize_work_limit: i32,
  base: AutomatonQuery,
  id: Identity,
}

impl WildcardQuery {
  /// String equality with support for wildcards
  pub const WILDCARD_STRING: char = '*';

  /// Char equality with support for wildcards
  pub const WILDCARD_CHAR: char = '?';

  /// Escape character
  pub const WILDCARD_ESCAPE: char = '\\';

  /// Constructs a query for terms matching `term`.
  pub fn new(term: Term) -> Result<Self> {
    Self::with_determinize_work_limit(term, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32)
  }

  /// Constructs a query for terms matching `term`.
  pub fn with_determinize_work_limit(term: Term, determinize_work_limit: i32) -> Result<Self> {
    Self::with_rewrite(term, determinize_work_limit, ConstantScoreBlendedRewrite)
  }

  /// Constructs a query for terms matching `term`.
  pub fn with_rewrite<R>(term: Term, determinize_work_limit: i32, rewrite_method: R) -> Result<Self>
  where
    R: Into<RewriteMethodEnum>,
  {
    let automaton = to_automaton(&term, determinize_work_limit)?;
    let base = AutomatonQuery::new(term, automaton, false, rewrite_method)?;
    Ok(Self {
      determinize_work_limit,
      base,
      id: Identity::default(),
    })
  }
}

impl QueryBase for WildcardQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    let mut buffer = String::new();

    if self.base.get_field() != field {
      buffer.push_str(self.base.get_field());
      buffer.push(':');
    }

    buffer.push_str(&self.base.term.text()?);
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

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
    todo!()
  }
}

impl Debug for WildcardQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.to_string("") {
      Ok(s) => write!(f, "{}", s),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl HasIdentity for WildcardQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl MultiTermQuery for WildcardQuery {
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

impl Hash for WildcardQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: std::hash::Hasher,
  {
    self.base.hash(state);
  }
}

impl Eq for WildcardQuery {}

impl PartialEq for WildcardQuery {
  fn eq(&self, other: &Self) -> bool {
    self.base == other.base
  }
}

pub fn to_automaton(wildcard_query: &Term, determinize_work_limit: i32) -> Result<Automaton> {
  let mut automata = Vec::new();
  let wildcard_text = wildcard_query.text()?;
  let chars: Vec<char> = wildcard_text.chars().collect();

  let mut i: usize = 0;
  while i < chars.len() {
    let c = chars[i];

    match c {
      WildcardQuery::WILDCARD_STRING => {
        automata.push(Automata::make_any_string()?);
      },
      WildcardQuery::WILDCARD_CHAR => {
        automata.push(Automata::make_any_char()?);
      },
      WildcardQuery::WILDCARD_ESCAPE if i + 1 < chars.len() => {
        let next = chars[i + 1] as i32;
        automata.push(Automata::make_char(next)?);
        i += 1;
      },
      _ => {
        let cp = c as i32;
        automata.push(Automata::make_char(cp)?);
      },
    }

    i += 1;
  }
  let automata = automata.iter().collect::<Vec<_>>();
  let a = Operations::concatenate_with_list(automata.as_ref())?;
  let v = Operations::determinize(&a, determinize_work_limit as usize)?;
  match v {
    Cow::Borrowed(_) => Ok(a),
    Cow::Owned(v) => Ok(v),
  }
}

impl crate::core::util::accountable::Accountable for WildcardQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}

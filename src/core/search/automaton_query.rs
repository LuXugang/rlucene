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
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{
  ConstantScoreBlendedRewrite, MultiTermQuery, MultiTermQuerySet, RewriteMethod, RewriteMethodEnum,
};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::accountable::Accountable;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::compiled_automaton::{CompiledAutomaton, CompiledAutomatonTE};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
/// A [`Query`] that will match terms against a finite-state machine.
///
/// This query will match documents that contain terms accepted by a given finite-state machine.
/// The automaton can be constructed with the automaton API.
/// Alternatively, it can be created from a regular expression with [`RegexpQuery`](crate::core::search::regexp_query::RegexpQuery) or from the
/// standard Lucene wildcard syntax with [`WildcardQuery`](crate::core::search::regexp_query::RegexpQuery).
///
/// When the query is executed, it will will enumerate the term dictionary in an intelligent way
/// to reduce the number of comparisons. For example: the regular expression of `[dl]og?`
/// will make approximately four comparisons: do, dog, lo, and log.
///

#[derive(Clone)]
pub struct AutomatonQuery {
  pub(crate) compiled: CompiledAutomaton,
  pub(crate) term: Term,
  #[allow(dead_code)]
  automaton_is_binary: bool,
  ram_bytes_used: i64,
  id: Identity,
  pub(crate) rewrite_method: RewriteMethodEnum,
}
impl AutomatonQuery {
  /// Create a new `AutomatonQuery` from an [`Automaton`].
  ///
  /// - `term`: [`Term`] containing field and possibly some pattern structure. The term text is
  ///   ignored.
  /// - `automaton`: [`Automaton`] to run, terms that are accepted are considered a match.
  pub fn from_automaton(term: Term, automaton: Automaton) -> Result<Self> {
    Self::from_automaton_with_binary(term, automaton, false)
  }

  /// Create a new `AutomatonQuery` from an [`Automaton`].
  ///
  /// - `term`: [`Term`] containing field and possibly some pattern structure. The term text is
  ///   ignored.
  /// - `automaton`: [`Automaton`] to run, terms that are accepted are considered a match.
  /// - `is_binary`: if `true`, this automaton is already binary and will not go through the
  ///   UTF32ToUTF8 conversion.
  pub fn from_automaton_with_binary(
    term: Term,
    automaton: Automaton,
    is_binary: bool,
  ) -> Result<Self> {
    Self::new(term, automaton, is_binary, ConstantScoreBlendedRewrite)
  }
  /// Create a new `AutomatonQuery` from an [`Automaton`].
  ///
  /// - `term`: [`Term`] containing field and possibly some pattern structure. The term text is
  ///   ignored.
  /// - `automaton`: [`Automaton`] to run, terms that are accepted are considered a match.
  /// - `is_binary`: unused.
  /// - `rewrite_method`: the rewrite method to use to build the final query from the automaton.
  pub fn new<T>(
    term: Term,
    automaton: Automaton,
    is_binary: bool,
    rewrite_method: T,
  ) -> Result<Self>
  where
    T: Into<RewriteMethodEnum>,
  {
    let rewrite_method = rewrite_method.into();
    let compiled = CompiledAutomaton::with_binary(automaton, false, true, is_binary)?;
    let ram_bytes_used = term
      .ram_bytes_used()?
      .saturating_add(compiled.ram_bytes_used()?);

    Ok(Self {
      compiled,
      term,
      #[allow(dead_code)]
      automaton_is_binary: is_binary,
      ram_bytes_used,
      id: Identity::new(),
      rewrite_method,
    })
  }
  #[cfg(test)]
  pub(crate) fn get_compiled(&self) -> &CompiledAutomaton {
    &self.compiled
  }
}

impl QueryBase for AutomatonQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    let mut buffer = String::new();

    if self.term.field() != field {
      buffer.push_str(self.term.field());
      buffer.push(':');
    }

    buffer.push_str("AutomatonQuery");
    buffer.push_str(" {");
    buffer.push('\n');
    buffer.push('}');

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
    let rewrite_method = self.rewrite_method.clone();
    rewrite_method.rewrite(searcher, self)
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
    todo!()
  }
}

impl Debug for AutomatonQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.to_string("") {
      Ok(s) => write!(f, "{}", s),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl HasIdentity for AutomatonQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl Accountable for AutomatonQuery {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(self.ram_bytes_used)
  }
}
impl PartialEq for AutomatonQuery {
  fn eq(&self, other: &Self) -> bool {
    if std::ptr::eq(self, other) {
      return true;
    }
    self.compiled == other.compiled && self.term == other.term
  }
}
impl Eq for AutomatonQuery {}
impl Hash for AutomatonQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.compiled.hash(state);
    self.term.hash(state);
  }
}
impl MultiTermQuery for AutomatonQuery {
  fn get_field(&self) -> &str {
    self.term.field()
  }

  type TermsEnum<T>
    = CompiledAutomatonTE<T>
  where
    T: Terms;

  fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
  where
    T: Terms + Clone,
  {
    self.compiled.get_terms_enum(terms)
  }

  fn to_query(&self) -> Query {
    MultiTermQuerySet::from(self.clone()).into()
  }
}

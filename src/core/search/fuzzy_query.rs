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
use crate::core::index::filtered_terms_enum::FilteredTermsEnum;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::single_terms_enum::SingleTermsEnum;
use crate::core::index::term::Term;
use crate::core::index::terms::{Terms, TermsTE};
use crate::core::search::fuzzy_automaton_builder::FuzzyAutomatonBuilder;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{
  MultiTermQuery, RewriteMethod, RewriteMethodEnum, TopTermsBlendedFreqScoringRewrite,
};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::automation::compiled_automaton::CompiledAutomaton;
use crate::core::util::automation::levenshtein_automata::LevenshteinAutomata;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
/// Implements the fuzzy search query. The similarity measurement is based on the
/// Damerau-Levenshtein (optimal string alignment) algorithm, though you can explicitly choose
/// classic Levenshtein by passing `false` to the `transpositions` parameter.
///
/// This query uses [`TopTermsBlendedFreqScoringRewrite`] as default. So terms will be collected and
/// scored according to their edit distance. Only the top terms are used for building the
/// [`BooleanQuery`]. It is not recommended to change the rewrite mode for fuzzy queries.
///
/// At most, this query will match terms up to
/// [`LevenshteinAutomata::MAXIMUM_SUPPORTED_DISTANCE`] edits. Higher distances (especially with
/// transpositions enabled), are generally not useful and will match a significant amount of the term
/// dictionary. If you really want this, consider using an n-gram indexing technique instead.
///
/// NOTE: terms of length 1 or 2 will sometimes not match because of how the scaled distance between
/// two terms is computed. For a term to match, the edit distance between the terms must be less than
/// the minimum length term (either the input term, or the candidate term). For example, `FuzzyQuery`
/// on term `"abcd"` with `maxEdits=2` will not match an indexed term `"ab"`, and `FuzzyQuery` on
/// term `"a"` with `maxEdits=2` will not match an indexed term `"abc"`.
#[derive(Clone)]
pub struct FuzzyQuery {
  max_edits: i32,
  max_expansions: usize,
  transpositions: bool,
  prefix_length: usize,
  term: Term,
  rewrite_method: RewriteMethodEnum,
  id: Identity,
}

impl FuzzyQuery {
  pub const DEFAULT_MAX_EDITS: i32 = LevenshteinAutomata::MAXIMUM_SUPPORTED_DISTANCE;
  pub const DEFAULT_PREFIX_LENGTH: usize = 0;
  pub const DEFAULT_MAX_EXPANSIONS: usize = 50;
  pub const DEFAULT_TRANSPOSITIONS: bool = true;
  /// Creates a default top-terms blended frequency scoring rewrite with the given max expansions
  pub fn default_rewrite_method(max_expansions: usize) -> TopTermsBlendedFreqScoringRewrite {
    TopTermsBlendedFreqScoringRewrite::new(max_expansions)
  }

  pub fn new(term: Term) -> Result<Self> {
    Self::with_max_edits(term, Self::DEFAULT_MAX_EDITS)
  }

  pub fn with_max_edits(term: Term, max_edits: i32) -> Result<Self> {
    Self::with_max_edits_and_prefix(term, max_edits, Self::DEFAULT_PREFIX_LENGTH)
  }

  pub fn with_max_edits_and_prefix(
    term: Term,
    max_edits: i32,
    prefix_length: usize,
  ) -> Result<Self> {
    Self::with_options(
      term,
      max_edits,
      prefix_length,
      Self::DEFAULT_MAX_EXPANSIONS,
      Self::DEFAULT_TRANSPOSITIONS,
    )
  }

  pub fn with_options(
    term: Term,
    max_edits: i32,
    prefix_length: usize,
    max_expansions: usize,
    transpositions: bool,
  ) -> Result<Self> {
    Self::with_rewrite(
      term,
      max_edits,
      prefix_length,
      max_expansions,
      transpositions,
      Self::default_rewrite_method(max_expansions),
    )
  }
  /// Create a new `FuzzyQuery` that will match terms with an edit distance of at most `max_edits` to
  /// `term`. If a `prefix_length` > 0 is specified, a common prefix of that length is also required.
  ///
  /// # Parameters
  ///
  /// - `term`: the term to search for.
  /// - `max_edits`: must be `>= 0` and `<=` [`LevenshteinAutomata::MAXIMUM_SUPPORTED_DISTANCE`].
  /// - `prefix_length`: length of common (non-fuzzy) prefix.
  /// - `max_expansions`: the maximum number of terms to match. If this number is greater than
  ///   [`IndexSearcher::get_max_clause_count`] when the query is rewritten, then the
  ///   `max_clause_count` will be used instead.
  /// - `transpositions`: `true` if transpositions should be treated as a primitive edit operation.
  ///   If this is `false`, comparisons will implement the classic Levenshtein algorithm.
  /// - `rewrite_method`: the rewrite method to use to build the final query.
  pub fn with_rewrite<R>(
    term: Term,
    max_edits: i32,
    prefix_length: usize,
    max_expansions: usize,
    transpositions: bool,
    rewrite_method: R,
  ) -> Result<Self>
  where
    R: Into<RewriteMethodEnum>,
  {
    if max_edits > LevenshteinAutomata::MAXIMUM_SUPPORTED_DISTANCE {
      return Err(LuceneError::illegal_argument(format!(
        "maxEdits must be between 0 and {}",
        LevenshteinAutomata::MAXIMUM_SUPPORTED_DISTANCE
      )));
    }
    if max_expansions == 0 {
      return Err(LuceneError::illegal_argument(
        "maxExpansions must be positive.",
      ));
    }

    Ok(Self {
      term,
      max_edits,
      prefix_length,
      transpositions,
      max_expansions,
      rewrite_method: rewrite_method.into(),
      id: Identity::default(),
    })
  }
  /// Returns:
  // the maximum number of edit distances allowed for this query to match.
  pub fn get_max_edits(&self) -> i32 {
    self.max_edits
  }
  /// Returns the non-fuzzy prefix length.
  /// This is the number of characters at the start of a term that must be identical (not fuzzy) to the query term if the query is to match that term.
  pub fn get_prefix_length(&self) -> usize {
    self.prefix_length
  }
  /// Returns true if transpositions should be treated as a primitive edit operation.
  /// If this is false, comparisons will implement the classic Levenshtein algorithm.
  pub fn get_transpositions(&self) -> bool {
    self.transpositions
  }
  /// Returns the compiled automata used to match terms
  pub fn get_automata(&self) -> Result<CompiledAutomaton> {
    Self::get_fuzzy_automaton(
      self.term.text()?,
      self.max_edits,
      self.prefix_length,
      self.transpositions,
    )
  }
  /// Returns the [`CompiledAutomaton`] internally used by [`FuzzyQuery`] to match terms. This is a
  /// very low-level method and may no longer exist in case the implementation of fuzzy-matching
  /// changes in the future.
  ///
  /// # Parameters
  ///
  /// - `term`: the term to search for.
  /// - `max_edits`: must be `>= 0` and `<=`
  ///   [`LevenshteinAutomata::MAXIMUM_SUPPORTED_DISTANCE`].
  /// - `prefix_length`: length of common (non-fuzzy) prefix.
  /// - `transpositions`: `true` if transpositions should be treated as a primitive edit operation.
  ///   If this is `false`, comparisons will implement the classic Levenshtein algorithm.
  ///
  /// Returns a [`CompiledAutomaton`] that matches terms that satisfy input parameters.
  pub fn get_fuzzy_automaton<T>(
    term: T,
    max_edits: i32,
    prefix_length: usize,
    transpositions: bool,
  ) -> Result<CompiledAutomaton>
  where
    T: Into<String>,
  {
    let builder = FuzzyAutomatonBuilder::new(term, max_edits, prefix_length, transpositions)?;
    builder.build_max_edit_automaton()
  }

  pub fn get_term(&self) -> &Term {
    &self.term
  }

  pub fn float_to_edits(minimum_similarity: f32, term_len: usize) -> i32 {
    if minimum_similarity >= 1.0 {
      minimum_similarity.min(LevenshteinAutomata::MAXIMUM_SUPPORTED_DISTANCE as f32) as i32
    } else if minimum_similarity == 0.0 {
      0
    } else {
      ((1.0 - minimum_similarity) as f64 * term_len as f64)
        .min(LevenshteinAutomata::MAXIMUM_SUPPORTED_DISTANCE as f64) as i32
    }
  }
}

impl QueryBase for FuzzyQuery {
  fn as_string(&self, field: &str) -> Result<String> {
    let mut buffer = String::new();
    if self.term.field() != field {
      buffer.push_str(self.term.field());
      buffer.push(':');
    }
    buffer.push_str(&self.term.text()?);
    buffer.push('~');
    buffer.push_str(&self.max_edits.to_string());
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

impl Debug for FuzzyQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.as_string("") {
      Ok(s) => write!(f, "{}", s),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl HasIdentity for FuzzyQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl MultiTermQuery for FuzzyQuery {
  fn get_field(&self) -> &str {
    self.term.field()
  }

  type TermsEnum<T>
    = FilteredTermsEnum<TermsTE<T>, SingleTermsEnum>
  where
    T: Terms;

  fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
  where
    T: Terms + Clone,
  {
    if self.max_edits == 0 {
      return Ok(SingleTermsEnum::new(
        terms.iterator()?,
        self.term.bytes().clone(),
      ));
    }
    // TODO IMPORTANT FuzzyTermsEnum未实现
    Err(LuceneError::unsupported_operation(
      "FuzzyQuery with maxEdits > 0 is unsupported.",
    ))
  }

  fn as_query(&self) -> Query {
    self.clone().into()
  }
}

impl Hash for FuzzyQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.term.hash(state);
    self.max_edits.hash(state);
    self.prefix_length.hash(state);
    self.max_expansions.hash(state);
    self.transpositions.hash(state);
    self.rewrite_method.hash(state);
  }
}

impl Eq for FuzzyQuery {}

impl PartialEq for FuzzyQuery {
  fn eq(&self, other: &Self) -> bool {
    self.term == other.term
      && self.max_edits == other.max_edits
      && self.prefix_length == other.prefix_length
      && self.max_expansions == other.max_expansions
      && self.transpositions == other.transpositions
      && self.rewrite_method == other.rewrite_method
  }
}

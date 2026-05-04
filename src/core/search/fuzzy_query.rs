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
use crate::core::codecs::block_term_state::TermStateEnum;
use crate::core::index::BytesRef;
use crate::core::index::filtered_terms_enum::FilteredTermsEnum;
use crate::core::index::impacts_enum::ImpactsEnumEnum2;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::single_terms_enum::SingleTermsEnum;
use crate::core::index::term::Term;
use crate::core::index::terms::{Terms, TermsIntersect, TermsPosting, TermsTE};
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::search::fuzzy_automaton_builder::FuzzyAutomatonBuilder;
use crate::core::search::fuzzy_terms_enum::FuzzyTermsEnum;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{
  MultiTermQuery, RewriteMethod, RewriteMethodEnum, TopTermsBlendedFreqScoringRewrite,
};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::attribute_source::AttributeSourceEnum2;
use crate::core::util::automation::compiled_automaton::CompiledAutomaton;
use crate::core::util::automation::levenshtein_automata::LevenshteinAutomata;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

/// Implements the fuzzy search query. The similarity measurement is based on the
/// Damerau-Levenshtein (optimal string alignment) algorithm, though you can explicitly choose
/// classic Levenshtein by passing `false` to the `transpositions` parameter.
///
/// This query uses [`TopTermsBlendedFreqScoringRewrite`] as default. So terms will be collected and
/// scored according to their edit distance. Only the top terms are used for building the
/// `BooleanQuery`. It is not recommended to change the rewrite mode for fuzzy queries.
///
/// At most, this query will match terms up to
/// `LevenshteinAutomata::MAXIMUM_SUPPORTED_DISTANCE` edits. Higher distances (especially with
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
  /// - `max_edits`: must be `>= 0` and `<=` `LevenshteinAutomata::MAXIMUM_SUPPORTED_DISTANCE`.
  /// - `prefix_length`: length of common (non-fuzzy) prefix.
  /// - `max_expansions`: the maximum number of terms to match. If this number is greater than
  ///   `IndexSearcher::get_max_clause_count` when the query is rewritten, then the
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
    if !(0..=LevenshteinAutomata::MAXIMUM_SUPPORTED_DISTANCE).contains(&max_edits) {
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
  ///   `LevenshteinAutomata::MAXIMUM_SUPPORTED_DISTANCE`.
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

pub enum FuzzyQueryTermsEnum<T>
where
  T: Terms,
{
  Single(FilteredTermsEnum<TermsTE<T>, SingleTermsEnum>),
  Fuzzy(FuzzyTermsEnum<T>),
}

impl<T> BytesRefIterator for FuzzyQueryTermsEnum<T>
where
  T: Terms,
  TermsIntersect<T>: TermsEnum<PostingsEnum = TermsPosting<T>>,
{
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    match self {
      Self::Single(t) => t.next(),
      Self::Fuzzy(t) => t.next(),
    }
  }

  fn set_next(&mut self) -> Result<bool> {
    match self {
      Self::Single(t) => t.set_next(),
      Self::Fuzzy(t) => t.set_next(),
    }
  }
}

impl<T> TermsEnum for FuzzyQueryTermsEnum<T>
where
  T: Terms,
  TermsIntersect<T>: TermsEnum<PostingsEnum = TermsPosting<T>>,
{
  type AttributeSource<'a>
    = AttributeSourceEnum2<
    <FilteredTermsEnum<TermsTE<T>, SingleTermsEnum> as TermsEnum>::AttributeSource<'a>,
    <FuzzyTermsEnum<T> as TermsEnum>::AttributeSource<'a>,
  >
  where
    Self: 'a;
  type AttributeSourceMut<'a>
    = AttributeSourceEnum2<
    <FilteredTermsEnum<TermsTE<T>, SingleTermsEnum> as TermsEnum>::AttributeSourceMut<'a>,
    <FuzzyTermsEnum<T> as TermsEnum>::AttributeSourceMut<'a>,
  >
  where
    Self: 'a;

  fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
    match self {
      Self::Single(t) => Ok(AttributeSourceEnum2::A(t.attributes()?)),
      Self::Fuzzy(t) => Ok(AttributeSourceEnum2::B(t.attributes()?)),
    }
  }

  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
    match self {
      Self::Single(t) => Ok(AttributeSourceEnum2::A(t.attributes_mut()?)),
      Self::Fuzzy(t) => Ok(AttributeSourceEnum2::B(t.attributes_mut()?)),
    }
  }

  fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
    match self {
      Self::Single(t) => t.seek_exact(term),
      Self::Fuzzy(t) => t.seek_exact(term),
    }
  }

  fn prepare_seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    match self {
      Self::Single(t) => t.prepare_seek_exact(text),
      Self::Fuzzy(t) => t.prepare_seek_exact(text),
    }
  }

  fn get_prepare_seek_exact_status(&mut self, target: &BytesRef<Vec<u8>>) -> Result<bool> {
    match self {
      Self::Single(t) => t.get_prepare_seek_exact_status(target),
      Self::Fuzzy(t) => t.get_prepare_seek_exact_status(target),
    }
  }

  fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    match self {
      Self::Single(t) => t.seek_ceil(term),
      Self::Fuzzy(t) => t.seek_ceil(term),
    }
  }

  fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
    match self {
      Self::Single(t) => t.seek_exact_with_ord(ord),
      Self::Fuzzy(t) => t.seek_exact_with_ord(ord),
    }
  }

  fn seek_exact_with_state(
    &mut self,
    term: &BytesRef<Vec<u8>>,
    state: &TermStateEnum,
  ) -> Result<()> {
    match self {
      Self::Single(t) => t.seek_exact_with_state(term, state),
      Self::Fuzzy(t) => t.seek_exact_with_state(term, state),
    }
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    match self {
      Self::Single(t) => t.term(),
      Self::Fuzzy(t) => t.term(),
    }
  }

  fn ord(&self) -> Result<i64> {
    match self {
      Self::Single(t) => t.ord(),
      Self::Fuzzy(t) => t.ord(),
    }
  }

  fn doc_freq(&mut self) -> Result<i32> {
    match self {
      Self::Single(t) => t.doc_freq(),
      Self::Fuzzy(t) => t.doc_freq(),
    }
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    match self {
      Self::Single(t) => t.total_term_freq(),
      Self::Fuzzy(t) => t.total_term_freq(),
    }
  }

  type PostingsEnum = TermsPosting<T>;

  fn postings(&mut self, reuse: Option<Self::PostingsEnum>) -> Result<Self::PostingsEnum> {
    match self {
      Self::Single(t) => t.postings(reuse),
      Self::Fuzzy(t) => t.postings(reuse),
    }
  }

  fn postings_with_flags(
    &mut self,
    reuse: Option<Self::PostingsEnum>,
    flags: i32,
  ) -> Result<Self::PostingsEnum> {
    match self {
      Self::Single(t) => t.postings_with_flags(reuse, flags),
      Self::Fuzzy(t) => t.postings_with_flags(reuse, flags),
    }
  }

  type ImpactsEnum = ImpactsEnumEnum2<
    <FilteredTermsEnum<TermsTE<T>, SingleTermsEnum> as TermsEnum>::ImpactsEnum,
    <FuzzyTermsEnum<T> as TermsEnum>::ImpactsEnum,
  >;

  fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
    match self {
      Self::Single(t) => Ok(ImpactsEnumEnum2::A(t.impacts(flags)?)),
      Self::Fuzzy(t) => Ok(ImpactsEnumEnum2::B(t.impacts(flags)?)),
    }
  }

  fn term_state(&mut self) -> Result<TermStateEnum> {
    match self {
      Self::Single(t) => t.term_state(),
      Self::Fuzzy(t) => t.term_state(),
    }
  }
}

impl MultiTermQuery for FuzzyQuery {
  fn get_field(&self) -> &str {
    self.term.field()
  }

  type TermsEnum<T>
    = FuzzyQueryTermsEnum<T>
  where
    T: Terms,
    TermsIntersect<T>: TermsEnum<PostingsEnum = TermsPosting<T>>;

  fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
  where
    T: Terms + Clone,
    TermsIntersect<T>: TermsEnum<PostingsEnum = TermsPosting<T>>,
  {
    if self.max_edits == 0 {
      return Ok(FuzzyQueryTermsEnum::Single(SingleTermsEnum::new(
        terms.iterator()?,
        self.term.bytes().clone(),
      )));
    }
    Ok(FuzzyQueryTermsEnum::Fuzzy(FuzzyTermsEnum::new_with_attrs(
      terms,
      self.term.clone(),
      self.max_edits,
      self.prefix_length,
      self.transpositions,
    )?))
  }

  fn as_query(&self) -> Query {
    self.clone().into()
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::document::field_type::FieldType;
  use crate::core::index::indexable_field::IndexableField;
  use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
  use crate::core::index::multi_reader::MultiReader;
  use crate::core::index::stored_fields::StoredFields;
  use crate::core::search::boolean_clause::Occur;
  use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
  use crate::core::search::multi_term_query::TopTermsBoostOnlyBooleanQueryRewrite;
  use crate::core::search::similarities_impl::classic_similarity;
  use crate::core::store::directory::DirEnum;
  use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
  use crate::test::core::index::random_index_writer::RandomIndexWriter;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    at_least, new_directory_shared, new_index_writer_config_with_analyzer, new_merge_policy,
    new_searcher_with_reader, new_string_field, new_text_field, random,
  };
  use crate::test::core::util::test_util::TestUtil;
  use rand::{Rng, RngExt};
  use std::cmp::Ordering;
  use std::collections::{HashMap, HashSet};

  #[allow(dead_code)] // for quick search
  struct TestFuzzyQuery;
  #[test]
  fn test_basic_prefix() -> Result<()> {
    let mut random = random();
    let directory = new_directory_shared(&mut random)?;
    let writer = RandomIndexWriter::new(&mut random, directory.clone());
    let mut field_to_type = HashMap::new();
    add_doc(&mut random, "abc", &writer, &mut field_to_type)?;

    let reader = writer.get_reader()?;
    let searcher = new_searcher_with_reader(reader)?;
    writer.close()?;

    let query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "abc"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      1,
    )?;

    let hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(1, hits.len());

    Ok(())
  }
  #[test]
  fn test_fuzziness() -> Result<()> {
    let mut random = random();
    let directory = new_directory_shared(&mut random)?;

    let mock = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    iwc.set_merge_policy(new_merge_policy(&mut random, false)?);

    let writer = RandomIndexWriter::with_config(&mut random, directory.clone(), iwc);
    let mut field_to_type = HashMap::new();

    add_doc(&mut random, "aaaaa", &writer, &mut field_to_type)?;
    add_doc(&mut random, "aaaab", &writer, &mut field_to_type)?;
    add_doc(&mut random, "aaabb", &writer, &mut field_to_type)?;
    add_doc(&mut random, "aabbb", &writer, &mut field_to_type)?;
    add_doc(&mut random, "abbbb", &writer, &mut field_to_type)?;
    add_doc(&mut random, "bbbbb", &writer, &mut field_to_type)?;
    add_doc(&mut random, "ddddd", &writer, &mut field_to_type)?;

    let reader = writer.get_reader()?;
    let searcher = new_searcher_with_reader(reader)?;
    writer.close()?;

    let mut query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaaaa"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      0,
    )?;
    let mut hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(3, hits.len());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaaaa"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      1,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(3, hits.len());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaaaa"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      2,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(3, hits.len());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaaaa"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      3,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(3, hits.len());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaaaa"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      4,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(2, hits.len());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaaaa"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      5,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(1, hits.len());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaaaa"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      6,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(1, hits.len());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "bbbbb"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      0,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(3, hits.len(), "3 documents should match");

    let mut order = vec!["bbbbb", "abbbb", "aabbb"];
    let mut stored_fields = searcher.stored_fields()?;
    for i in 0..hits.len() {
      let document = stored_fields.document(hits[i].doc)?;
      let term = document
        .get_field("field")
        .unwrap()
        .string_value()?
        .unwrap();
      assert_eq!(order[i], term.as_ref().as_str());
    }

    query = FuzzyQuery::with_options(
      Term::from_text("field", "bbbbb"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      0,
      2,
      false,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(2, hits.len(), "only 2 documents should match");

    order = vec!["bbbbb", "abbbb"];
    for i in 0..hits.len() {
      let document = stored_fields.document(hits[i].doc)?;
      let term = document
        .get_field("field")
        .unwrap()
        .string_value()?
        .unwrap();
      assert_eq!(order[i], term.as_ref().as_str());
    }

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "xxxxx"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      0,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(0, hits.len());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaccc"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      0,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(0, hits.len());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaaaa"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      0,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(3, hits.len());

    let document = stored_fields.document(hits[0].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaaaa", term.as_ref().as_str());

    let document = stored_fields.document(hits[1].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaaab", term.as_ref().as_str());

    let document = stored_fields.document(hits[2].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaabb", term.as_ref().as_str());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaaac"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      0,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(3, hits.len());

    let document = stored_fields.document(hits[0].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaaaa", term.as_ref().as_str());

    let document = stored_fields.document(hits[1].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaaab", term.as_ref().as_str());

    let document = stored_fields.document(hits[2].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaabb", term.as_ref().as_str());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaaac"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      1,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(3, hits.len());

    let document = stored_fields.document(hits[0].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaaaa", term.as_ref().as_str());

    let document = stored_fields.document(hits[1].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaaab", term.as_ref().as_str());

    let document = stored_fields.document(hits[2].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaabb", term.as_ref().as_str());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaaac"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      2,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(3, hits.len());

    let document = stored_fields.document(hits[0].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaaaa", term.as_ref().as_str());

    let document = stored_fields.document(hits[1].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaaab", term.as_ref().as_str());

    let document = stored_fields.document(hits[2].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaabb", term.as_ref().as_str());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaaac"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      3,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(3, hits.len());

    let document = stored_fields.document(hits[0].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaaaa", term.as_ref().as_str());

    let document = stored_fields.document(hits[1].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaaab", term.as_ref().as_str());

    let document = stored_fields.document(hits[2].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaabb", term.as_ref().as_str());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaaac"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      4,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(2, hits.len());

    let document = stored_fields.document(hits[0].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaaaa", term.as_ref().as_str());

    let document = stored_fields.document(hits[1].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("aaaab", term.as_ref().as_str());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "aaaac"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      5,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(0, hits.len());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "ddddX"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      0,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(1, hits.len());

    let document = stored_fields.document(hits[0].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("ddddd", term.as_ref().as_str());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "ddddX"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      1,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(1, hits.len());

    let document = stored_fields.document(hits[0].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("ddddd", term.as_ref().as_str());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "ddddX"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      2,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(1, hits.len());

    let document = stored_fields.document(hits[0].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("ddddd", term.as_ref().as_str());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "ddddX"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      3,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(1, hits.len());

    let document = stored_fields.document(hits[0].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("ddddd", term.as_ref().as_str());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "ddddX"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      4,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(1, hits.len());

    let document = stored_fields.document(hits[0].doc)?;
    let term = document
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert_eq!("ddddd", term.as_ref().as_str());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "ddddX"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      5,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(0, hits.len());

    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("anotherfield", "ddddX"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      0,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(0, hits.len());

    Ok(())
  }
  #[test]
  fn test_prefix_length_equal_string_length() -> Result<()> {
    let mut random = random();
    let directory = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = RandomIndexWriter::with_config(&mut random, directory.clone(), iwc);
    let mut field_to_type = HashMap::new();
    add_doc(&mut random, "b*a", &writer, &mut field_to_type)?;
    add_doc(&mut random, "b*ab", &writer, &mut field_to_type)?;
    add_doc(&mut random, "b*abc", &writer, &mut field_to_type)?;
    add_doc(&mut random, "b*abcd", &writer, &mut field_to_type)?;
    let multibyte = "아프리카코끼리속";
    add_doc(&mut random, multibyte, &writer, &mut field_to_type)?;
    let reader = writer.get_reader()?;
    let searcher = new_searcher_with_reader(reader)?;
    writer.close()?;
    let mut max_edits = 0;
    let mut prefix_length = 3;
    let mut query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "b*a"),
      max_edits,
      prefix_length,
    )?;
    let mut hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(1, hits.len());

    max_edits = 1;
    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "b*a"),
      max_edits,
      prefix_length,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(2, hits.len());

    max_edits = 2;
    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", "b*a"),
      max_edits,
      prefix_length,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(3, hits.len());

    max_edits = 1;
    prefix_length = multibyte.chars().count() - 1;
    let multibyte_prefix: String = multibyte.chars().take(prefix_length).collect();
    query = FuzzyQuery::with_max_edits_and_prefix(
      Term::from_text("field", multibyte_prefix),
      max_edits,
      prefix_length,
    )?;
    hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(1, hits.len());

    Ok(())
  }
  #[test]
  fn test2() -> Result<()> {
    let mut random = random();
    let directory = new_directory_shared(&mut random)?;
    // TODO IMPORTANT: Java uses MockTokenizer.KEYWORD here. Use the default MockAnalyzer for now.
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = RandomIndexWriter::with_config(&mut random, directory.clone(), iwc);
    let mut field_to_type = HashMap::new();
    add_doc(&mut random, "LANGE", &writer, &mut field_to_type)?;
    add_doc(&mut random, "LUETH", &writer, &mut field_to_type)?;
    add_doc(&mut random, "PIRSING", &writer, &mut field_to_type)?;
    add_doc(&mut random, "RIEGEL", &writer, &mut field_to_type)?;
    add_doc(&mut random, "TRZECZIAK", &writer, &mut field_to_type)?;
    add_doc(&mut random, "WALKER", &writer, &mut field_to_type)?;
    add_doc(&mut random, "WBR", &writer, &mut field_to_type)?;
    add_doc(&mut random, "WE", &writer, &mut field_to_type)?;
    add_doc(&mut random, "WEB", &writer, &mut field_to_type)?;
    add_doc(&mut random, "WEBE", &writer, &mut field_to_type)?;
    add_doc(&mut random, "WEBER", &writer, &mut field_to_type)?;
    add_doc(&mut random, "WEBERE", &writer, &mut field_to_type)?;
    add_doc(&mut random, "WEBREE", &writer, &mut field_to_type)?;
    add_doc(&mut random, "WEBEREI", &writer, &mut field_to_type)?;
    add_doc(&mut random, "WBRE", &writer, &mut field_to_type)?;
    add_doc(&mut random, "WITTKOPF", &writer, &mut field_to_type)?;
    add_doc(&mut random, "WOJNAROWSKI", &writer, &mut field_to_type)?;
    add_doc(&mut random, "WRICKE", &writer, &mut field_to_type)?;

    let reader = writer.get_reader()?;
    let searcher = new_searcher_with_reader(reader)?;
    writer.close()?;

    let query = FuzzyQuery::with_max_edits_and_prefix(Term::from_text("field", "WEBER"), 2, 1)?;
    let hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(8, hits.len());

    Ok(())
  }
  #[test]
  fn test_single_query_exact_match_scores_highest() -> Result<()> {
    let mut random = random();
    let directory = new_directory_shared(&mut random)?;
    let writer = RandomIndexWriter::new(&mut random, directory.clone());
    let mut field_to_type = HashMap::new();
    add_doc(&mut random, "smith", &writer, &mut field_to_type)?;
    add_doc(&mut random, "smith", &writer, &mut field_to_type)?;
    add_doc(&mut random, "smith", &writer, &mut field_to_type)?;
    add_doc(&mut random, "smith", &writer, &mut field_to_type)?;
    add_doc(&mut random, "smith", &writer, &mut field_to_type)?;
    add_doc(&mut random, "smith", &writer, &mut field_to_type)?;
    add_doc(&mut random, "smythe", &writer, &mut field_to_type)?;
    add_doc(&mut random, "smdssasd", &writer, &mut field_to_type)?;

    let reader = writer.get_reader()?;
    let mut searcher = new_searcher_with_reader(reader)?;
    searcher.set_similarity(classic_similarity::new());
    writer.close()?;
    let search_terms = vec!["smith", "smythe", "smdssasd"];
    let mut stored_fields = searcher.stored_fields()?;
    for search_term in search_terms {
      let query =
        FuzzyQuery::with_max_edits_and_prefix(Term::from_text("field", search_term), 2, 1)?;
      let hits = searcher.search(query, 1000)?.score_docs;
      let best_doc = stored_fields.document(hits[0].doc)?;
      assert!(!hits.is_empty());
      let top_match = best_doc
        .get_field("field")
        .unwrap()
        .string_value()?
        .unwrap();
      assert_eq!(search_term, top_match.as_ref().as_str());
      if hits.len() > 1 {
        let worst_doc = stored_fields.document(hits[hits.len() - 1].doc)?;
        let worst_match = worst_doc
          .get_field("field")
          .unwrap()
          .string_value()?
          .unwrap();
        assert_ne!(search_term, worst_match.as_ref().as_str());
      }
    }
    Ok(())
  }
  #[test]
  fn test_multiple_queries_idf_works() -> Result<()> {
    let mut random = random();
    let directory = new_directory_shared(&mut random)?;
    let writer = RandomIndexWriter::new(&mut random, directory.clone());
    let mut field_to_type = HashMap::new();

    add_doc(&mut random, "michael smith", &writer, &mut field_to_type)?;
    add_doc(&mut random, "michael lucero", &writer, &mut field_to_type)?;
    add_doc(&mut random, "doug cutting", &writer, &mut field_to_type)?;
    add_doc(&mut random, "doug cuttin", &writer, &mut field_to_type)?;
    add_doc(&mut random, "michael wardle", &writer, &mut field_to_type)?;
    add_doc(&mut random, "micheal vegas", &writer, &mut field_to_type)?;
    add_doc(&mut random, "michael lydon", &writer, &mut field_to_type)?;

    let reader = writer.get_reader()?;
    let mut searcher = new_searcher_with_reader(reader)?;
    searcher.set_similarity(classic_similarity::new());

    writer.close()?;

    let mut query = BooleanQueryBuilder::new();
    let common_search_term = "michael";
    let common_query =
      FuzzyQuery::with_max_edits_and_prefix(Term::from_text("field", common_search_term), 2, 1)?;
    query.add(common_query, Occur::Should)?;

    let rare_search_term = "cutting";
    let rare_query =
      FuzzyQuery::with_max_edits_and_prefix(Term::from_text("field", rare_search_term), 2, 1)?;
    query.add(rare_query, Occur::Should)?;
    let hits = searcher.search(query.build(), 1000)?.score_docs;

    assert_eq!(7, hits.len());
    let best_doc = searcher.stored_fields()?.document(hits[0].doc)?;
    let top_match = best_doc
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert!(top_match.as_ref().as_str().contains(rare_search_term));

    let runner_up_doc = searcher.stored_fields()?.document(hits[1].doc)?;
    let runner_up_match = runner_up_doc
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert!(runner_up_match.as_ref().as_str().contains("cuttin"));

    let worst_doc = searcher
      .stored_fields()?
      .document(hits[hits.len() - 1].doc)?;
    let worst_match = worst_doc
      .get_field("field")
      .unwrap()
      .string_value()?
      .unwrap();
    assert!(worst_match.as_ref().as_str().contains("micheal"));

    Ok(())
  }
  #[test]
  fn test_tie_breaker() -> Result<()> {
    let mut random = random();
    let directory = new_directory_shared(&mut random)?;
    let writer = RandomIndexWriter::new(&mut random, directory.clone());
    let mut field_to_type = HashMap::new();
    add_doc(&mut random, "a123456", &writer, &mut field_to_type)?;
    add_doc(&mut random, "c123456", &writer, &mut field_to_type)?;
    add_doc(&mut random, "d123456", &writer, &mut field_to_type)?;
    add_doc(&mut random, "e123456", &writer, &mut field_to_type)?;

    let directory2 = new_directory_shared(&mut random)?;
    let writer2 = RandomIndexWriter::new(&mut random, directory2.clone());
    let mut field_to_type2 = HashMap::new();
    add_doc(&mut random, "a123456", &writer2, &mut field_to_type2)?;
    add_doc(&mut random, "b123456", &writer2, &mut field_to_type2)?;
    add_doc(&mut random, "b123456", &writer2, &mut field_to_type2)?;
    add_doc(&mut random, "b123456", &writer2, &mut field_to_type2)?;
    add_doc(&mut random, "c123456", &writer2, &mut field_to_type2)?;
    add_doc(&mut random, "f123456", &writer2, &mut field_to_type2)?;

    let ir1 = writer.get_reader()?;
    let ir2 = writer2.get_reader()?;

    let mr = MultiReader::with_composite_reader(vec![ir1, ir2])?;
    let searcher = new_searcher_with_reader(mr)?;
    let fq = FuzzyQuery::with_options(Term::from_text("field", "z123456"), 1, 0, 2, false)?;
    let docs = searcher.search(fq, 2)?;
    assert_eq!(5, docs.total_hits.value());
    writer.close()?;
    writer2.close()?;

    Ok(())
  }
  /// Test the TopTermsBoostOnlyBooleanQueryRewrite rewrite method.
  #[test]
  fn test_boost_only_rewrite() -> Result<()> {
    let mut random = random();
    let directory = new_directory_shared(&mut random)?;
    let writer = RandomIndexWriter::new(&mut random, directory.clone());
    let mut field_to_type = HashMap::new();
    add_doc(&mut random, "Lucene", &writer, &mut field_to_type)?;
    add_doc(&mut random, "Lucene", &writer, &mut field_to_type)?;
    add_doc(&mut random, "Lucenne", &writer, &mut field_to_type)?;

    let reader = writer.get_reader()?;
    let searcher = new_searcher_with_reader(reader)?;
    writer.close()?;

    let query = FuzzyQuery::with_rewrite(
      Term::from_text("field", "lucene"),
      FuzzyQuery::DEFAULT_MAX_EDITS,
      FuzzyQuery::DEFAULT_PREFIX_LENGTH,
      FuzzyQuery::DEFAULT_MAX_EXPANSIONS,
      FuzzyQuery::DEFAULT_TRANSPOSITIONS,
      TopTermsBoostOnlyBooleanQueryRewrite::new(50),
    )?;
    let hits = searcher.search(query, 1000)?.score_docs;
    assert_eq!(3, hits.len());
    assert_eq!(
      "Lucene",
      searcher
        .stored_fields()?
        .document(hits[0].doc)?
        .get_field("field")
        .unwrap()
        .string_value()?
        .unwrap()
        .as_ref()
        .as_str()
    );
    assert_eq!(
      "Lucene",
      searcher
        .stored_fields()?
        .document(hits[1].doc)?
        .get_field("field")
        .unwrap()
        .string_value()?
        .unwrap()
        .as_ref()
        .as_str()
    );
    assert_eq!(
      "Lucenne",
      searcher
        .stored_fields()?
        .document(hits[2].doc)?
        .get_field("field")
        .unwrap()
        .string_value()?
        .unwrap()
        .as_ref()
        .as_str()
    );

    Ok(())
  }
  #[test]
  fn test_giga() -> Result<()> {
    let mut random = random();
    let index = new_directory_shared(&mut random)?;
    let w = RandomIndexWriter::new(&mut random, index.clone());
    let mut field_to_type = HashMap::new();

    add_doc(&mut random, "Lucene in Action", &w, &mut field_to_type)?;
    add_doc(&mut random, "Lucene for Dummies", &w, &mut field_to_type)?;

    add_doc(&mut random, "Giga byte", &w, &mut field_to_type)?;

    add_doc(
      &mut random,
      "ManagingGigabytesManagingGigabyte",
      &w,
      &mut field_to_type,
    )?;
    add_doc(
      &mut random,
      "ManagingGigabytesManagingGigabytes",
      &w,
      &mut field_to_type,
    )?;

    add_doc(
      &mut random,
      "The Art of Computer Science",
      &w,
      &mut field_to_type,
    )?;
    add_doc(&mut random, "J. K. Rowling", &w, &mut field_to_type)?;
    add_doc(&mut random, "JK Rowling", &w, &mut field_to_type)?;
    add_doc(&mut random, "Joanne K Roling", &w, &mut field_to_type)?;
    add_doc(&mut random, "Bruce Willis", &w, &mut field_to_type)?;
    add_doc(&mut random, "Willis bruce", &w, &mut field_to_type)?;
    add_doc(&mut random, "Brute willis", &w, &mut field_to_type)?;
    add_doc(&mut random, "B. willis", &w, &mut field_to_type)?;
    let r = w.get_reader()?;

    let q = FuzzyQuery::with_max_edits(Term::from_text("field", "giga"), 0)?;

    let searcher = new_searcher_with_reader(r)?;
    let hits = searcher.search(q, 10)?.score_docs;
    assert_eq!(1, hits.len());
    assert_eq!(
      "Giga byte",
      searcher
        .stored_fields()?
        .document(hits[0].doc)?
        .get_field("field")
        .unwrap()
        .string_value()?
        .unwrap()
        .as_ref()
        .as_str()
    );
    w.close()?;

    Ok(())
  }
  #[test]
  fn test_distance_as_edits_searching() -> Result<()> {
    let mut random = random();
    let index = new_directory_shared(&mut random)?;
    let w = RandomIndexWriter::new(&mut random, index.clone());
    let mut field_to_type = HashMap::new();
    add_doc(&mut random, "foobar", &w, &mut field_to_type)?;
    add_doc(&mut random, "test", &w, &mut field_to_type)?;
    add_doc(&mut random, "working", &w, &mut field_to_type)?;
    let reader = w.get_reader()?;
    let searcher = new_searcher_with_reader(reader)?;
    w.close()?;

    let mut q = FuzzyQuery::with_max_edits(Term::from_text("field", "fouba"), 2)?;
    let mut hits = searcher.search(q, 10)?.score_docs;
    assert_eq!(1, hits.len());
    assert_eq!(
      "foobar",
      searcher
        .stored_fields()?
        .document(hits[0].doc)?
        .get_field("field")
        .unwrap()
        .string_value()?
        .unwrap()
        .as_ref()
        .as_str()
    );

    q = FuzzyQuery::with_max_edits(Term::from_text("field", "foubara"), 2)?;
    hits = searcher.search(q, 10)?.score_docs;
    assert_eq!(1, hits.len());
    assert_eq!(
      "foobar",
      searcher
        .stored_fields()?
        .document(hits[0].doc)?
        .get_field("field")
        .unwrap()
        .string_value()?
        .unwrap()
        .as_ref()
        .as_str()
    );

    let expected = FuzzyQuery::with_max_edits(Term::from_text("field", "t"), 3).unwrap_err();
    assert!(format!("{expected}").contains("maxEdits"));

    Ok(())
  }
  #[test]
  fn test_validation() {
    let expected =
      FuzzyQuery::with_options(Term::from_text("field", "foo"), -1, 0, 1, false).unwrap_err();
    assert!(format!("{expected}").contains("maxEdits"));

    let expected = FuzzyQuery::with_options(
      Term::from_text("field", "foo"),
      LevenshteinAutomata::MAXIMUM_SUPPORTED_DISTANCE + 1,
      0,
      1,
      false,
    )
    .unwrap_err();
    assert!(format!("{expected}").contains("maxEdits must be between"));

    let expected =
      FuzzyQuery::with_options(Term::from_text("field", "foo"), 1, 0, 0, false).unwrap_err();
    assert!(format!("{expected}").contains("maxExpansions must be positive"));

    let expected =
      FuzzyQuery::with_options(Term::from_text("field", "foo"), 1, 0, 0, false).unwrap_err();
    assert!(format!("{expected}").contains("maxExpansions must be positive"));
  }
  fn add_doc<R>(
    random: &mut R,
    text: &str,
    writer: &RandomIndexWriter<DirEnum>,
    field_to_type: &mut HashMap<String, FieldType>,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      "field",
      text,
      Store::Yes,
      field_to_type,
    )?);
    writer.add_document(doc)?;

    Ok(())
  }
  fn random_simple_string<R>(random: &mut R, digits: i32) -> String
  where
    R: Rng + ?Sized,
  {
    let term_length = TestUtil::next_int(random, 1, 8);
    let mut chars = Vec::with_capacity(term_length as usize);

    for _ in 0..term_length {
      let ch = (b'a' + random.random_range(0..digits) as u8) as char;
      chars.push(ch);
    }

    chars.into_iter().collect()
  }
  #[test]
  fn test_random() -> Result<()> {
    let mut random = random();
    let digits = TestUtil::next_int(&mut random, 2, 3);
    let vocabulary_size = digits << 7;
    let num_terms = std::cmp::min(at_least(&mut random, 100), vocabulary_size);
    let mut terms = HashSet::new();
    while terms.len() < num_terms as usize {
      terms.insert(random_simple_string(&mut random, digits));
    }

    let dir = new_directory_shared(&mut random)?;
    let w = RandomIndexWriter::new(&mut random, dir.clone());
    let mut field_to_type = HashMap::new();
    for term in &terms {
      let mut doc = Document::new();
      doc.add(new_string_field(
        &mut random,
        "field",
        term.as_str(),
        Store::Yes,
        &mut field_to_type,
      )?);
      w.add_document(doc)?;
    }
    let r = w.get_reader()?;
    w.close()?;
    let s = new_searcher_with_reader(r)?;
    let iters = at_least(&mut random, 200);
    for _iter in 0..iters {
      let query_term = random_simple_string(&mut random, digits);
      let prefix_length = random.random_range(0..query_term.len());
      let query_prefix = &query_term[0..prefix_length];

      let mut expected: Vec<Vec<TermAndScore>> = Vec::with_capacity(3);
      for _ed in 0..3 {
        expected.push(Vec::new());
      }
      for term in &terms {
        if !term.starts_with(query_prefix) {
          continue;
        }
        let mut ed = get_distance(term, &query_term);
        let score = 1.0 - ed as f32 / std::cmp::min(query_term.len(), term.len()) as f32;
        while ed < 3 {
          expected[ed as usize].push(TermAndScore::new(term.clone(), score));
          ed += 1;
        }
      }
      #[allow(clippy::needless_range_loop)]
      for ed in 0..3 {
        expected[ed].sort();
        let queue_size = TestUtil::next_int(&mut random, 1, terms.len() as i32) as usize;
        let query = FuzzyQuery::with_options(
          Term::from_text("field", query_term.as_str()),
          ed as i32,
          prefix_length,
          queue_size,
          true,
        )?;
        let hits = s.search(query, terms.len())?;
        let mut actual = HashSet::new();
        let mut stored_fields = s.stored_fields()?;
        for hit in hits.score_docs {
          let doc = stored_fields.document(hit.doc)?;
          actual.insert(
            doc
              .get_field("field")
              .unwrap()
              .string_value()?
              .unwrap()
              .as_ref()
              .as_str()
              .to_string(),
          );
        }
        let mut expected_top = HashSet::new();
        let limit = std::cmp::min(queue_size, expected[ed].len());
        #[allow(clippy::needless_range_loop)]
        for i in 0..limit {
          expected_top.insert(expected[ed][i].term.clone());
        }

        if actual != expected_top {
          let mut sb = String::new();
          sb.push_str(&format!(
            "FAILED: query={} ed={} queueSize={} vs expected match size={} prefixLength={}\n",
            query_term,
            ed,
            queue_size,
            expected[ed].len(),
            prefix_length
          ));

          let mut first = true;
          for term in &actual {
            if !expected_top.contains(term) {
              if first {
                sb.push_str("  these matched but shouldn't:\n");
                first = false;
              }
              sb.push_str(&format!("    {term}\n"));
            }
          }
          first = true;
          for term in &expected_top {
            if !actual.contains(term) {
              if first {
                sb.push_str("  these did not match but should:\n");
                first = false;
              }
              sb.push_str(&format!("    {term}\n"));
            }
          }
          panic!("{sb}");
        }
      }
    }

    Ok(())
  }

  #[derive(Debug, Clone)]
  struct TermAndScore {
    term: String,
    score: f32,
  }

  impl TermAndScore {
    fn new(term: String, score: f32) -> Self {
      Self { term, score }
    }
  }

  impl Eq for TermAndScore {}

  impl PartialEq for TermAndScore {
    fn eq(&self, other: &Self) -> bool {
      self.term == other.term && self.score == other.score
    }
  }

  impl Ord for TermAndScore {
    fn cmp(&self, other: &Self) -> Ordering {
      other
        .score
        .partial_cmp(&self.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| self.term.cmp(&other.term))
    }
  }

  impl PartialOrd for TermAndScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
      Some(self.cmp(other))
    }
  }

  fn get_distance(target: &str, other: &str) -> i32 {
    let target_points = to_ints_ref(target);
    let other_points = to_ints_ref(other);
    let n = target_points.len();
    let m = other_points.len();
    let mut d = vec![vec![0; m + 1]; n + 1];

    if n == 0 || m == 0 {
      if n == m {
        return 0;
      } else {
        return std::cmp::max(n, m) as i32;
      }
    }

    for (i, row) in d.iter_mut().enumerate().take(n + 1) {
      row[0] = i as i32;
    }
    #[allow(clippy::needless_range_loop)]
    for j in 0..=m {
      d[0][j] = j as i32;
    }

    for j in 1..=m {
      let t_j = other_points[j - 1];

      for i in 1..=n {
        let cost = if target_points[i - 1] == t_j { 0 } else { 1 };
        d[i][j] = std::cmp::min(
          std::cmp::min(d[i - 1][j] + 1, d[i][j - 1] + 1),
          d[i - 1][j - 1] + cost,
        );
        if i > 1
          && j > 1
          && target_points[i - 1] == other_points[j - 2]
          && target_points[i - 2] == other_points[j - 1]
        {
          d[i][j] = std::cmp::min(d[i][j], d[i - 2][j - 2] + cost);
        }
      }
    }

    d[n][m]
  }

  fn to_ints_ref(s: &str) -> Vec<i32> {
    let mut ref_ = Vec::with_capacity(s.len());
    for cp in s.chars() {
      ref_.push(cp as i32);
    }
    ref_
  }
  #[test]
  fn test_visitor() -> Result<()> {
    // TODO IMPORTANT query visitor 未实现
    Ok(())
  }
}

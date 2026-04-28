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
use crate::core::index::terms::{Terms, TermsPosting};
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::automaton_query::AutomatonQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query_constant_score_blended_wrapper::MultiTermQueryConstantScoreBlendedWrapper;
use crate::core::search::multi_term_query_constant_score_wrapper::MultiTermQueryConstantScoreWrapper;
use crate::core::search::prefix_query::PrefixQuery;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::regexp_query::RegexpQuery;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::term_range_query::TermRangeQuery;
use crate::core::search::wildcard_query::WildcardQuery;
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::impl_from_for_enum;
#[cfg(test)]
use crate::test::core::search::test_prefix_random::DumbPrefixQuery;
#[cfg(test)]
use crate::test::core::search::test_regexp_random2::DumbRegexpQuery;
use std::fmt::Debug;

pub trait MultiTermQuery: QueryBase + Clone {
  fn get_field(&self) -> &str;
  type TermsEnum<T>: TermsEnum<PostingsEnum = TermsPosting<T>>
  where
    T: Terms;
  fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
  where
    T: Terms + Clone;
  fn get_terms_count(&self) -> i64 {
    -1
  }

  fn as_query(&self) -> Query;
}
pub trait RewriteMethod {
  fn rewrite<IRC, Q>(self, index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQueryEnum>,
    IRC: IndexReaderContext;
  fn get_terms_enum<M, T>(&self, query: &M, terms: T) -> Result<<M as MultiTermQuery>::TermsEnum<T>>
  where
    M: MultiTermQuery,
    T: Terms + Clone,
  {
    query.get_terms_enum(terms)
  }
}
/// A rewrite method where documents are assigned a constant score equal to the query's boost.
/// Maintains a boolean query-like implementation over the most costly terms while pre-processing
/// the less costly terms into a filter bitset. Enforces an upper-limit on the number of terms
/// allowed in the boolean query-like implementation.
///
/// This method aims to balance the benefits of both [`ConstantScoreRewrite`] and
/// [`ConstantScoreRewrite`] by enabling skipping and early termination over costly terms
/// while limiting the overhead of a BooleanQuery with many terms. It also ensures you cannot hit
/// `IndexSearcher.TooManyClauses`. For some use-cases with all low
/// cost terms, [`ConstantScoreRewrite`] may be more performant. While for some use-cases
/// with all high cost terms, [`ConstantScoreBooleanRewrite`] may be better.
#[derive(Default, Clone)]
pub struct ConstantScoreBlendedRewrite;
impl RewriteMethod for ConstantScoreBlendedRewrite {
  fn rewrite<IRC, Q>(self, _index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQueryEnum>,
    IRC: IndexReaderContext,
  {
    Ok(MultiTermQueryConstantScoreBlendedWrapper::new(query).into())
  }
}
/// A rewrite method that first creates a private Filter, by visiting each term in sequence and
/// marking all docs for that term. Matching documents are assigned a constant score equal to the
/// query's boost.
///
/// This method is faster than the BooleanQuery rewrite methods when the number of matched terms
/// or matched documents is non-trivial. Also, it will never hit an errant `IndexSearcher.TooManyClauses`
/// exception.
#[derive(Default, Clone)]
pub struct ConstantScoreRewrite;
impl RewriteMethod for ConstantScoreRewrite {
  fn rewrite<IRC, Q>(self, _index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQueryEnum>,
    IRC: IndexReaderContext,
  {
    Ok(MultiTermQueryConstantScoreWrapper::new(query).into())
  }
}
#[derive(Clone)]
pub enum RewriteMethodEnum {
  Standard(ConstantScoreRewrite),
  Blended(ConstantScoreBlendedRewrite),
}
impl RewriteMethod for RewriteMethodEnum {
  fn rewrite<IRC, Q>(self, index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQueryEnum>,
    IRC: IndexReaderContext,
  {
    match self {
      RewriteMethodEnum::Standard(r) => r.rewrite(index_searcher, query),
      RewriteMethodEnum::Blended(r) => r.rewrite(index_searcher, query),
    }
  }
}
impl_from_for_enum!(
    RewriteMethodEnum,
    ConstantScoreRewrite => Standard,
    ConstantScoreBlendedRewrite => Blended,
);

macro_rules! dispatch_multi_term_query {
  ($self:expr, |$inner:ident| $body:expr) => {{
    match $self {
      MultiTermQueryEnum::Prefix($inner) => $body,
      MultiTermQueryEnum::TermRange($inner) => $body,
      MultiTermQueryEnum::Automaton($inner) => $body,
      MultiTermQueryEnum::Wildcard($inner) => $body,
      MultiTermQueryEnum::Regexp($inner) => $body,
      #[cfg(test)]
      MultiTermQueryEnum::DumbPrefix($inner) => $body,
      #[cfg(test)]
      MultiTermQueryEnum::DumbRegexp($inner) => $body,
    }
  }};
}
use crate::core::index::term::Term;
use crate::core::index::term_states::TermStates;
use crate::core::search::blended_term_query::BooleanRewrite;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::term_collecting_rewrite::TermCollectingRewrite;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_terms_rewrite::TopTermsRewrite;
use crate::core::search::{blended_term_query, boolean_query, index_searcher};
pub(crate) use dispatch_multi_term_query;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MultiTermQueryEnum {
  Prefix(PrefixQuery),
  TermRange(TermRangeQuery),
  Automaton(AutomatonQuery),
  Wildcard(WildcardQuery),
  Regexp(RegexpQuery),
  #[cfg(test)]
  DumbPrefix(DumbPrefixQuery),
  #[cfg(test)]
  DumbRegexp(DumbRegexpQuery),
}
#[cfg(debug_assertions)]
impl From<MultiTermQueryEnum> for Query {
  fn from(value: MultiTermQueryEnum) -> Self {
    match value {
      MultiTermQueryEnum::Prefix(q) => Query::Prefix(q),
      MultiTermQueryEnum::TermRange(q) => Query::TermRange(q),
      MultiTermQueryEnum::Automaton(q) => Query::Automaton(q),
      MultiTermQueryEnum::Wildcard(q) => Query::Wildcard(q),
      MultiTermQueryEnum::Regexp(q) => Query::Regexp(q),
      #[cfg(test)]
      MultiTermQueryEnum::DumbPrefix(q) => Query::DumbPrefix(q),
      #[cfg(test)]
      MultiTermQueryEnum::DumbRegexp(q) => Query::DumbRegexp(q),
    }
  }
}

#[cfg(debug_assertions)]
impl MultiTermQueryEnum {
  pub fn from_query(query: &Query) -> Option<Self> {
    match query {
      Query::Prefix(q) => Some(Self::Prefix(q.clone())),
      Query::TermRange(q) => Some(Self::TermRange(q.clone())),
      Query::Automaton(q) => Some(Self::Automaton(q.clone())),
      Query::Wildcard(q) => Some(Self::Wildcard(q.clone())),
      Query::Regexp(q) => Some(Self::Regexp(q.clone())),
      #[cfg(test)]
      Query::DumbPrefix(q) => Some(Self::DumbPrefix(q.clone())),
      #[cfg(test)]
      Query::DumbRegexp(q) => Some(Self::DumbRegexp(q.clone())),
      _ => None,
    }
  }
}
#[cfg(debug_assertions)]
impl Query {
  pub fn is_multi_term_query(&self) -> bool {
    MultiTermQueryEnum::from_query(self).is_some()
  }
}

impl QueryBase for MultiTermQueryEnum {
  fn as_string(&self, field: &str) -> Result<String> {
    dispatch_multi_term_query!(self, |q| q.as_string(field))
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

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Err(LuceneError::unsupported_operation(""))
  }

  fn visit<QV>(&self, visitor: &QV)
  where
    QV: QueryVisitor,
  {
    dispatch_multi_term_query!(self, |q| q.visit(visitor))
  }
}

impl HasIdentity for MultiTermQueryEnum {
  fn identity(&self) -> &Identity {
    dispatch_multi_term_query!(self, |q| q.identity())
  }
}

impl_from_for_enum!(
    MultiTermQueryEnum,
    PrefixQuery => Prefix,
    TermRangeQuery => TermRange,
    AutomatonQuery => Automaton,
    WildcardQuery => Wildcard,
    RegexpQuery => Regexp,
);
#[cfg(test)]
impl_from_for_enum!(
    MultiTermQueryEnum,
    DumbPrefixQuery => DumbPrefix,
    DumbRegexpQuery => DumbRegexp,
);
/// A rewrite method that first translates each term into [`Occur::Should`] clause
/// in a [`BooleanQuery`], and keeps the scores as computed by the query.
///
/// This rewrite method only uses the top scoring terms so it will not overflow the boolean max
/// clause count.
pub struct TopTermsScoringBooleanQueryRewrite {
  size: usize,
}

impl TopTermsScoringBooleanQueryRewrite {
  /// Create a [`TopTermsScoringBooleanQueryRewrite`] for at most `size` terms.
  ///
  /// NOTE: if [`IndexSearcher::get_max_clause_count`] is smaller than `size`, then
  /// it will be used instead.
  pub fn new(size: usize) -> Self {
    Self { size }
  }
}

impl TermCollectingRewrite for TopTermsScoringBooleanQueryRewrite {
  type B = boolean_query::Builder;

  fn get_top_level_builder(&self) -> Result<Self::B> {
    Ok(boolean_query::Builder::new())
  }

  fn build(&self, builder: Self::B) -> Result<Query> {
    Ok(builder.build().into())
  }

  fn add_clause_with_states(
    &self,
    top_level: &mut Self::B,
    term: Term,
    _doc_count: i32,
    boost: f32,
    states: Option<TermStates>,
  ) -> Result<()> {
    let tq = TermQuery::with_term_state(term, states);
    top_level.add(BoostQuery::new(tq, boost)?, Occur::Should)?;
    Ok(())
  }
}

impl RewriteMethod for TopTermsScoringBooleanQueryRewrite {
  fn rewrite<IRC, Q>(self, index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQueryEnum>,
    IRC: IndexReaderContext,
  {
    self.default_rewrite(index_searcher, &query)
  }
}

impl TopTermsRewrite for TopTermsScoringBooleanQueryRewrite {
  fn get_size(&self) -> usize {
    self.size
  }

  fn get_max_size(&self) -> usize {
    index_searcher::get_max_clause_count()
  }
}
/// A rewrite method that first translates each term into [`Occur::Should`] clause
/// in a [`BooleanQuery`], but adjusts the frequencies used for scoring to be blended across the
/// terms, otherwise the rarest term typically ranks highest (often not useful eg in the set of
/// expanded terms in a [`FuzzyQuery`]).
///
/// This rewrite method only uses the top scoring terms so it will not overflow the boolean max
/// clause count.
pub struct TopTermsBlendedFreqScoringRewrite {
  size: usize,
}

impl TopTermsBlendedFreqScoringRewrite {
  /// Create a [`TopTermsBlendedFreqScoringRewrite`] for at most `size` terms.
  ///
  /// NOTE: if [`IndexSearcher::get_max_clause_count`] is smaller than `size`, then
  /// it will be used instead.
  pub fn new(size: usize) -> Self {
    Self { size }
  }
}

impl TermCollectingRewrite for TopTermsBlendedFreqScoringRewrite {
  type B = blended_term_query::Builder;

  fn get_top_level_builder(&self) -> Result<Self::B> {
    let mut builder = blended_term_query::Builder::new();
    builder.set_rewrite_method(BooleanRewrite);
    Ok(builder)
  }

  fn build(&self, builder: Self::B) -> Result<Query> {
    Ok(builder.build()?.into())
  }

  fn add_clause_with_states(
    &self,
    top_level: &mut Self::B,
    term: Term,
    _doc_count: i32,
    boost: f32,
    states: Option<TermStates>,
  ) -> Result<()> {
    top_level.add_with_term_states(term, boost, states)?;
    Ok(())
  }
}

impl RewriteMethod for TopTermsBlendedFreqScoringRewrite {
  fn rewrite<IRC, Q>(self, index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQueryEnum>,
    IRC: IndexReaderContext,
  {
    self.default_rewrite(index_searcher, &query)
  }
}

impl TopTermsRewrite for TopTermsBlendedFreqScoringRewrite {
  fn get_size(&self) -> usize {
    self.size
  }

  fn get_max_size(&self) -> usize {
    index_searcher::get_max_clause_count()
  }
}

/// A rewrite method that first translates each term into [`Occur::Should`] clause
/// in a [`BooleanQuery`], and keeps the scores as computed by the query.
///
/// This rewrite method only uses the top scoring terms so it will not overflow the boolean max
/// clause count.
pub struct TopTermsBoostOnlyBooleanQueryRewrite {
  size: usize,
}
impl TopTermsBoostOnlyBooleanQueryRewrite {
  /// Create a [`TopTermsScoringBooleanQueryRewrite`] for at most `size` terms.
  ///
  /// NOTE: if [`IndexSearcher::get_max_clause_count`] is smaller than `size`, then
  /// it will be used instead.
  pub fn new(size: usize) -> Self {
    Self { size }
  }
}

impl TermCollectingRewrite for TopTermsBoostOnlyBooleanQueryRewrite {
  type B = boolean_query::Builder;

  fn get_top_level_builder(&self) -> Result<Self::B> {
    Ok(boolean_query::Builder::new())
  }

  fn build(&self, builder: Self::B) -> Result<Query> {
    Ok(builder.build().into())
  }

  fn add_clause_with_states(
    &self,
    top_level: &mut Self::B,
    term: Term,
    _doc_count: i32,
    boost: f32,
    states: Option<TermStates>,
  ) -> Result<()> {
    let tq = TermQuery::with_term_state(term, states);
    top_level.add(BoostQuery::new(tq, boost)?, Occur::Should)?;
    Ok(())
  }
}

impl RewriteMethod for TopTermsBoostOnlyBooleanQueryRewrite {
  fn rewrite<IRC, Q>(self, index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQueryEnum>,
    IRC: IndexReaderContext,
  {
    self.default_rewrite(index_searcher, &query)
  }
}

impl TopTermsRewrite for TopTermsBoostOnlyBooleanQueryRewrite {
  fn get_size(&self) -> usize {
    self.size
  }

  fn get_max_size(&self) -> usize {
    index_searcher::get_max_clause_count()
  }
}

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
use crate::core::search::fuzzy_query::FuzzyQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query_constant_score_blended_wrapper::MultiTermQueryConstantScoreBlendedWrapper;
use crate::core::search::multi_term_query_constant_score_wrapper::MultiTermQueryConstantScoreWrapper;
use crate::core::search::prefix_query::PrefixQuery;
use crate::core::search::query::{IntoQuery, Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::regexp_query::RegexpQuery;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::term_in_set_query::TermInSetQuery;
use crate::core::search::term_range_query::TermRangeQuery;
use crate::core::search::wildcard_query::WildcardQuery;
use crate::core::util::HasIdentity;
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::Result;
use crate::impl_from_for_enum;
#[cfg(test)]
use crate::test::support::core::search::multi_term::BoostCheckingQuery;
#[cfg(test)]
use crate::test::support::core::search::multi_term::DumbPrefixQuery;
#[cfg(test)]
use crate::test::support::core::search::multi_term::DumbRegexpQuery;
use std::fmt::Debug;
/// An abstract [`Query`] that matches documents containing a subset of terms provided by a
/// `FilteredTermsEnum` enumeration.
///
/// This query cannot be used directly; implement the trait and define
/// `MultiTermQuery::get_terms_enum` to provide a `FilteredTermsEnum` that iterates
/// through the terms to be matched.
///
/// **NOTE**: if `RewriteMethod` is either `MultiTermQuery::CONSTANT_SCORE_BOOLEAN_REWRITE` or
/// `MultiTermQuery::SCORING_BOOLEAN_REWRITE`, you may encounter a
/// `IndexSearcherError::TooManyClauses` error during searching, which happens when the number of
/// terms to be searched exceeds `IndexSearcher::get_max_clause_count`. Setting `RewriteMethod`
/// to `MultiTermQuery::CONSTANT_SCORE_BLENDED_REWRITE` or
/// `MultiTermQuery::CONSTANT_SCORE_REWRITE` prevents this.
///
/// The recommended rewrite method is `MultiTermQuery::CONSTANT_SCORE_BLENDED_REWRITE`: it doesn't
/// spend CPU computing unhelpful scores, and is the most performant rewrite method given the query.
/// If you need scoring (like [`FuzzyQuery`], use [`TopTermsScoringBooleanQueryRewrite`] which uses
/// a priority queue to only collect competitive terms and not hit this limitation.
///
/// Note that org.apache.lucene.queryparser.classic.QueryParser produces MultiTermQueries using
/// `MultiTermQuery::CONSTANT_SCORE_REWRITE` by default.
pub trait MultiTermQuery: QueryBase + Clone {
  /// Returns the field name for this query
  fn get_field(&self) -> &str;
  type TermsEnum<T>: TermsEnum<PostingsEnum = TermsPosting<T>>
  where
    T: Terms;
  /// Construct the enumeration to be used, expanding the pattern term. This method should only be
  /// called if the field exists (ie, implementations can assume the field does exist). This method
  /// should not return `None` (should instead return `TermsEnum::EMPTY` if no terms match). The
  /// [`TermsEnum`] must already be positioned to the first matching term. The given
  /// `AttributeSource` is passed by the `RewriteMethod` to share information between segments,
  /// for example `TopTermsRewrite` uses it to share maximum competitive boosts.
  fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
  where
    T: Terms + Clone;
  /// Return the number of unique terms contained in this query, if known up-front. If not known, -1 will be returned.
  fn get_terms_count(&self) -> i64 {
    -1
  }

  fn to_query(&self) -> Query;
}
/// Trait defining how the query is rewritten.
pub trait RewriteMethod {
  fn rewrite<IRC, Q>(self, index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQuerySet>,
    IRC: IndexReaderContext;
  /// Returns the [`MultiTermQuery`]s [`TermsEnum`].
  ///
  /// See [`MultiTermQuery::get_terms_enum`].
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
/// with all high cost terms, `ConstantScoreBooleanRewrite` may be better.
#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstantScoreBlendedRewrite;
impl RewriteMethod for ConstantScoreBlendedRewrite {
  fn rewrite<IRC, Q>(self, _index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQuerySet>,
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
/// error.
#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstantScoreRewrite;
impl RewriteMethod for ConstantScoreRewrite {
  fn rewrite<IRC, Q>(self, _index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQuerySet>,
    IRC: IndexReaderContext,
  {
    Ok(MultiTermQueryConstantScoreWrapper::new(query).into())
  }
}

pub const DOC_VALUES_REWRITE: DocValuesRewriteMethod = DocValuesRewriteMethod;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RewriteMethodEnum {
  Blended(ConstantScoreBlendedRewrite),
  ConstantScoreBoolean(ConstantScoreBooleanRewrite),
  DocValues(DocValuesRewriteMethod),
  ScoringBoolean(ScoringBooleanRewrite),
  Standard(ConstantScoreRewrite),
  TopTermsBlendedFreqScoring(TopTermsBlendedFreqScoringRewrite),
  TopTermsBoostOnlyBoolean(TopTermsBoostOnlyBooleanQueryRewrite),
  TopTermsScoringBoolean(TopTermsScoringBooleanQueryRewrite),
}
impl RewriteMethod for RewriteMethodEnum {
  fn rewrite<IRC, Q>(self, index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQuerySet>,
    IRC: IndexReaderContext,
  {
    match self {
      RewriteMethodEnum::Blended(r) => r.rewrite(index_searcher, query),
      RewriteMethodEnum::ConstantScoreBoolean(r) => r.rewrite(index_searcher, query),
      RewriteMethodEnum::DocValues(r) => r.rewrite(index_searcher, query),
      RewriteMethodEnum::ScoringBoolean(r) => r.rewrite(index_searcher, query),
      RewriteMethodEnum::Standard(r) => r.rewrite(index_searcher, query),
      RewriteMethodEnum::TopTermsBlendedFreqScoring(r) => r.rewrite(index_searcher, query),
      RewriteMethodEnum::TopTermsBoostOnlyBoolean(r) => r.rewrite(index_searcher, query),
      RewriteMethodEnum::TopTermsScoringBoolean(r) => r.rewrite(index_searcher, query),
    }
  }
}
/// A rewrite method that first translates each term into [`Occur::Should`] clause
/// in a `BooleanQuery`, and keeps the scores as computed by the query. Note that typically such
/// scores are meaningless to the user, and require non-trivial CPU to compute, so it's almost
/// always better to use `MultiTermQuery::CONSTANT_SCORE_REWRITE` instead.
///
/// **NOTE**: This rewrite method will hit `IndexSearcherError::TooManyClauses` if the number
/// of terms exceeds `IndexSearcher::get_max_clause_count`.
pub const SCORING_BOOLEAN_REWRITE: ScoringBooleanRewrite = ScoringBooleanRewrite;
/// Like `Self::SCORING_BOOLEAN_REWRITE` except scores are not computed. Instead, each matching
/// document receives a constant score equal to the query's boost.
///
/// **NOTE**: This rewrite method will hit `IndexSearcherError::TooManyClauses` if the number
/// of terms exceeds `IndexSearcher::get_max_clause_count`.
pub const CONSTANT_SCORE_BOOLEAN_REWRITE: ConstantScoreBooleanRewrite = ConstantScoreBooleanRewrite;
impl_from_for_enum!(
    RewriteMethodEnum,
    ConstantScoreBlendedRewrite => Blended,
    ConstantScoreBooleanRewrite => ConstantScoreBoolean,
    DocValuesRewriteMethod => DocValues,
    ScoringBooleanRewrite => ScoringBoolean,
    ConstantScoreRewrite => Standard,
    TopTermsBlendedFreqScoringRewrite => TopTermsBlendedFreqScoring,
    TopTermsBoostOnlyBooleanQueryRewrite => TopTermsBoostOnlyBoolean,
    TopTermsScoringBooleanQueryRewrite => TopTermsScoringBoolean,
);

macro_rules! dispatch_multi_term_query {
  ($self:expr, |$inner:ident| $body:expr) => {{
    match $self {
      MultiTermQuerySet::Prefix($inner) => $body,
      MultiTermQuerySet::TermRange($inner) => $body,
      MultiTermQuerySet::Automaton($inner) => $body,
      MultiTermQuerySet::Fuzzy($inner) => $body,
      MultiTermQuerySet::TermInSet($inner) => $body,
      MultiTermQuerySet::Wildcard($inner) => $body,
      MultiTermQuerySet::Regexp($inner) => $body,
      #[cfg(test)]
      MultiTermQuerySet::BoostChecking($inner) => $body,
      #[cfg(test)]
      MultiTermQuerySet::DumbPrefix($inner) => $body,
      #[cfg(test)]
      MultiTermQuerySet::DumbRegexp($inner) => $body,
    }
  }};
}

/// A rewrite method that first translates each term into [`Occur::Should`] clause
/// in a `BooleanQuery`, and keeps the scores as computed by the query.
///
/// This rewrite method only uses the top scoring terms so it will not overflow the boolean max
/// clause count.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TopTermsScoringBooleanQueryRewrite {
  size: usize,
}

impl TopTermsScoringBooleanQueryRewrite {
  /// Create a [`TopTermsScoringBooleanQueryRewrite`] for at most `size` terms.
  ///
  /// NOTE: if `IndexSearcher::get_max_clause_count` is smaller than `size`, then
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
    Q: MultiTermQuery + Into<MultiTermQuerySet>,
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
/// in a `BooleanQuery`, but adjusts the frequencies used for scoring to be blended across the
/// terms, otherwise the rarest term typically ranks highest (often not useful eg in the set of
/// expanded terms in a [`FuzzyQuery`]).
///
/// This rewrite method only uses the top scoring terms so it will not overflow the boolean max
/// clause count.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TopTermsBlendedFreqScoringRewrite {
  size: usize,
}

impl TopTermsBlendedFreqScoringRewrite {
  /// Create a [`TopTermsBlendedFreqScoringRewrite`] for at most `size` terms.
  ///
  /// NOTE: if `IndexSearcher::get_max_clause_count` is smaller than `size`, then
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
    Q: MultiTermQuery + Into<MultiTermQuerySet>,
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
/// in a `BooleanQuery`, and keeps the scores as computed by the query.
///
/// This rewrite method only uses the top scoring terms so it will not overflow the boolean max
/// clause count.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TopTermsBoostOnlyBooleanQueryRewrite {
  size: usize,
}
impl TopTermsBoostOnlyBooleanQueryRewrite {
  /// Create a [`TopTermsScoringBooleanQueryRewrite`] for at most `size` terms.
  ///
  /// NOTE: if `IndexSearcher::get_max_clause_count` is smaller than `size`, then
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
    let q = ConstantScoreQuery::new(tq);
    top_level.add(BoostQuery::new(q, boost)?, Occur::Should)?;
    Ok(())
  }
}

impl RewriteMethod for TopTermsBoostOnlyBooleanQueryRewrite {
  fn rewrite<IRC, Q>(self, index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQuerySet>,
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

use crate::core::index::term::Term;
use crate::core::index::term_states::TermStates;
use crate::core::search::blended_term_query::BooleanRewrite;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::doc_values_rewrite_method::DocValuesRewriteMethod;
use crate::core::search::scoring_rewrite::{ConstantScoreBooleanRewrite, ScoringBooleanRewrite};
use crate::core::search::term_collecting_rewrite::TermCollectingRewrite;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_terms_rewrite::TopTermsRewrite;
use crate::core::search::{blended_term_query, boolean_query, index_searcher};
pub(crate) use dispatch_multi_term_query;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MultiTermQuerySet {
  Automaton(AutomatonQuery),
  Fuzzy(FuzzyQuery),
  Prefix(PrefixQuery),
  Regexp(RegexpQuery),
  TermInSet(TermInSetQuery),
  TermRange(TermRangeQuery),
  Wildcard(WildcardQuery),

  #[cfg(test)]
  BoostChecking(BoostCheckingQuery),
  #[cfg(test)]
  DumbPrefix(DumbPrefixQuery),
  #[cfg(test)]
  DumbRegexp(DumbRegexpQuery),
}

impl QueryBase for MultiTermQuerySet {
  fn to_string(&self, field: &str) -> Result<String> {
    dispatch_multi_term_query!(self, |q| q.to_string(field))
  }

  fn create_weight<IRC>(
    self,
    searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    dispatch_multi_term_query!(self, |q| q.create_weight(searcher, score_mode, boost))
  }

  fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    dispatch_multi_term_query!(self, |q| q.rewrite(searcher))
  }

  fn visit<QV>(&self, visitor: &QV)
  where
    QV: QueryVisitor,
  {
    dispatch_multi_term_query!(self, |q| q.visit(visitor))
  }
}

impl Accountable for MultiTermQuerySet {
  fn ram_bytes_used(&self) -> Result<i64> {
    dispatch_multi_term_query!(self, |q| q.ram_bytes_used())
  }
}

impl HasIdentity for MultiTermQuerySet {
  fn identity(&self) -> &Identity {
    dispatch_multi_term_query!(self, |q| q.identity())
  }
}

impl_from_for_enum!(
    MultiTermQuerySet,
    AutomatonQuery => Automaton,
    FuzzyQuery => Fuzzy,
    PrefixQuery => Prefix,
    RegexpQuery => Regexp,
    TermInSetQuery => TermInSet,
    TermRangeQuery => TermRange,
    WildcardQuery => Wildcard,
);

#[cfg(test)]
impl_from_for_enum!(
    MultiTermQuerySet,
    BoostCheckingQuery => BoostChecking,
    DumbPrefixQuery => DumbPrefix,
    DumbRegexpQuery => DumbRegexp,
);

macro_rules! impl_into_query_for_multi_term_query {
    ($($ty:ty),* $(,)?) => {
        $(
            impl IntoQuery for $ty {
                fn into_query(self) -> Query {
                    MultiTermQuerySet::from(self).into()
                }
            }
        )*
    };
}

impl_into_query_for_multi_term_query!(
  AutomatonQuery,
  FuzzyQuery,
  PrefixQuery,
  RegexpQuery,
  TermInSetQuery,
  TermRangeQuery,
  WildcardQuery,
);

#[cfg(test)]
impl_into_query_for_multi_term_query!(BoostCheckingQuery, DumbPrefixQuery, DumbRegexpQuery,);

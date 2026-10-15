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
//! Closed built-in weights; recursive children retain concrete enum types.
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer};
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;
use crate::core::{document, search};

type VectorWeight<
  Q,
  IRC,
  SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity,
> = search::abstract_vector_similarity_query::AbstractVectorSimilarityQueryWeight<
  Q,
  IRC,
  std::sync::Arc<crate::core::index::query_timeout::QueryTimeoutEnum>,
  SIM,
>;
pub struct BuiltinQueryWeight<
  IRC: IndexReaderContext + 'static,
  SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity,
> where
  SIM: Similarity,
{
  inner: WeightNode<IRC, SIM>,
  reader_type: std::marker::PhantomData<fn() -> IRC>,
}
macro_rules! define_nodes {
  // Native weights keep their supplier type until this heterogeneous boundary.
  (native { $( $(#[$attrs:meta])* $native:ident: $native_ty:ty, )+ }
   unified { $( $unified:ident: $unified_ty:ty, )+ }
   native_inline { $( $native_inline:ident: $native_inline_ty:ty, )+ }
   unified_inline { $( $unified_inline:ident: $unified_inline_ty:ty, )+ }) => {
    enum WeightNode<IRC: IndexReaderContext + 'static, SIM> where SIM: Similarity, {
      $( $(#[$attrs])* $native(Box<$native_ty>), )+
      $( $unified(Box<$unified_ty>), )+
      $( $native_inline($native_inline_ty), )+
      $( $unified_inline($unified_inline_ty), )+
      #[cfg(test)] Fixture(Box<dyn Weight<IRC, SIM, ScorerSupplier=QueryWeightSs<IRC, SIM>> + Send + Sync>),
    }
    $( $(#[$attrs])* impl<IRC: IndexReaderContext + 'static, SIM> From<$native_ty> for BuiltinQueryWeight<IRC, SIM> where SIM: Similarity, {
      fn from(value: $native_ty) -> Self { Self { inner: WeightNode::$native(Box::new(value)), reader_type: std::marker::PhantomData } }
    } )+
    $( $(#[$attrs])* impl<IRC: IndexReaderContext + 'static, SIM> From<Box<$native_ty>> for BuiltinQueryWeight<IRC, SIM> where SIM: Similarity, {
      fn from(value: Box<$native_ty>) -> Self { Self { inner: WeightNode::$native(value), reader_type: std::marker::PhantomData } }
    } )+
    $( impl<IRC: IndexReaderContext + 'static, SIM> From<$unified_ty> for BuiltinQueryWeight<IRC, SIM> where SIM: Similarity, {
      fn from(value: $unified_ty) -> Self { Self { inner: WeightNode::$unified(Box::new(value)), reader_type: std::marker::PhantomData } }
    } )+
    $( impl<IRC: IndexReaderContext + 'static, SIM> From<Box<$unified_ty>> for BuiltinQueryWeight<IRC, SIM> where SIM: Similarity, {
      fn from(value: Box<$unified_ty>) -> Self { Self { inner: WeightNode::$unified(value), reader_type: std::marker::PhantomData } }
    } )+
    $( impl<IRC: IndexReaderContext + 'static, SIM> From<$native_inline_ty> for BuiltinQueryWeight<IRC, SIM> where SIM: Similarity, {
      fn from(value: $native_inline_ty) -> Self { Self { inner: WeightNode::$native_inline(value), reader_type: std::marker::PhantomData } }
    } )+
    $( impl<IRC: IndexReaderContext + 'static, SIM> From<Box<$native_inline_ty>> for BuiltinQueryWeight<IRC, SIM> where SIM: Similarity, {
      fn from(value: Box<$native_inline_ty>) -> Self { Self { inner: WeightNode::$native_inline(*value), reader_type: std::marker::PhantomData } }
    } )+
    $( impl<IRC: IndexReaderContext + 'static, SIM> From<$unified_inline_ty> for BuiltinQueryWeight<IRC, SIM> where SIM: Similarity, {
      fn from(value: $unified_inline_ty) -> Self { Self { inner: WeightNode::$unified_inline(value), reader_type: std::marker::PhantomData } }
    } )+
    $( impl<IRC: IndexReaderContext + 'static, SIM> From<Box<$unified_inline_ty>> for BuiltinQueryWeight<IRC, SIM> where SIM: Similarity, {
      fn from(value: Box<$unified_inline_ty>) -> Self { Self { inner: WeightNode::$unified_inline(*value), reader_type: std::marker::PhantomData } }
    } )+
    macro_rules! dispatch_weight {
      ($value:expr, |$inner:ident| $expression:expr, |$native_inner:ident| $native_expression:expr) => {
        match $value {
          $( $(#[$attrs])* WeightNode::$native($native_inner) => { let $native_inner = $native_inner.as_ref(); $native_expression }, )+
          $( WeightNode::$unified($inner) => { let $inner = $inner.as_ref(); $expression }, )+
          $( WeightNode::$native_inline($native_inner) => $native_expression, )+
          $( WeightNode::$unified_inline($inner) => $expression, )+
          #[cfg(test)] WeightNode::Fixture($inner) => { let $inner = $inner.as_ref(); $expression },
        }
      };
      ($value:expr, |$inner:ident| $expression:expr) => {
        dispatch_weight!($value, |$inner| $expression, |$inner| $expression)
      };
      (mut $value:expr, |$inner:ident| $expression:expr) => {
        match $value {
          $( $(#[$attrs])* WeightNode::$native($inner) => { let $inner = $inner.as_mut(); $expression }, )+
          $( WeightNode::$unified($inner) => { let $inner = $inner.as_mut(); $expression }, )+
          $( WeightNode::$native_inline($inner) => $expression, )+
          $( WeightNode::$unified_inline($inner) => $expression, )+
          #[cfg(test)] WeightNode::Fixture($inner) => { let $inner = $inner.as_mut(); $expression },
        }
      };
      (owned $value:expr, |$inner:ident| $expression:expr) => {
        match $value {
          $( $(#[$attrs])* WeightNode::$native($inner) => $expression, )+
          $( WeightNode::$unified($inner) => $expression, )+
          // These leaf weights have no wrapped weight to extract.
          $( WeightNode::$native_inline(_) => None, )+
          $( WeightNode::$unified_inline(_) => None, )+
          #[cfg(test)] WeightNode::Fixture($inner) => $expression,
        }
      };
    }
  };
}
define_nodes! {
  native {
    NumericRange: document::sorted_numeric_doc_values_range_query::SortedNumericDocValuesRangeQueryWeight,
    Term: search::term_query::TermWeight<SIM>,
    Boolean: search::boolean_weight::BooleanWeight<IRC, SIM>,
    Constant: search::constant_score_query::ConstantScoreQueryWeight<IRC, SIM>,
    Phrase: search::phrase_weight::PhraseWeight<search::phrase_query::PhraseQueryWeightBase<SIM>, SIM>,
    MultiPhrase: search::phrase_weight::PhraseWeight<search::multi_phrase_query::MultiPhraseQueryWeightBase<SIM>, SIM>,
    IndexOrDocValues: search::index_or_doc_values_query::IndexOrDocValuesQueryWeight<IRC, SIM>,
    Synonym: search::synonym_query::SynonymWeight<SIM>,
    RewritePrefix: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::prefix_query::PrefixQuery>,
    RewriteFuzzy: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::fuzzy_query::FuzzyQuery>,
    RewriteAutomaton: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::automaton_query::AutomatonQuery>,
    RewriteRegexp: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::regexp_query::RegexpQuery>,
    RewriteWildcard: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::wildcard_query::WildcardQuery>,
    RewriteTermRange: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::term_range_query::TermRangeQuery>,
    RewriteTermSet: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::term_in_set_query::TermInSetQuery>,
    BinaryRange: document::binary_range_field_range_query::BinaryRangeFieldRangeWeight,
    NumericSet: document::sorted_numeric_doc_values_set_query::SortedNumericDocValuesSetQueryWeight,
    LatLonDocValues: document::lat_lon_doc_values_query::LatLonDocValuesQueryWeight,
    LatLonBox: document::lat_lon_doc_values_box_query::LatLonDocValuesBoxQueryWeight,
    LatLonDistance: document::lat_lon_point_distance_query::LatLonPointDistanceWeight,
    LatLonDistanceFeature: document::lat_lon_point_distance_feature_query::LatLonPointDistanceFeatureWeight,
    XYDocValues: document::xy_doc_values_point_in_geometry_query::XYDocValuesPointInGeometryQueryWeight,
    XYPoint: document::xy_point_in_geometry_query::XYPointInGeometryWeight,
    Automaton: crate::sandbox::search::term_automaton_query::TermAutomatonWeight<SIM>,
    #[cfg(test)] AssertingWeight: crate::test_framework::core::search::asserting_weight::AssertingWeight<IRC, SIM>,
    #[cfg(test)] WANDScorerQueryWeight: crate::test_framework::core::search::query::WANDScorerQueryWeight,
    #[cfg(test)] WeightWrapper: crate::test::core::search::test_lru_query_cache::WeightWrapper<IRC, SIM>,
    #[cfg(test)] BrokenExplainWeight: crate::test_framework::core::search::query::BrokenExplainWeight<IRC, SIM>,
    #[cfg(test)] BitSetQueryWeight: crate::test_framework::core::search::query::BitSetQueryWeight,
    #[cfg(test)] RandomQueryWeight: crate::test_framework::core::search::query::RandomQueryWeight,
    #[cfg(test)] DummyQueryWeight1: crate::test_framework::core::search::query::DummyQueryWeight1,
    #[cfg(test)] CrazyMustUseBulkScorerWeight: crate::test_framework::core::search::query::CrazyMustUseBulkScorerWeight,
    #[cfg(test)] TestLRUWeight: crate::test_framework::core::search::query::TestLRUWeight,
    #[cfg(test)] DVCacheWeight: crate::test_framework::core::search::query::DVCacheWeight,
    #[cfg(test)] MaxScoreWrapperQueryWeight: crate::test_framework::core::search::query::MaxScoreWrapperQueryWeight<IRC, SIM>,
    #[cfg(test)] BlockScoreWeight: crate::test_framework::core::search::block_score_query_wrapper::BlockScoreWeight<IRC, SIM>,
    #[cfg(test)] RandomApproximationWeight: crate::test_framework::core::search::random_approximation_query::RandomApproximationWeight<IRC, SIM>,
    #[cfg(test)] RewriteBoostCheckingQuery: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<crate::test_framework::core::search::multi_term::BoostCheckingQuery>,
    #[cfg(test)] RewriteDumbPrefixQuery: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<crate::test_framework::core::search::multi_term::DumbPrefixQuery>,
    #[cfg(test)] RewriteDumbRegexpQuery: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<crate::test_framework::core::search::multi_term::DumbRegexpQuery>,
  }
  unified {
    DisjunctionMax: search::disjunction_max_query::DisjunctionMaxWeight<IRC, SIM>,
    IndexSortRange: search::index_sort_sorted_numeric_doc_values_range_query::IndexSortSortedNumericDocValuesRangeQueryWeight<IRC, SIM>,
    PointRange: search::point_range_query::PointRangeWeight,
    PointSet: search::point_in_set_query::PointInSetWeight,
    Named: search::named_matches::NamedWeight<IRC, SIM>,
    ByteVector: VectorWeight<search::byte_vector_similarity_query::ByteVectorSimilarityQuery, IRC, SIM>,
    FloatVector: VectorWeight<search::float_vector_similarity_query::FloatVectorSimilarityQuery, IRC, SIM>,
    Caching: search::lru_query_cache::CachingWrapperWeight<IRC, SIM>,
    LatLonPoint: document::spatial_query::SpatialWeight<crate::core::geo::lat_lon_geometry::LatLonGeometryEnum, document::lat_lon_point_query::LatLonPointSpatial>,
  }
  native_inline {
    MatchNone: search::match_no_docs_query::MatchNoDocsWeight,
    SortedSetRange: document::sorted_set_doc_values_range_query::SortedSetDocValuesRangeQueryWeight,
    DocAndScore: search::abstract_knn_vector_query::DocAndScoreQueryWeight,
    MatchAll: search::match_all_docs_query::MatchAllWeight,
    FieldExists: search::field_exists_query::FieldExistsWeight,
    DocValuesRewrite: search::doc_values_rewrite_method::MultiTermQueryDocValuesWeight,
  }
  unified_inline {
    RangeField: document::range_field_query::RangeFieldWeight,
  }
}

impl<IRC: IndexReaderContext + 'static, SIM> SegmentCacheable<IRC> for BuiltinQueryWeight<IRC, SIM>
where
  SIM: Similarity,
{
  fn is_cacheable(&self, context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    dispatch_weight!(&self.inner, |inner| SegmentCacheable::<IRC>::is_cacheable(
      inner, context
    ))
  }
}
impl<IRC: IndexReaderContext + 'static, SIM> Weight<IRC, SIM> for BuiltinQueryWeight<IRC, SIM>
where
  SIM: Similarity,
{
  type ScorerSupplier = QueryWeightSs<IRC, SIM>;
  fn matches<'a>(
    &'a self,
    context: &'a LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &'a IndexSearcher<IRC, SIM>,
  ) -> Result<Option<search::query::QueryWeightMatches<'a, IRC, SIM>>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    dispatch_weight!(&self.inner, |inner| Weight::<IRC, SIM>::matches(
      inner, context, doc, searcher
    ))
  }
  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<search::explanation::Explanation>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    dispatch_weight!(&self.inner, |inner| Weight::<IRC, SIM>::explain(
      inner, context, doc, searcher
    ))
  }
  fn get_query(&self) -> std::sync::Arc<search::query::Query> {
    dispatch_weight!(&self.inner, |inner| Weight::<IRC, SIM>::get_query(inner))
  }
  fn scorer(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<QueryWeightSsScorer<IRC, SIM>>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    dispatch_weight!(
      &self.inner,
      |inner| Weight::<IRC, SIM>::scorer(inner, context, searcher),
      |inner| Weight::<IRC, SIM>::scorer(inner, context, searcher)
        .map(|scorer| scorer.map(QueryWeightSsScorer::<IRC, SIM>::new))
    )
  }
  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<QueryWeightSs<IRC, SIM>>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    dispatch_weight!(
      &self.inner,
      |inner| Weight::<IRC, SIM>::scorer_supplier(inner, context, searcher),
      |inner| Weight::<IRC, SIM>::scorer_supplier(inner, context, searcher)
        .map(|supplier| supplier.map(Into::into))
    )
  }
  fn bulk_scorer(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<QueryWeightSsBulkScorer<IRC, SIM>>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    dispatch_weight!(
      &self.inner,
      |inner| Weight::<IRC, SIM>::bulk_scorer(inner, context, searcher),
      |inner| Weight::<IRC, SIM>::bulk_scorer(inner, context, searcher)
        .map(|scorer| scorer.map(QueryWeightSsBulkScorer::<IRC, SIM>::new))
    )
  }
  fn count(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<i32>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    dispatch_weight!(&self.inner, |inner| Weight::<IRC, SIM>::count(
      inner, context, searcher
    ))
  }
  fn default_count(&self, context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
    dispatch_weight!(&self.inner, |inner| Weight::<IRC, SIM>::default_count(
      inner, context
    ))
  }
  fn is_cache_wrapper(&self) -> bool {
    dispatch_weight!(&self.inner, |inner| Weight::<IRC, SIM>::is_cache_wrapper(
      inner
    ))
  }
  fn into_inner_weight(self: Box<Self>) -> Option<search::query::QueryWeight<IRC, SIM>> {
    (*self).into_inner_weight()
  }
  #[cfg(test)]
  fn as_any(&mut self) -> &mut dyn std::any::Any {
    dispatch_weight!(mut &mut self.inner, |inner| Weight::<IRC, SIM>::as_any(
      inner
    ))
  }
}

impl<IRC: IndexReaderContext + 'static, SIM> AsRef<Self> for BuiltinQueryWeight<IRC, SIM>
where
  SIM: Similarity,
{
  fn as_ref(&self) -> &Self {
    self
  }
}

impl<IRC: IndexReaderContext + 'static, SIM> BuiltinQueryWeight<IRC, SIM>
where
  SIM: Similarity,
{
  pub fn into_inner_weight(self) -> Option<search::query::QueryWeight<IRC, SIM>> {
    dispatch_weight!(owned self.inner, |inner| Weight::<IRC, SIM>::into_inner_weight(inner))
  }
}

#[cfg(test)]
impl<IRC: IndexReaderContext + 'static, SIM> BuiltinQueryWeight<IRC, SIM>
where
  SIM: Similarity,
{
  pub(crate) fn fixture<
    W: search::weight::Weight<IRC, SIM, ScorerSupplier = QueryWeightSs<IRC, SIM>>
      + Send
      + Sync
      + 'static,
  >(
    weight: W,
  ) -> Self {
    Self {
      inner: WeightNode::Fixture(Box::new(weight)),
      reader_type: std::marker::PhantomData,
    }
  }
}

#[cfg(test)]
impl<IRC: IndexReaderContext + 'static, SIM>
  From<
    crate::test_framework::core::search::base_vector_similarity_query_test_case::CountingWeight<
      IRC,
      SIM,
    >,
  > for BuiltinQueryWeight<IRC, SIM>
where
  SIM: Similarity,
{
  fn from(
    value: crate::test_framework::core::search::base_vector_similarity_query_test_case::CountingWeight<IRC, SIM>,
  ) -> Self {
    Self::fixture(value)
  }
}

#[cfg(test)]
impl<IRC: IndexReaderContext + 'static, SIM>
  From<crate::test_framework::core::search::query::AssertNeedsScoresWeight<IRC, SIM>>
  for BuiltinQueryWeight<IRC, SIM>
where
  SIM: Similarity,
{
  fn from(
    value: crate::test_framework::core::search::query::AssertNeedsScoresWeight<IRC, SIM>,
  ) -> Self {
    Self::fixture(value)
  }
}

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
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;
use crate::core::{document, search};

type VectorWeight<Q, IRC> =
  search::abstract_vector_similarity_query::AbstractVectorSimilarityQueryWeight<
    Q,
    IRC,
    std::sync::Arc<crate::core::index::query_timeout::QueryTimeoutEnum>,
  >;
pub struct BuiltinQueryWeight<IRC: IndexReaderContext + 'static> {
  inner: WeightNode<IRC>,
  reader_type: std::marker::PhantomData<fn() -> IRC>,
}
macro_rules! define_nodes {
  // Native weights keep their supplier type until this heterogeneous boundary.
  (native { $( $(#[$attrs:meta])* $native:ident: $native_ty:ty, )+ }
   unified { $( $unified:ident: $unified_ty:ty, )+ }) => {
    enum WeightNode<IRC: IndexReaderContext + 'static> {
      $( $(#[$attrs])* $native(Box<$native_ty>), )+
      $( $unified(Box<$unified_ty>), )+
      #[cfg(test)] Fixture(Box<dyn Weight<IRC, ScorerSupplier=QueryWeightSs<IRC>> + Send + Sync>),
    }
    $( $(#[$attrs])* impl<IRC: IndexReaderContext + 'static> From<Box<$native_ty>> for BuiltinQueryWeight<IRC> {
      fn from(value: Box<$native_ty>) -> Self { Self { inner: WeightNode::$native(value), reader_type: std::marker::PhantomData } }
    } )+
    $( impl<IRC: IndexReaderContext + 'static> From<Box<$unified_ty>> for BuiltinQueryWeight<IRC> {
      fn from(value: Box<$unified_ty>) -> Self { Self { inner: WeightNode::$unified(value), reader_type: std::marker::PhantomData } }
    } )+
    macro_rules! dispatch_weight {
      ($value:expr, |$inner:ident| $expression:expr, |$native_inner:ident| $native_expression:expr) => {
        match $value {
          $( $(#[$attrs])* WeightNode::$native($native_inner) => $native_expression, )+
          $( WeightNode::$unified($inner) => $expression, )+
          #[cfg(test)] WeightNode::Fixture($inner) => $expression,
        }
      };
      ($value:expr, |$inner:ident| $expression:expr) => {
        match $value {
          $( $(#[$attrs])* WeightNode::$native($inner) => $expression, )+
          $( WeightNode::$unified($inner) => $expression, )+
          #[cfg(test)] WeightNode::Fixture($inner) => $expression,
        }
      };
    }
  };
}
define_nodes! {
  native {
    Term: search::term_query::TermWeight,
    Boolean: search::boolean_weight::BooleanWeight<IRC>,
    Constant: search::constant_score_query::ConstantScoreQueryWeight<IRC>,
    Phrase: search::phrase_weight::PhraseWeight<search::phrase_query::PhraseQueryWeightBase>,
    MultiPhrase: search::phrase_weight::PhraseWeight<search::multi_phrase_query::MultiPhraseQueryWeightBase>,
    MatchAll: search::match_all_docs_query::MatchAllWeight,
    FieldExists: search::field_exists_query::FieldExistsWeight,
    IndexOrDocValues: search::index_or_doc_values_query::IndexOrDocValuesQueryWeight<IRC>,
    Synonym: search::synonym_query::SynonymWeight,
    DocAndScore: search::abstract_knn_vector_query::DocAndScoreQueryWeight,
    DocValuesRewrite: search::doc_values_rewrite_method::MultiTermQueryDocValuesWeight,
    RewritePrefix: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::prefix_query::PrefixQuery>,
    RewriteFuzzy: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::fuzzy_query::FuzzyQuery>,
    RewriteAutomaton: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::automaton_query::AutomatonQuery>,
    RewriteRegexp: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::regexp_query::RegexpQuery>,
    RewriteWildcard: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::wildcard_query::WildcardQuery>,
    RewriteTermRange: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::term_range_query::TermRangeQuery>,
    RewriteTermSet: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::term_in_set_query::TermInSetQuery>,
    BinaryRange: document::binary_range_field_range_query::BinaryRangeFieldRangeWeight,
    NumericSet: document::sorted_numeric_doc_values_set_query::SortedNumericDocValuesSetQueryWeight,
    SortedSetRange: document::sorted_set_doc_values_range_query::SortedSetDocValuesRangeQueryWeight,
    LatLonDocValues: document::lat_lon_doc_values_query::LatLonDocValuesQueryWeight,
    LatLonBox: document::lat_lon_doc_values_box_query::LatLonDocValuesBoxQueryWeight,
    LatLonDistance: document::lat_lon_point_distance_query::LatLonPointDistanceWeight,
    LatLonDistanceFeature: document::lat_lon_point_distance_feature_query::LatLonPointDistanceFeatureWeight,
    XYDocValues: document::xy_doc_values_point_in_geometry_query::XYDocValuesPointInGeometryQueryWeight,
    XYPoint: document::xy_point_in_geometry_query::XYPointInGeometryWeight,
    Automaton: crate::sandbox::search::term_automaton_query::TermAutomatonWeight,
    #[cfg(test)] RewriteBoostCheckingQuery: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<crate::test_framework::core::search::multi_term::BoostCheckingQuery>,
    #[cfg(test)] RewriteDumbPrefixQuery: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<crate::test_framework::core::search::multi_term::DumbPrefixQuery>,
    #[cfg(test)] RewriteDumbRegexpQuery: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<crate::test_framework::core::search::multi_term::DumbRegexpQuery>,
  }
  unified {
    DisjunctionMax: search::disjunction_max_query::DisjunctionMaxWeight<IRC>,
    MatchNone: search::match_no_docs_query::MatchNoDocsWeight,
    IndexSortRange: search::index_sort_sorted_numeric_doc_values_range_query::IndexSortSortedNumericDocValuesRangeQueryWeight<IRC>,
    PointRange: search::point_range_query::PointRangeWeight,
    PointSet: search::point_in_set_query::PointInSetWeight,
    Named: search::named_matches::NamedWeight<IRC>,
    ByteVector: VectorWeight<search::byte_vector_similarity_query::ByteVectorSimilarityQuery, IRC>,
    FloatVector: VectorWeight<search::float_vector_similarity_query::FloatVectorSimilarityQuery, IRC>,
    Caching: search::lru_query_cache::CachingWrapperWeight<IRC>,
    RangeField: document::range_field_query::RangeFieldWeight,
    NumericRange: document::sorted_numeric_doc_values_range_query::SortedNumericDocValuesRangeQueryWeight,
    LatLonPoint: document::spatial_query::SpatialWeight<crate::core::geo::lat_lon_geometry::LatLonGeometryEnum, document::lat_lon_point_query::LatLonPointSpatial>,
  }
}

impl<IRC: IndexReaderContext + 'static> SegmentCacheable<IRC> for BuiltinQueryWeight<IRC> {
  fn is_cacheable(&self, context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    dispatch_weight!(&self.inner, |inner| SegmentCacheable::<IRC>::is_cacheable(
      inner.as_ref(),
      context
    ))
  }
}
impl<IRC: IndexReaderContext + 'static> Weight<IRC> for BuiltinQueryWeight<IRC> {
  type ScorerSupplier = QueryWeightSs<IRC>;
  fn matches<'a>(
    &'a self,
    context: &'a LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &'a IndexSearcher<IRC>,
  ) -> Result<Option<search::query::QueryWeightMatches<'a, IRC>>>
  where
    IndexSearcher<IRC>: Sync,
  {
    dispatch_weight!(&self.inner, |inner| Weight::<IRC>::matches(
      inner.as_ref(),
      context,
      doc,
      searcher
    ))
  }
  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<search::explanation::Explanation>
  where
    IndexSearcher<IRC>: Sync,
  {
    dispatch_weight!(&self.inner, |inner| Weight::<IRC>::explain(
      inner.as_ref(),
      context,
      doc,
      searcher
    ))
  }
  fn get_query(&self) -> std::sync::Arc<search::query::Query> {
    dispatch_weight!(&self.inner, |inner| Weight::<IRC>::get_query(
      inner.as_ref()
    ))
  }
  fn scorer(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<QueryWeightSsScorer<IRC>>>
  where
    IndexSearcher<IRC>: Sync,
  {
    dispatch_weight!(&self.inner, |inner| Weight::<IRC>::scorer(
      inner.as_ref(),
      context,
      searcher
    ))
  }
  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<QueryWeightSs<IRC>>>
  where
    IndexSearcher<IRC>: Sync,
  {
    dispatch_weight!(
      &self.inner,
      |inner| Weight::<IRC>::scorer_supplier(inner.as_ref(), context, searcher),
      |inner| Weight::<IRC>::scorer_supplier(inner.as_ref(), context, searcher)
        .map(|supplier| supplier.map(Into::into))
    )
  }
  fn bulk_scorer(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<QueryWeightSsBulkScorer<IRC>>>
  where
    IndexSearcher<IRC>: Sync,
  {
    dispatch_weight!(
      &self.inner,
      |inner| Weight::<IRC>::bulk_scorer(inner.as_ref(), context, searcher),
      |inner| Weight::<IRC>::bulk_scorer(inner.as_ref(), context, searcher)
        .map(|scorer| scorer.map(QueryWeightSsBulkScorer::<IRC>::new))
    )
  }
  fn count(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<i32>
  where
    IndexSearcher<IRC>: Sync,
  {
    dispatch_weight!(&self.inner, |inner| Weight::<IRC>::count(
      inner.as_ref(),
      context,
      searcher
    ))
  }
  fn default_count(&self, context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
    dispatch_weight!(&self.inner, |inner| Weight::<IRC>::default_count(
      inner.as_ref(),
      context
    ))
  }
  fn is_cache_wrapper(&self) -> bool {
    dispatch_weight!(&self.inner, |inner| Weight::<IRC>::is_cache_wrapper(
      inner.as_ref()
    ))
  }
  fn into_inner_weight(self: Box<Self>) -> Option<search::query::QueryWeight<IRC>> {
    (*self).into_inner_weight()
  }
  #[cfg(test)]
  fn as_any(&mut self) -> &mut dyn std::any::Any {
    dispatch_weight!(&mut self.inner, |inner| Weight::<IRC>::as_any(
      inner.as_mut()
    ))
  }
}

impl<IRC: IndexReaderContext + 'static> AsRef<Self> for BuiltinQueryWeight<IRC> {
  fn as_ref(&self) -> &Self {
    self
  }
}

impl<IRC: IndexReaderContext + 'static> BuiltinQueryWeight<IRC> {
  pub fn into_inner_weight(self) -> Option<search::query::QueryWeight<IRC>> {
    dispatch_weight!(self.inner, |inner| Weight::<IRC>::into_inner_weight(inner))
  }
}

#[cfg(test)]
impl<IRC: IndexReaderContext + 'static> BuiltinQueryWeight<IRC> {
  pub(crate) fn fixture<
    W: search::weight::Weight<IRC, ScorerSupplier = QueryWeightSs<IRC>> + Send + Sync + 'static,
  >(
    weight: W,
  ) -> Self {
    Self {
      inner: WeightNode::Fixture(Box::new(weight)),
      reader_type: std::marker::PhantomData,
    }
  }
}

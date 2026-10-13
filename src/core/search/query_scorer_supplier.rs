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
//! Closed built-in suppliers; no production trait-object fallback.
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{QueryWeightSsBulkScorer, QueryWeightSsScorer};
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::util::error::lucene_error::Result;
use crate::core::{document, search};

type PointValues<IRC> = <IRCLeafReader<IRC> as LeafReader>::PointValues;
type RewriteSupplier<IRC, Q> =
  search::abstract_multi_term_query_constant_score_wrapper::ScorerSupplierImpl<
    std::rc::Rc<<IRCLeafReader<IRC> as LeafReader>::Terms>,
    <Q as search::multi_term_query::MultiTermQuery>::TermsEnum<
      std::rc::Rc<<IRCLeafReader<IRC> as LeafReader>::Terms>,
    >,
    Q,
  >;
type FilteredVectorSupplier<IRC> = search::abstract_vector_similarity_query::VectorSimilarityScorerSupplier<
  search::abstract_vector_similarity_query::FilteredDocIdSetIteratorImpl1<
    search::vector_scorer::VectorScorerEnum2<
      <search::scorer_types::LeafScorerTypes<IRCLeafReader<IRC>> as search::scorer_types::ScorerTypes>::ByteScorer,
      <search::scorer_types::LeafScorerTypes<IRCLeafReader<IRC>> as search::scorer_types::ScorerTypes>::FloatScorer,
    >,
    crate::core::util::bit_set::SparseFixedBitSetBitSet,
  >,
>;
pub struct BuiltinQueryScorerSupplier<IRC: IndexReaderContext + 'static> {
  inner: SupplierNode<IRC>,
}
macro_rules! define_nodes {
  (boxed { $( $(#[$attrs:meta])* $variant:ident: $ty:ty, )+ }
   inline { $( $inline:ident: $inline_ty:ty, )+ }) => {
    enum SupplierNode<IRC: IndexReaderContext + 'static> {
      $( $(#[$attrs])* $variant(Box<$ty>), )+
      $( $inline($inline_ty), )+
      Default(search::weight::DefaultScorerSupplier<QueryWeightSsScorer<IRC>>),
    }
    $( $(#[$attrs])* impl<IRC: IndexReaderContext + 'static> From<$ty> for BuiltinQueryScorerSupplier<IRC> {
      fn from(value: $ty) -> Self { Self { inner: SupplierNode::$variant(Box::new(value)) } }
    } )+
    $( $(#[$attrs])* impl<IRC: IndexReaderContext + 'static> From<Box<$ty>> for BuiltinQueryScorerSupplier<IRC> {
      fn from(value: Box<$ty>) -> Self { Self { inner: SupplierNode::$variant(value) } }
    } )+
    $( impl<IRC: IndexReaderContext + 'static> From<$inline_ty> for BuiltinQueryScorerSupplier<IRC> {
      fn from(value: $inline_ty) -> Self { Self { inner: SupplierNode::$inline(value) } }
    } )+
    $( impl<IRC: IndexReaderContext + 'static> From<Box<$inline_ty>> for BuiltinQueryScorerSupplier<IRC> {
      fn from(value: Box<$inline_ty>) -> Self { Self { inner: SupplierNode::$inline(*value) } }
    } )+
    macro_rules! dispatch_supplier {
      ($value:expr, |$inner:ident| $expression:expr) => {
        match $value {
          $( $(#[$attrs])* SupplierNode::$variant($inner) => { let $inner = $inner.as_mut(); $expression }, )+
          $( SupplierNode::$inline($inner) => $expression, )+
          SupplierNode::Default($inner) => $expression,
        }
      };
    }
  };
}
define_nodes! {
  boxed {
    Term: search::term_query::TermScorerSupplier<IRCLeafReader<IRC>>,
    Boolean: search::boolean_scorer_supplier::BooleanScorerSupplier<IRC>,
    Constant: search::constant_score_query::ScorerSupplierImpl<IRC>,
    DisjunctionMax: search::disjunction_max_query::ScorerSupplierImpl<IRC>,
    IndexOrDocValues: search::index_or_doc_values_query::IndexOrDocValuesQuerySs<IRC>,
    IndexSort: search::index_sort_sorted_numeric_doc_values_range_query::ScorerSupplierImpl<search::index_sort_sorted_numeric_doc_values_range_query::Disi<IRCLeafReader<IRC>>>,
    PointRange: search::point_range_query::ScorerSupplierImpl1<PointValues<IRC>>,
    MergePoint: search::point_in_set_query::MergePointScorerSupplier<PointValues<IRC>>,
    SinglePoint: search::point_in_set_query::SinglePointScorerSupplier<PointValues<IRC>>,
    Synonym: search::synonym_query::SynonymScorerSupplier<IRCLeafReader<IRC>>,
    RewritePrefix: RewriteSupplier<IRC, search::prefix_query::PrefixQuery>,
    RewriteFuzzy: RewriteSupplier<IRC, search::fuzzy_query::FuzzyQuery>,
    RewriteAutomaton: RewriteSupplier<IRC, search::automaton_query::AutomatonQuery>,
    RewriteRegexp: RewriteSupplier<IRC, search::regexp_query::RegexpQuery>,
    RewriteWildcard: RewriteSupplier<IRC, search::wildcard_query::WildcardQuery>,
    RewriteTermRange: RewriteSupplier<IRC, search::term_range_query::TermRangeQuery>,
    RewriteTermSet: RewriteSupplier<IRC, search::term_in_set_query::TermInSetQuery>,
    VectorScores: search::abstract_vector_similarity_query::VectorSimilarityScorerSupplier<search::abstract_vector_similarity_query::DocsIteratorImpl>,
    VectorFiltered: FilteredVectorSupplier<IRC>,
    CacheMiss: search::lru_query_cache::ScorerSupplierImpl1<<IRCLeafReader<IRC> as LeafReader>::CacheHelper, IRC>,
    CacheHit: search::lru_query_cache::ScorerSupplierImpl2,
    RangeField: document::range_field_query::RangeFieldScorerSupplier<PointValues<IRC>>,
    SortedSetRange: document::sorted_set_doc_values_range_query::ScorerSupplierImpl3<crate::core::index::doc_values::SortedSet<IRCLeafReader<IRC>>>,
    LatLonDistance: document::lat_lon_point_distance_query::ScorerSupplierImpl<PointValues<IRC>>,
    LatLonFeature: document::lat_lon_point_distance_feature_query::ScorerSupplierImpl<document::lat_lon_point_distance_feature_query::SelectValue<IRC>>,
    Spatial: document::spatial_query::RelationScorerSupplier<PointValues<IRC>, std::sync::Arc<<document::lat_lon_point_query::LatLonPointSpatial as document::spatial_query::SpatialQueryBase>::SpatialVisitor>>,
    XYPoint: document::xy_point_in_geometry_query::XYPointInGeometryScorerSupplier<PointValues<IRC>>,
    #[cfg(test)] Fake: crate::test::core::search::test_boolean2_scorer_supplier::FakeScorerSupplier,
    #[cfg(test)] Asserting: crate::test_framework::core::search::asserting_weight::AssertingScorerSupplier<IRC>,
    #[cfg(test)] WeightWrapper: crate::test::core::search::test_lru_query_cache::WeightWrapperScorerSupplier<IRC>,
    #[cfg(test)] CrazyMustUseBulk: crate::test_framework::core::search::query::CrazyMustUseBulkScorerSupplier,
    #[cfg(test)] TestLRU: crate::test_framework::core::search::query::TestLRUScorerSupplier,
    #[cfg(test)] DVCache: crate::test_framework::core::search::query::DVCacheScorerSupplier,
    #[cfg(test)] MaxScoreWrapper: crate::test_framework::core::search::query::ScorerSupplierImpl<IRC>,
    #[cfg(test)] RewriteBoostCheckingQuery: RewriteSupplier<IRC, crate::test_framework::core::search::multi_term::BoostCheckingQuery>,
    #[cfg(test)] RewriteDumbPrefixQuery: RewriteSupplier<IRC, crate::test_framework::core::search::multi_term::DumbPrefixQuery>,
    #[cfg(test)] RewriteDumbRegexpQuery: RewriteSupplier<IRC, crate::test_framework::core::search::multi_term::DumbRegexpQuery>,
  }
  inline {
    DocValues: search::doc_values_rewrite_method::DocValuesScorerSupplier,
    MatchAll: search::match_all_docs_query::MatchAllDocsScorerSupplier,
    PointAll: search::point_range_query::ScorerSupplierImpl,
    RangeAll: document::range_field_query::RangeFieldAllScorerSupplier,
    SpatialAll: document::spatial_query::ScorerSupplierImpl,
  }
}

impl<IRC: IndexReaderContext + 'static> ScorerSupplier<IRC> for BuiltinQueryScorerSupplier<IRC> {
  type Scorer = QueryWeightSsScorer<IRC>;
  type BulkScorer = QueryWeightSsBulkScorer<IRC>;
  fn get(
    &mut self,
    lead_cost: i64,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<QueryWeightSsScorer<IRC>>
  where
    IndexSearcher<IRC>: Sync,
  {
    dispatch_supplier!(&mut self.inner, |inner| {
      ScorerSupplier::<IRC>::get(inner, lead_cost, context, searcher)
        .map(QueryWeightSsScorer::<IRC>::new)
    })
  }
  fn bulk_scorer(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<QueryWeightSsBulkScorer<IRC>>>
  where
    IndexSearcher<IRC>: Sync,
  {
    dispatch_supplier!(&mut self.inner, |inner| {
      ScorerSupplier::<IRC>::bulk_scorer(inner, context, searcher)
        .map(|scorer| scorer.map(QueryWeightSsBulkScorer::<IRC>::new))
    })
  }
  fn default_bulk_scorer(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<search::weight::DefaultBulkScorer<QueryWeightSsScorer<IRC>>>
  where
    IndexSearcher<IRC>: Sync,
  {
    dispatch_supplier!(&mut self.inner, |inner| {
      ScorerSupplier::<IRC>::default_bulk_scorer(inner, context, searcher).map(|scorer| {
        search::weight::DefaultBulkScorer::new(QueryWeightSsScorer::<IRC>::new(scorer.scorer))
      })
    })
  }
  fn cost(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<i64>
  where
    IndexSearcher<IRC>: Sync,
  {
    dispatch_supplier!(&mut self.inner, |inner| ScorerSupplier::<IRC>::cost(
      inner, context, searcher
    ))
  }
  fn set_top_level_scoring_clause(&mut self) -> Result<()> {
    dispatch_supplier!(&mut self.inner, |inner| {
      ScorerSupplier::<IRC>::set_top_level_scoring_clause(inner)
    })
  }
  #[cfg(test)]
  fn as_any(&mut self) -> &mut dyn std::any::Any {
    dispatch_supplier!(&mut self.inner, |inner| ScorerSupplier::<IRC>::as_any(
      inner
    ))
  }
}

impl<IRC, S> From<search::weight::DefaultScorerSupplier<S>> for BuiltinQueryScorerSupplier<IRC>
where
  IRC: IndexReaderContext + 'static,
  S: search::scorer::Scorer
    + search::query_scorer::IntoQueryScorer<search::scorer_types::LeafScorerTypes<IRCLeafReader<IRC>>>
    + 'static,
{
  fn from(value: search::weight::DefaultScorerSupplier<S>) -> Self {
    Self {
      inner: SupplierNode::Default(search::weight::DefaultScorerSupplier {
        scorer: value.scorer.map(QueryWeightSsScorer::<IRC>::new),
      }),
    }
  }
}

impl<IRC, S> From<Box<search::weight::DefaultScorerSupplier<S>>> for BuiltinQueryScorerSupplier<IRC>
where
  IRC: IndexReaderContext + 'static,
  S: search::scorer::Scorer
    + search::query_scorer::IntoQueryScorer<search::scorer_types::LeafScorerTypes<IRCLeafReader<IRC>>>
    + 'static,
{
  fn from(value: Box<search::weight::DefaultScorerSupplier<S>>) -> Self {
    (*value).into()
  }
}

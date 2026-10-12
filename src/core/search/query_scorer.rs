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
//! The closed set of built-in scorers. Recursive query nodes own concrete boxes.
use crate::core::document::{
  binary_range_field_range_query, lat_lon_doc_values_box_query, lat_lon_doc_values_query,
  lat_lon_point_distance_feature_query, sorted_numeric_doc_values_range_query,
  sorted_numeric_doc_values_set_query, sorted_set_doc_values_range_query,
  xy_doc_values_point_in_geometry_query,
};
use crate::core::geo::geometry::Geometry;
use crate::core::geo::lat_lon_geometry::{LatLonGeometryEnumComponent2D, LatLonGeometryType};
use crate::core::geo::xy_geometry::{XYGeometryEnum, XYGeometryType};
use crate::core::index::doc_values::NumericDocValuesEnum3WithEmpty;
use crate::core::index::doc_values::{
  BinaryDocValuesWithEmpty, SortedDocValuesWithEmpty, SortedNumericDocValuesEnum3WithEmpty,
};
use crate::core::index::impacts_enum::ImpactsEnumEnum2;
use crate::core::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
use crate::core::index::singleton_sorted_set_doc_values::SingletonSortedSetDocValues;
use crate::core::index::slow_impacts_enum::SlowImpactsEnum;
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::sorted_set_doc_values_writer::SortedSetDocValuesEnum2;
use crate::core::search::block_max_conjunction_scorer::BlockMaxConjunctionScorer;
use crate::core::search::boolean_scorer_supplier::FilterScorerImpl;
use crate::core::search::conjunction_scorer::ConjunctionScorer;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::disjunction_disi_approximation::DisjunctionDISIApproximation;
use crate::core::search::disjunction_max_scorer::DisjunctionMaxScorer;
use crate::core::search::disjunction_scorer::DisjunctionScorer;
use crate::core::search::disjunction_sum_scorer::DisjunctionSumScorer;
use crate::core::search::doc_id_set_iterator::{
  AllDISI, BitSetIteratorAccess, DocIdSetIterator, DocIdSetIteratorExtensions, EmptyDISI,
};
use crate::core::search::doc_id_set_iterator::{
  DocIdSetIteratorEnum2, DocIdSetIteratorEnum4, DocIdSetIteratorEnum5,
};
use crate::core::search::doc_values_range_iterator::DocValuesRangeIterator;
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::multi_phrase_query::MultiPhrasePostingsEnum;
use crate::core::search::phrase_scorer::PhraseScorer;
use crate::core::search::phrase_weight::SimScorerType;
use crate::core::search::req_excl_scorer::ReqExclScorer;
use crate::core::search::req_opt_sum_scorer::ReqOptSumScorer;
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::search::scorer::{Scorer, ScorerEnum2, ScorerEnum3, ScorerEnum4, TwoPhaseState};
use crate::core::search::scorer_types::{LeafScorerTypes, ScorerTypes};
use crate::core::search::similarities_impl::similarities::SimilaritySimScorer;
use crate::core::search::term_query::TermQuerySimScorer;
use crate::core::search::term_scorer::TermScorer;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::search::two_phase_iterator::TwoPhaseIteratorEnum2;
use crate::core::search::wand_scorer::WANDScorer;
use crate::core::search::{
  abstract_knn_vector_query, abstract_vector_similarity_query, doc_values_rewrite_method,
  index_sort_sorted_numeric_doc_values_range_query, lru_query_cache,
};
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::doc_id_set_builder::DocIdSetBuilderIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::sandbox::search::term_automaton_scorer::TermAutomatonScorer;
use std::sync::Arc;

/// A concrete scorer shared by heterogeneous and recursive built-in queries.
/// Its private variants retain the actual implementation type.
pub struct BuiltinQueryScorer<ST: ScorerTypes> {
  inner: ScorerNode<ST>,
}

pub trait IntoQueryScorer<ST: ScorerTypes> {
  fn into_query_scorer(self) -> BuiltinQueryScorer<ST>;
}

macro_rules! scorer_nodes {
  ($( $(#[$attrs:meta])* $variant:ident: $ty:ty ),+ $(,)?) => {
    pub(crate) enum DefaultQueryBulkScorer<ST: ScorerTypes> {
      #[cfg(test)] Fixture(crate::core::search::weight::DefaultBulkScorer<Box<dyn crate::test_framework::core::search::test_scorer::TestScorer>>),
      $( $(#[$attrs])* $variant(crate::core::search::weight::DefaultBulkScorer<Box<$ty>>), )+
    }
    impl<ST: ScorerTypes> crate::core::search::bulk_scorer::BulkScorer for DefaultQueryBulkScorer<ST> {
      fn score<CLC: crate::core::search::leaf_collector::LeafCollector, AB: crate::core::util::bits::Bits + ?Sized>(&mut self, collector: &mut CLC, accept_docs: Option<&AB>, min: i32, max: i32) -> Result<i32> {
        match self { $( $(#[$attrs])* Self::$variant(scorer) => scorer.score(collector, accept_docs, min, max), )+ #[cfg(test)] Self::Fixture(scorer) => scorer.score(collector, accept_docs, min, max), }
      }
      fn cost(&mut self) -> Result<i64> { match self { $( $(#[$attrs])* Self::$variant(scorer) => scorer.cost(), )+ #[cfg(test)] Self::Fixture(scorer) => scorer.cost(), } }
      #[cfg(test)]
      fn kind(&self) -> crate::core::search::bulk_scorer::BulkScorerKind { crate::core::search::bulk_scorer::BulkScorerKind::Default }
    }
    impl<ST: ScorerTypes> BuiltinQueryScorer<ST> {
      pub(crate) fn into_default_bulk(self) -> DefaultQueryBulkScorer<ST> {
        match self.inner { $( $(#[$attrs])* ScorerNode::$variant(scorer) => DefaultQueryBulkScorer::$variant(crate::core::search::weight::DefaultBulkScorer::new(scorer)), )+ #[cfg(test)] ScorerNode::Fixture(scorer) => DefaultQueryBulkScorer::Fixture(crate::core::search::weight::DefaultBulkScorer::new(scorer)), }
      }
    }
    enum ScorerNode<ST: ScorerTypes> {
      #[cfg(test)] Fixture(Box<dyn crate::test_framework::core::search::test_scorer::TestScorer>),
      $( $(#[$attrs])* $variant(Box<$ty>), )+
    }

    $( $(#[$attrs])* impl<ST: ScorerTypes> IntoQueryScorer<ST> for $ty {
      fn into_query_scorer(self) -> BuiltinQueryScorer<ST> {
        BuiltinQueryScorer { inner: ScorerNode::$variant(Box::new(self)) }
      }
    } )+

    macro_rules! dispatch_scorer {
      ($node:expr, |$inner:ident| $body:expr) => {
        match $node { $( $(#[$attrs])* ScorerNode::$variant($inner) => $body, )+ #[cfg(test)] ScorerNode::Fixture($inner) => $body, }
      };
    }
  };
}

type OwnedSortedNumeric<ST> = SortedNumericDocValuesEnum3WithEmpty<
  <ST as ScorerTypes>::SortedNumeric,
  SingletonSortedNumericDocValues<<ST as ScorerTypes>::Numeric>,
>;
type OwnedSortedSet<ST> = SortedSetDocValuesEnum2<
  <ST as ScorerTypes>::SortedSet,
  SingletonSortedSetDocValues<SortedDocValuesWithEmpty<<ST as ScorerTypes>::Sorted>>,
>;
type SingletonNumeric<ST> = <OwnedSortedNumeric<ST> as SortedNumericDocValues>::NumericDocValues;
type SingletonSorted<ST> = <OwnedSortedSet<ST> as SortedSetDocValues>::SortedDocValues;
type NumericRangeTwoPhase<ST> = TwoPhaseIteratorEnum2<
  sorted_numeric_doc_values_range_query::TwoPhaseIterator3<SingletonNumeric<ST>>,
  sorted_numeric_doc_values_range_query::TwoPhaseIterator4<OwnedSortedNumeric<ST>>,
>;
type NumericSetTwoPhase<ST> = TwoPhaseIteratorEnum2<
  sorted_numeric_doc_values_set_query::TwoPhaseIterator1<SingletonNumeric<ST>>,
  sorted_numeric_doc_values_set_query::TwoPhaseIterator2<OwnedSortedNumeric<ST>>,
>;
type SortedRangeTwoPhase<ST> = TwoPhaseIteratorEnum2<
  sorted_set_doc_values_range_query::TwoPhaseIterator5<SingletonSorted<ST>>,
  sorted_set_doc_values_range_query::TwoPhaseIterator6<OwnedSortedSet<ST>>,
>;
type SortedTermSetTwoPhase<ST> = TwoPhaseIteratorEnum2<
  doc_values_rewrite_method::SingletonTermSetTwoPhaseIterator<SingletonSorted<ST>>,
  doc_values_rewrite_method::SortedSetTermSetTwoPhaseIterator<OwnedSortedSet<ST>>,
>;
type DistanceValues<ST> = NumericDocValuesEnum3WithEmpty<
  lat_lon_point_distance_feature_query::NumericDocValuesImpl<<ST as ScorerTypes>::SortedNumeric>,
  <ST as ScorerTypes>::Numeric,
>;
type FieldExistsIterator<ST> = DocIdSetIteratorEnum4<
  <ST as ScorerTypes>::ByteIterator,
  <ST as ScorerTypes>::FloatIterator,
  <ST as ScorerTypes>::Norm,
  DocIdSetIteratorEnum5<
    <ST as ScorerTypes>::Numeric,
    <ST as ScorerTypes>::Binary,
    <ST as ScorerTypes>::Sorted,
    <ST as ScorerTypes>::SortedNumeric,
    <ST as ScorerTypes>::SortedSet,
  >,
>;
type BlendedIterator<ST> = DisjunctionDISIApproximation<
  ConstantScoreScorer<
    DocIdSetIteratorEnum2<DocIdSetBuilderIterator, <ST as ScorerTypes>::Posting>,
    DummyTwoPhaseIterator,
  >,
>;

scorer_nodes! {
  Synonym: crate::core::search::synonym_query::SynonymScorer<ST>,
  SynonymTerm: crate::core::search::synonym_query::SynonymTermScorer<ST>,
  BoostedSynonymTerm: crate::core::search::synonym_query::FreqBoostTermScorer<ST>,

  Phrase: PhraseScorer<ImpactsEnumEnum2<ST::Impacts, SlowImpactsEnum<ST::Posting>>, Arc<SimScorerType>, ST::Norm>,
  MultiPhrase: PhraseScorer<SlowImpactsEnum<MultiPhrasePostingsEnum<ST::Posting>>, Arc<SimScorerType>, ST::Norm>,
  SpatialSet: ConstantScoreScorer<DocIdSetIteratorEnum2<EmptyDISI, BitSetIterator<FixedBitSet>>, DummyTwoPhaseIterator>,
  CachedSet: ConstantScoreScorer<lru_query_cache::CacheAndCountDISI, DummyTwoPhaseIterator>,
  CachedSetOrEmpty: ConstantScoreScorer<lru_query_cache::DISI, DummyTwoPhaseIterator>,
  SortedRangeIterator: ConstantScoreScorer<sorted_numeric_doc_values_range_query::DISI, DummyTwoPhaseIterator>,
  IndexSortedIterator: ConstantScoreScorer<index_sort_sorted_numeric_doc_values_range_query::IteratorAndCountDisi<SingletonNumeric<ST>>, DummyTwoPhaseIterator>,
  FieldExists: ConstantScoreScorer<FieldExistsIterator<ST>, DummyTwoPhaseIterator>,
  BlendedTerms: ConstantScoreScorer<BlendedIterator<ST>, DummyTwoPhaseIterator>,
  BinaryRange: ConstantScoreScorer<DummyDISI, binary_range_field_range_query::BinaryRangeFieldRangeTPI<BinaryDocValuesWithEmpty<ST::Binary>>>,
  LatLonBox: ConstantScoreScorer<DummyDISI, lat_lon_doc_values_box_query::LatLonDocValuesBoxTwoPhaseIterator<ST::SortedNumeric>>,
  LatLonShape: ConstantScoreScorer<DummyDISI, lat_lon_doc_values_query::LatLonDocValuesTwoPhaseIterator<ST::SortedNumeric, Arc<LatLonGeometryType<LatLonGeometryEnumComponent2D>>>>,
  XYShape: ConstantScoreScorer<DummyDISI, xy_doc_values_point_in_geometry_query::XYDocValuesPointInGeometryTPI<ST::SortedNumeric, Arc<XYGeometryType<<XYGeometryEnum as Geometry>::Component2D>>>>,
  NumericRange: ConstantScoreScorer<DummyDISI, NumericRangeTwoPhase<ST>>,
  NumericRangeWithSkip: ConstantScoreScorer<DummyDISI, DocValuesRangeIterator<NumericRangeTwoPhase<ST>, ST::Skipper>>,
  NumericSet: ConstantScoreScorer<DummyDISI, NumericSetTwoPhase<ST>>,
  SortedRange: ConstantScoreScorer<DummyDISI, SortedRangeTwoPhase<ST>>,
  SortedRangeWithSkip: ConstantScoreScorer<DummyDISI, DocValuesRangeIterator<SortedRangeTwoPhase<ST>, ST::Skipper>>,
  SortedTerms: ConstantScoreScorer<DummyDISI, SortedTermSetTwoPhase<ST>>,
  SortedTermsWithSkip: ConstantScoreScorer<DummyDISI, DocValuesRangeIterator<SortedTermSetTwoPhase<ST>, ST::Skipper>>,
  DistanceFeature: lat_lon_point_distance_feature_query::DistanceScorer<ST::PointValues, DistanceValues<ST>>,
  VectorMatches: abstract_knn_vector_query::ScorerImpl,
  FilteredVectors: abstract_vector_similarity_query::ScorerImpl<abstract_vector_similarity_query::FilteredDocIdSetIteratorImpl1<crate::core::search::vector_scorer::VectorScorerEnum2<ST::ByteScorer, ST::FloatScorer>, crate::core::util::bit_set::SparseFixedBitSetBitSet>>,
  VectorScores: abstract_vector_similarity_query::ScorerImpl<abstract_vector_similarity_query::DocsIteratorImpl>,
  TermAutomaton: TermAutomatonScorer<ST::Posting, SimilaritySimScorer, ST::Norm>,
  Term: TermScorer<ST::Posting, Arc<TermQuerySimScorer>, ST::Norm, ST::Impacts>,
  MatchAll: ConstantScoreScorer<AllDISI, DummyTwoPhaseIterator>,
  Empty: ConstantScoreScorer<EmptyDISI, DummyTwoPhaseIterator>,
  BuiltSet: ConstantScoreScorer<DocIdSetBuilderIterator, DummyTwoPhaseIterator>,
  BitSet: ConstantScoreScorer<BitSetIterator<FixedBitSet>, DummyTwoPhaseIterator>,
  ConstantIterator: ConstantScoreScorer<BuiltinQueryScorer<ST>, DummyTwoPhaseIterator>,
  ConstantTwoPhase: ConstantScoreScorer<DummyDISI, QueryOwnedTwoPhase<ST>>,
  Conjunction: ConjunctionScorer<BuiltinQueryScorer<ST>>,
  BlockMaxConjunction: BlockMaxConjunctionScorer<BuiltinQueryScorer<ST>>,
  DisjunctionSum: DisjunctionScorer<BuiltinQueryScorer<ST>, DisjunctionSumScorer>,
  DisjunctionMax: DisjunctionScorer<BuiltinQueryScorer<ST>, DisjunctionMaxScorer>,
  Wand: WANDScorer<BuiltinQueryScorer<ST>>,
  RequiredOptional: ReqOptSumScorer<BuiltinQueryScorer<ST>, BuiltinQueryScorer<ST>>,
  RequiredExcluded: ReqExclScorer<BuiltinQueryScorer<ST>, BuiltinQueryScorer<ST>>,
  Filter: FilterScorerImpl<BuiltinQueryScorer<ST>>,
}

impl<ST: ScorerTypes> BuiltinQueryScorer<ST> {
  pub fn new<S: IntoQueryScorer<ST>>(scorer: S) -> Self {
    scorer.into_query_scorer()
  }

  fn has_two_phase_view(&self) -> bool {
    dispatch_scorer!(&self.inner, |scorer| scorer.two_phase_iterator().is_some())
  }

  fn matches(&mut self) -> Result<bool> {
    dispatch_scorer!(&mut self.inner, |scorer| {
      #[allow(unused_mut)]
      let mut iterator = scorer
        .two_phase_iterator_mut()
        .ok_or_else(|| LuceneError::illegal_state("scorer has no two-phase iterator"))?;
      iterator.matches()
    })
  }

  fn match_cost(&self) -> f32 {
    dispatch_scorer!(&self.inner, |scorer| scorer
      .two_phase_iterator()
      .map_or(0.0, |iterator| iterator.match_cost()))
  }
}

#[cfg(test)]
impl<ST: ScorerTypes> IntoQueryScorer<ST>
  for Box<dyn crate::test_framework::core::search::test_scorer::TestScorer>
{
  fn into_query_scorer(self) -> BuiltinQueryScorer<ST> {
    BuiltinQueryScorer {
      inner: ScorerNode::Fixture(self),
    }
  }
}

impl<ST: ScorerTypes> IntoQueryScorer<ST> for BuiltinQueryScorer<ST> {
  fn into_query_scorer(self) -> BuiltinQueryScorer<ST> {
    self
  }
}

impl<ST: ScorerTypes, S: IntoQueryScorer<ST>> IntoQueryScorer<ST> for Box<S> {
  fn into_query_scorer(self) -> BuiltinQueryScorer<ST> {
    (*self).into_query_scorer()
  }
}

macro_rules! convert_scorer_enum {
  ($name:ident, $( $variant:ident: $ty:ident ),+) => {
    impl<ST: ScorerTypes, $( $ty: Scorer + IntoQueryScorer<ST> ),+> IntoQueryScorer<ST> for $name<$( $ty ),+> {
      fn into_query_scorer(self) -> BuiltinQueryScorer<ST> {
        match self { $( Self::$variant(scorer) => scorer.into_query_scorer(), )+ }
      }
    }
  };
}
convert_scorer_enum!(ScorerEnum2, A: S1, B: S2);
convert_scorer_enum!(ScorerEnum3, A: S1, B: S2, C: S3);
convert_scorer_enum!(ScorerEnum4, A: S1, B: S2, C: S3, D: S4);

impl<ST: ScorerTypes> DocIdSetIteratorExtensions for BuiltinQueryScorer<ST> {
  fn get_fixed_bit_set(&self) -> Option<&FixedBitSet> {
    dispatch_scorer!(&self.inner, |scorer| scorer.get_fixed_bit_set())
  }
  fn get_sparse_fixed_bit_set(
    &self,
  ) -> Option<&crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet> {
    dispatch_scorer!(&self.inner, |scorer| scorer.get_sparse_fixed_bit_set())
  }
  fn get_doc_base_fixed_bit_set(&self) -> Option<(usize, &FixedBitSet)> {
    dispatch_scorer!(&self.inner, |scorer| scorer.get_doc_base_fixed_bit_set())
  }
}
impl<ST: ScorerTypes> BitSetIteratorAccess for BuiltinQueryScorer<ST> {
  fn is_bit_iter(&self) -> bool {
    dispatch_scorer!(&self.inner, |scorer| scorer.is_bit_iter())
  }
  fn get(&self, index: usize) -> Result<bool> {
    dispatch_scorer!(&self.inner, |scorer| scorer.get(index))
  }
  fn set_doc_id(&mut self, doc: i32) -> Result<()> {
    dispatch_scorer!(&mut self.inner, |scorer| scorer.set_doc_id(doc))
  }
  fn bit_set_length(&self) -> Result<usize> {
    dispatch_scorer!(&self.inner, |scorer| scorer.bit_set_length())
  }
}
impl<ST: ScorerTypes> DocIdSetIterator for BuiltinQueryScorer<ST> {
  fn doc_id(&self) -> i32 {
    dispatch_scorer!(&self.inner, |scorer| scorer.doc_id())
  }
  fn next_doc(&mut self) -> Result<i32> {
    dispatch_scorer!(&mut self.inner, |scorer| scorer.next_doc())
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    dispatch_scorer!(&mut self.inner, |scorer| scorer.advance(target))
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    dispatch_scorer!(&mut self.inner, |scorer| scorer.slow_advance(target))
  }
  fn cost(&self) -> Result<i64> {
    dispatch_scorer!(&self.inner, |scorer| DocIdSetIterator::cost(scorer))
  }
}
impl<ST: ScorerTypes> Scorable for BuiltinQueryScorer<ST> {
  type Child<'a>
    = QueryChildScorable<'a, ST>
  where
    Self: 'a;

  fn score(&mut self) -> Result<f32> {
    dispatch_scorer!(&mut self.inner, |scorer| scorer.score())
  }
  fn smoothing_score(&mut self, doc: i32) -> Result<f32> {
    dispatch_scorer!(&mut self.inner, |scorer| scorer.smoothing_score(doc))
  }
  fn set_min_competitive_score(&mut self, min: f32) -> Result<()> {
    dispatch_scorer!(&mut self.inner, |scorer| scorer
      .set_min_competitive_score(min))
  }
  fn cost(&self) -> Result<i64> {
    dispatch_scorer!(&self.inner, |scorer| Scorable::cost(scorer))
  }
  fn get_children(&mut self) -> Result<Vec<ChildScorable<Self::Child<'_>>>> {
    dispatch_scorer!(&mut self.inner, |scorer| scorer.get_children().map(
      |children| children
        .into_iter()
        .map(|child| ChildScorable {
          child: child.child.into_query_child(),
          relationship: child.relationship
        })
        .collect()
    ))
  }
  #[cfg(test)]
  fn as_any(&mut self) -> &mut dyn std::any::Any {
    self
  }
}
impl<ST: ScorerTypes> crate::core::search::scorable::FixedScore for BuiltinQueryScorer<ST> {}
impl<ST: ScorerTypes> Scorer for BuiltinQueryScorer<ST> {
  type Iterator = Self;
  type OwnedTwoPhase = QueryOwnedTwoPhase<ST>;
  type ApproximationRef<'a>
    = QueryApproximationRef<'a, ST>
  where
    Self: 'a;
  type ApproximationMut<'a>
    = QueryApproximationMut<'a, ST>
  where
    Self: 'a;
  type TwoPhaseRef<'a>
    = QueryTwoPhaseRef<'a, ST>
  where
    Self: 'a;
  type TwoPhaseMut<'a>
    = QueryTwoPhaseMut<'a, ST>
  where
    Self: 'a;
  fn scoring_doc_id(&mut self) -> Result<i32> {
    dispatch_scorer!(&mut self.inner, |scorer| scorer.scoring_doc_id())
  }
  fn take_iterator(self) -> Self::Iterator {
    self
  }
  fn take_two_phase_iterator(self) -> Option<Self::OwnedTwoPhase> {
    self
      .has_two_phase_view()
      .then_some(QueryOwnedTwoPhase { owner: self })
  }
  fn approximation(&self) -> Self::ApproximationRef<'_> {
    QueryApproximationRef { owner: self }
  }
  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    QueryApproximationMut { owner: self }
  }
  fn two_phase_iterator(&self) -> Option<Self::TwoPhaseRef<'_>> {
    self
      .has_two_phase_view()
      .then_some(QueryTwoPhaseRef { owner: self })
  }
  fn two_phase_iterator_mut(&mut self) -> Option<Self::TwoPhaseMut<'_>> {
    self
      .has_two_phase_view()
      .then_some(QueryTwoPhaseMut { owner: self })
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    dispatch_scorer!(&self.inner, |scorer| scorer.has_two_phase_iterator())
  }
  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    dispatch_scorer!(&mut self.inner, |scorer| scorer.advance_shallow(target))
  }
  fn default_advance_shallow(&mut self, target: i32) -> Result<i32> {
    dispatch_scorer!(&mut self.inner, |scorer| scorer
      .default_advance_shallow(target))
  }
  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    dispatch_scorer!(&mut self.inner, |scorer| scorer.get_max_score(upto))
  }
  fn default_cost(&mut self) -> Result<i64> {
    dispatch_scorer!(&mut self.inner, |scorer| scorer.default_cost())
  }
  #[cfg(test)]
  fn kind(&self) -> crate::core::search::scorer::ScorerKind {
    dispatch_scorer!(&self.inner, |scorer| scorer.kind())
  }
}

pub struct QueryApproximationRef<'a, ST: ScorerTypes> {
  owner: &'a BuiltinQueryScorer<ST>,
}
pub struct QueryApproximationMut<'a, ST: ScorerTypes> {
  owner: &'a mut BuiltinQueryScorer<ST>,
}
macro_rules! approximation_view {
  ($view:ident, $( $setter:item )*) => {
    impl<ST: ScorerTypes> DocIdSetIteratorExtensions for $view<'_, ST> {
      fn get_fixed_bit_set(&self) -> Option<&FixedBitSet> { self.owner.get_fixed_bit_set() }
      fn get_sparse_fixed_bit_set(&self) -> Option<&crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet> { self.owner.get_sparse_fixed_bit_set() }
      fn get_doc_base_fixed_bit_set(&self) -> Option<(usize, &FixedBitSet)> { self.owner.get_doc_base_fixed_bit_set() }
    }
    impl<ST: ScorerTypes> BitSetIteratorAccess for $view<'_, ST> {
      $( $setter )*
      fn is_bit_iter(&self) -> bool {
        dispatch_scorer!(&self.owner.inner, |scorer| scorer
          .approximation()
          .is_bit_iter())
      }
      fn get(&self, index: usize) -> Result<bool> {
        dispatch_scorer!(&self.owner.inner, |scorer| scorer
          .approximation()
          .get(index))
      }
      fn bit_set_length(&self) -> Result<usize> {
        dispatch_scorer!(&self.owner.inner, |scorer| scorer
          .approximation()
          .bit_set_length())
      }
    }
  };
}
approximation_view!(QueryApproximationRef,);
approximation_view!(
  QueryApproximationMut,
  fn set_doc_id(&mut self, doc: i32) -> Result<()> {
    dispatch_scorer!(&mut self.owner.inner, |scorer| scorer
      .approximation_mut()
      .set_doc_id(doc))
  }
);
impl<ST: ScorerTypes> DocIdSetIterator for QueryApproximationRef<'_, ST> {
  fn doc_id(&self) -> i32 {
    dispatch_scorer!(&self.owner.inner, |scorer| scorer.approximation().doc_id())
  }
  fn cost(&self) -> Result<i64> {
    dispatch_scorer!(&self.owner.inner, |scorer| DocIdSetIterator::cost(
      &scorer.approximation()
    ))
  }
  fn next_doc(&mut self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(
      "immutable approximation",
    ))
  }
}
impl<ST: ScorerTypes> DocIdSetIterator for QueryApproximationMut<'_, ST> {
  fn doc_id(&self) -> i32 {
    dispatch_scorer!(&self.owner.inner, |scorer| scorer.approximation().doc_id())
  }
  fn cost(&self) -> Result<i64> {
    dispatch_scorer!(&self.owner.inner, |scorer| DocIdSetIterator::cost(
      &scorer.approximation()
    ))
  }
  fn next_doc(&mut self) -> Result<i32> {
    dispatch_scorer!(&mut self.owner.inner, |scorer| scorer
      .approximation_mut()
      .next_doc())
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    dispatch_scorer!(&mut self.owner.inner, |scorer| scorer
      .approximation_mut()
      .advance(target))
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    dispatch_scorer!(&mut self.owner.inner, |scorer| scorer
      .approximation_mut()
      .slow_advance(target))
  }
}

pub struct QueryTwoPhaseRef<'a, ST: ScorerTypes> {
  owner: &'a BuiltinQueryScorer<ST>,
}
pub struct QueryTwoPhaseMut<'a, ST: ScorerTypes> {
  owner: &'a mut BuiltinQueryScorer<ST>,
}
pub struct QueryOwnedTwoPhase<ST: ScorerTypes> {
  owner: BuiltinQueryScorer<ST>,
}
impl<ST: ScorerTypes> TwoPhaseIterator for QueryTwoPhaseRef<'_, ST> {
  type ApproximationRef<'a>
    = QueryApproximationRef<'a, ST>
  where
    Self: 'a;
  type ApproximationMut<'a>
    = QueryApproximationRef<'a, ST>
  where
    Self: 'a;
  fn approximation(&self) -> Self::ApproximationRef<'_> {
    self.owner.approximation()
  }
  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    self.owner.approximation()
  }
  fn matches(&mut self) -> Result<bool> {
    Err(LuceneError::unsupported_operation(
      "immutable two-phase iterator",
    ))
  }
  fn match_cost(&self) -> f32 {
    self.owner.match_cost()
  }
}
macro_rules! mutable_two_phase {
  ($ty:ty) => {
    impl<ST: ScorerTypes> TwoPhaseIterator for $ty {
      type ApproximationRef<'a>
        = QueryApproximationRef<'a, ST>
      where
        Self: 'a;
      type ApproximationMut<'a>
        = QueryApproximationMut<'a, ST>
      where
        Self: 'a;
      fn approximation(&self) -> Self::ApproximationRef<'_> {
        self.owner.approximation()
      }
      fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
        self.owner.approximation_mut()
      }
      fn matches(&mut self) -> Result<bool> {
        self.owner.matches()
      }
      fn match_cost(&self) -> f32 {
        self.owner.match_cost()
      }
    }
  };
}
mutable_two_phase!(QueryTwoPhaseMut<'_, ST>);
mutable_two_phase!(QueryOwnedTwoPhase<ST>);

/// Built-in query scorer specialized by the reader's owned execution types.
pub type QueryScorer<LR> = BuiltinQueryScorer<LeafScorerTypes<LR>>;

#[cfg(test)]
impl<ST: ScorerTypes> BuiltinQueryScorer<ST> {
  pub(crate) fn for_test<S: Scorer + 'static>(scorer: S) -> Self {
    let scorer: Box<dyn crate::test_framework::core::search::test_scorer::TestScorer> =
      Box::new(scorer);
    scorer.into_query_scorer()
  }
}

/// Borrowed children of built-in scorers retain their original mutable owners.
pub enum QueryChildScorable<'a, ST: ScorerTypes> {
  Scorer(&'a mut BuiltinQueryScorer<ST>),
  Empty(crate::core::search::dummy::dummy_scorable::DummyScorable),
  #[cfg(test)]
  Fixture(Box<dyn crate::test_framework::core::search::test_scorer::TestScorable + 'a>),
}
impl<'a, ST: ScorerTypes> Scorable for QueryChildScorable<'a, ST> {
  type Child<'b>
    = QueryChildScorable<'b, ST>
  where
    Self: 'b;
  fn score(&mut self) -> Result<f32> {
    match self {
      Self::Scorer(s) => s.score(),
      Self::Empty(s) => s.score(),
      #[cfg(test)]
      Self::Fixture(s) => s.score(),
    }
  }
  fn smoothing_score(&mut self, doc: i32) -> Result<f32> {
    match self {
      Self::Scorer(s) => s.smoothing_score(doc),
      Self::Empty(s) => s.smoothing_score(doc),
      #[cfg(test)]
      Self::Fixture(s) => s.smoothing_score(doc),
    }
  }
  fn set_min_competitive_score(&mut self, score: f32) -> Result<()> {
    match self {
      Self::Scorer(s) => s.set_min_competitive_score(score),
      Self::Empty(s) => s.set_min_competitive_score(score),
      #[cfg(test)]
      Self::Fixture(s) => s.set_min_competitive_score(score),
    }
  }
  fn cost(&self) -> Result<i64> {
    match self {
      Self::Scorer(s) => Scorable::cost(s),
      Self::Empty(s) => Scorable::cost(s),
      #[cfg(test)]
      Self::Fixture(s) => Scorable::cost(s),
    }
  }
  fn get_children(&mut self) -> Result<Vec<ChildScorable<Self::Child<'_>>>> {
    match self {
      Self::Scorer(s) => s.get_children(),
      Self::Empty(_) => Ok(vec![]),
      #[cfg(test)]
      Self::Fixture(s) => s.get_children().map(|children| {
        children
          .into_iter()
          .map(|child| ChildScorable {
            child: QueryChildScorable::Fixture(child.child),
            relationship: child.relationship,
          })
          .collect()
      }),
    }
  }
  #[cfg(test)]
  fn as_any(&mut self) -> &mut dyn std::any::Any {
    match self {
      Self::Scorer(s) => s.as_any(),
      Self::Empty(s) => s.as_any(),
      Self::Fixture(s) => s.as_any(),
    }
  }
}
impl<ST: ScorerTypes> crate::core::search::scorable::FixedScore for QueryChildScorable<'_, ST> {
  fn set_score(&mut self, score: f32) -> Result<()> {
    match self {
      Self::Scorer(s) => s.set_score(score),
      Self::Empty(s) => s.set_score(score),
      #[cfg(test)]
      Self::Fixture(s) => s.set_score(score),
    }
  }
}
trait IntoQueryChild<'a, ST: ScorerTypes> {
  fn into_query_child(self) -> QueryChildScorable<'a, ST>;
}
impl<'a, ST: ScorerTypes> IntoQueryChild<'a, ST> for &'a mut BuiltinQueryScorer<ST> {
  fn into_query_child(self) -> QueryChildScorable<'a, ST> {
    QueryChildScorable::Scorer(self)
  }
}
impl<'a, ST: ScorerTypes> IntoQueryChild<'a, ST>
  for crate::core::search::dummy::dummy_scorable::DummyScorable
{
  fn into_query_child(self) -> QueryChildScorable<'a, ST> {
    QueryChildScorable::Empty(self)
  }
}
impl<'a, ST: ScorerTypes> IntoQueryChild<'a, ST> for QueryChildScorable<'a, ST> {
  fn into_query_child(self) -> QueryChildScorable<'a, ST> {
    self
  }
}
#[cfg(test)]
impl<'a, ST: ScorerTypes> IntoQueryChild<'a, ST>
  for Box<dyn crate::test_framework::core::search::test_scorer::TestScorable + 'a>
{
  fn into_query_child(self) -> QueryChildScorable<'a, ST> {
    QueryChildScorable::Fixture(self)
  }
}
macro_rules! either_child {
 ($name:ident { $( $v:ident:$t:ident ),+ }) => {
  impl<'a, ST: ScorerTypes, $( $t: IntoQueryChild<'a, ST> ),+> IntoQueryChild<'a, ST> for crate::core::search::scorable::$name<$( $t ),+> {
   fn into_query_child(self) -> QueryChildScorable<'a, ST> { match self { $( Self::$v(child) => child.into_query_child(), )+ } }
  }
 };
}
either_child!(ScorableEnum2 { A: A, B: B });
either_child!(ScorableEnum3 { A: A, B: B, C: C });
either_child!(ScorableEnum4 {
  A: A,
  B: B,
  C: C,
  D: D
});

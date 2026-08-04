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
use crate::core::document::binary_range_field_range_query::BinaryRangeFieldRangeQuery;
use crate::core::document::lat_lon_doc_values_box_query::LatLonDocValuesBoxQuery;
use crate::core::document::lat_lon_doc_values_query::LatLonDocValuesQuery;
use crate::core::document::lat_lon_point_distance_feature_query::LatLonPointDistanceFeatureQuery;
use crate::core::document::lat_lon_point_distance_query::LatLonPointDistanceQuery;
use crate::core::document::range_field_query::RangeFieldQuery;
use crate::core::document::sorted_numeric_doc_values_range_query::SortedNumericDocValuesRangeQuery;
use crate::core::document::sorted_numeric_doc_values_set_query::SortedNumericDocValuesSetQuery;
use crate::core::document::sorted_set_doc_values_range_query::SortedSetDocValuesRangeQuery;
use crate::core::document::xy_doc_values_point_in_geometry_query::XYDocValuesPointInGeometryQuery;
use crate::core::document::xy_point_in_geometry_query::XYPointInGeometryQuery;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::search::boolean_query::BooleanQuery;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::doc_values_rewrite_method::MultiTermQueryDocValuesWrapper;
use crate::core::search::dummy::dummy_query::DummyQuery;
use crate::core::search::field_exists_query::FieldExistsQuery;
use crate::core::search::float_vector_similarity_query::FloatVectorSimilarityQuery;
use crate::core::search::index_searcher::IndexSearcher;

use crate::core::document::lat_lon_point_query::LatLonPointQuery;
use crate::core::search::abstract_knn_vector_query::DocAndScoreQuery;
use crate::core::search::automaton_query::AutomatonQuery;
use crate::core::search::blended_term_query::BlendedTermQuery;
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::byte_vector_similarity_query::ByteVectorSimilarityQuery;
use crate::core::search::disjunction_max_query::DisjunctionMaxQuery;
use crate::core::search::fuzzy_query::FuzzyQuery;
use crate::core::search::index_or_doc_values_query::IndexOrDocValuesQuery;
use crate::core::search::index_sort_sorted_numeric_doc_values_range_query::IndexSortSortedNumericDocValuesRangeQuery;
use crate::core::search::knn_byte_vector_query::KnnByteVectorQuery;
use crate::core::search::knn_float_vector_query::KnnFloatVectorQuery;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::matches::Matches;
use crate::core::search::matches_iterator::MatchesIterator;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::multi_phrase_query::MultiPhraseQuery;
use crate::core::search::multi_term_query::MultiTermQuerySet;
use crate::core::search::multi_term_query_constant_score_blended_wrapper::MultiTermQueryConstantScoreBlendedWrapper;
use crate::core::search::multi_term_query_constant_score_wrapper::MultiTermQueryConstantScoreWrapper;
use crate::core::search::n_gram_phrase_query::NGramPhraseQuery;
use crate::core::search::named_matches::{NamedMatches, NamedQuery};
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::point_in_set_query::PointInSetQuery;
use crate::core::search::point_range_query::PointRangeQuery;
use crate::core::search::prefix_query::PrefixQuery;
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::regexp_query::RegexpQuery;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::Scorer;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::synonym_query::SynonymQuery;
use crate::core::search::term_in_set_query::TermInSetQuery;
use crate::core::search::term_query::TermQuery;
use crate::core::search::term_range_query::TermRangeQuery;
use crate::core::search::weight::Weight;
use crate::core::search::wildcard_query::WildcardQuery;
use crate::core::util::accountable::Accountable;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::impl_from_for_enum;
#[cfg(test)]
use crate::test_framework::core::search::asserting_query::AssertingQuery;
#[cfg(test)]
use crate::test_framework::core::search::base_vector_similarity_query_test_case::CountingQuery;
#[cfg(test)]
use crate::test_framework::core::search::block_score_query_wrapper::BlockScoreQueryWrapper;
#[cfg(test)]
use crate::test_framework::core::search::multi_term::{
  BoostCheckingQuery, DumbPrefixQuery, DumbRegexpQuery,
};
#[cfg(test)]
use crate::test_framework::core::search::query::AssertNeedsScores;
#[cfg(test)]
use crate::test_framework::core::search::query::BitSetQuery;
#[cfg(test)]
use crate::test_framework::core::search::query::BrokenExplainTermQuery;
#[cfg(test)]
use crate::test_framework::core::search::query::CrazyMustUseBulkScorerQuery;
#[cfg(test)]
use crate::test_framework::core::search::query::DummyQuery1;
#[cfg(test)]
use crate::test_framework::core::search::query::RandomQuery;
#[cfg(test)]
use crate::test_framework::core::search::query::TestRewriteQuery;
#[cfg(test)]
use crate::test_framework::core::search::query::{DVCacheQuery, TestLRUQuery};
#[cfg(test)]
use crate::test_framework::core::search::query::{MaxScoreWrapperQuery, WANDScorerQuery};
#[cfg(test)]
use crate::test_framework::core::search::random_approximation_query::RandomApproximationQuery;
use std::cmp::PartialEq;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub type QueryWeight<IRC> = Box<dyn Weight<IRC, ScorerSupplier = QueryWeightSs<IRC>> + Send + Sync>;
pub enum QueryWeightMatches<'a> {
  MatchWithNoTerms(MatchWithNoTerms),
  NamedMatches(Box<NamedMatches<'a>>),
  Matches(Box<dyn Matches + 'a>),
}
impl Matches for QueryWeightMatches<'_> {
  fn get_matches(&self, field: &str) -> Result<Option<QueryWeightMatchesIterator<'_>>> {
    match self {
      QueryWeightMatches::MatchWithNoTerms(matches) => matches.get_matches(field),
      QueryWeightMatches::NamedMatches(matches) => matches.get_matches(field),
      QueryWeightMatches::Matches(matches) => matches.get_matches(field),
    }
  }

  fn get_sub_matches(&self) -> Vec<&QueryWeightMatches<'_>> {
    match self {
      QueryWeightMatches::MatchWithNoTerms(matches) => matches.get_sub_matches(),
      QueryWeightMatches::NamedMatches(matches) => matches.get_sub_matches(),
      QueryWeightMatches::Matches(matches) => matches.get_sub_matches(),
    }
  }

  fn field(&self) -> &[String] {
    match self {
      QueryWeightMatches::MatchWithNoTerms(matches) => matches.field(),
      QueryWeightMatches::NamedMatches(matches) => matches.field(),
      QueryWeightMatches::Matches(matches) => matches.field(),
    }
  }
}
pub type QueryWeightMatchesIterator<'a> = Box<dyn MatchesIterator + 'a>;
pub type QueryWeightSs<IRC> =
  Box<dyn ScorerSupplier<IRC, BulkScorer = QueryWeightSsBulkScorer, Scorer = QueryWeightSsScorer>>;
pub type QueryWeightSsBulkScorer = Box<dyn BulkScorer>;
pub type QueryWeightSsScorer = Box<dyn Scorer>;
macro_rules! impl_into_box_query {
    ($($ty:ty),* $(,)?) => {
        $(
            impl IntoBoxQuery for $ty {
                fn into_box_query(self) -> Box<Query> {
                    Box::new(self.into())
                }
            }
        )*
    };
}
macro_rules! dispatch_query {
  ($self:expr, |$inner:ident| $body:expr) => {{
    match $self {
      Query::BinaryRangeFieldRange($inner) => $body,
      Query::BlendedTerm($inner) => $body,
      Query::Boolean($inner) => $body,
      Query::Boost($inner) => $body,
      Query::ByteVectorSimilarity($inner) => $body,
      Query::ConstantScore($inner) => $body,
      Query::DisjunctionMax($inner) => $body,
      Query::DocAndScore($inner) => $body,
      Query::Dummy($inner) => $body,
      Query::FieldExists($inner) => $body,
      Query::FloatVectorSimilarity($inner) => $body,
      Query::IndexOrDocValues($inner) => $body,
      Query::IndexSortSortedNumericDocValuesRange($inner) => $body,
      Query::KnnByteVector($inner) => $body,
      Query::KnnFloatVector($inner) => $body,
      Query::LatLonDocValues($inner) => $body,
      Query::LatLonDocValuesBox($inner) => $body,
      Query::LatLonPoint($inner) => $body,
      Query::LatLonPointDistance($inner) => $body,
      Query::LatLonPointDistanceFeature($inner) => $body,
      Query::MatchAllDocs($inner) => $body,
      Query::MatchNoDocs($inner) => $body,
      Query::Named($inner) => $body,
      Query::MultiPhrase($inner) => $body,
      Query::MultiTermQuery($inner) => $body,
      Query::MultiTermQueryDocValuesWrapper($inner) => $body,
      Query::MultiTermQueryConstantScoreBlendedWrapper($inner) => $body,
      Query::MultiTermQueryConstantScoreWrapper($inner) => $body,
      Query::NGramPhrase($inner) => $body,
      Query::Phrase($inner) => $body,
      Query::PointInSet($inner) => $body,
      Query::PointRange($inner) => $body,
      Query::RangeField($inner) => $body,
      Query::SortedNumericDocValuesRange($inner) => $body,
      Query::SortedNumericDocValuesSet($inner) => $body,
      Query::SortedSetDocValuesRange($inner) => $body,
      Query::Synonym($inner) => $body,
      Query::Term($inner) => $body,
      Query::XYDocValuesPointInGeometry($inner) => $body,
      Query::XYPointInGeometry($inner) => $body,
      #[cfg(test)]
      Query::Asserting($inner) => $body,
      #[cfg(test)]
      Query::AssertNeedsScores($inner) => $body,
      #[cfg(test)]
      Query::BitSet($inner) => $body,
      #[cfg(test)]
      Query::BlockScoreQueryWrapper($inner) => $body,
      #[cfg(test)]
      Query::BrokenExplainTerm($inner) => $body,
      #[cfg(test)]
      Query::Counting($inner) => $body,
      #[cfg(test)]
      Query::CrazyMustUseBulkScorer($inner) => $body,
      #[cfg(test)]
      Query::Dummy1($inner) => $body,
      #[cfg(test)]
      Query::TestLRU($inner) => $body,
      #[cfg(test)]
      Query::DVCache($inner) => $body,
      #[cfg(test)]
      Query::MaxScoreWrapper($inner) => $body,
      #[cfg(test)]
      Query::Random($inner) => $body,
      #[cfg(test)]
      Query::RandomApproximation($inner) => $body,
      #[cfg(test)]
      Query::TestRewrite($inner) => $body,
      #[cfg(test)]
      Query::WANDScorer($inner) => $body,
    }
  }};
}
impl_from_for_enum!(
    Query,
    BinaryRangeFieldRangeQuery => BinaryRangeFieldRange,
    BlendedTermQuery=> BlendedTerm,
    BooleanQuery => Boolean,
    BoostQuery => Boost,
    ByteVectorSimilarityQuery => ByteVectorSimilarity,
    ConstantScoreQuery => ConstantScore,
    DisjunctionMaxQuery => DisjunctionMax,
    DocAndScoreQuery => DocAndScore,
    DummyQuery => Dummy,
    FieldExistsQuery => FieldExists,
    FloatVectorSimilarityQuery => FloatVectorSimilarity,
    IndexOrDocValuesQuery => IndexOrDocValues,
    IndexSortSortedNumericDocValuesRangeQuery => IndexSortSortedNumericDocValuesRange,
    KnnByteVectorQuery => KnnByteVector,
    KnnFloatVectorQuery => KnnFloatVector,
    LatLonDocValuesBoxQuery => LatLonDocValuesBox,
    LatLonDocValuesQuery => LatLonDocValues,
    LatLonPointDistanceFeatureQuery => LatLonPointDistanceFeature,
    LatLonPointDistanceQuery => LatLonPointDistance,
    LatLonPointQuery=> LatLonPoint,
    MatchAllDocsQuery => MatchAllDocs,
    MatchNoDocsQuery => MatchNoDocs,
    NamedQuery => Named,
    MultiTermQuerySet => MultiTermQuery,
    MultiPhraseQuery=> MultiPhrase,
    MultiTermQueryDocValuesWrapper => MultiTermQueryDocValuesWrapper,
    MultiTermQueryConstantScoreBlendedWrapper => MultiTermQueryConstantScoreBlendedWrapper,
    MultiTermQueryConstantScoreWrapper => MultiTermQueryConstantScoreWrapper,
    NGramPhraseQuery=> NGramPhrase,
    PhraseQuery=> Phrase,
    PointInSetQuery => PointInSet,
    PointRangeQuery => PointRange,
    RangeFieldQuery => RangeField,
    SortedNumericDocValuesRangeQuery => SortedNumericDocValuesRange,
    SortedNumericDocValuesSetQuery => SortedNumericDocValuesSet,
    SortedSetDocValuesRangeQuery => SortedSetDocValuesRange,
    SynonymQuery => Synonym,
    TermQuery => Term,
    XYDocValuesPointInGeometryQuery => XYDocValuesPointInGeometry,
    XYPointInGeometryQuery => XYPointInGeometry,
);
#[cfg(test)]
impl_from_for_enum!(
    Query,
    AssertingQuery => Asserting,
    AssertNeedsScores => AssertNeedsScores,
    BitSetQuery => BitSet,
    BlockScoreQueryWrapper => BlockScoreQueryWrapper,
    BrokenExplainTermQuery => BrokenExplainTerm,
    CountingQuery => Counting,
    CrazyMustUseBulkScorerQuery => CrazyMustUseBulkScorer,
    DummyQuery1=> Dummy1,
    TestLRUQuery => TestLRU,
    DVCacheQuery => DVCache,
    MaxScoreWrapperQuery => MaxScoreWrapper,
    RandomApproximationQuery => RandomApproximation,
    RandomQuery => Random,
    TestRewriteQuery => TestRewrite,
    WANDScorerQuery => WANDScorer
);
impl_into_box_query!(
  BinaryRangeFieldRangeQuery,
  BlendedTermQuery,
  BooleanQuery,
  BoostQuery,
  ByteVectorSimilarityQuery,
  ConstantScoreQuery,
  DisjunctionMaxQuery,
  DocAndScoreQuery,
  DummyQuery,
  FieldExistsQuery,
  FloatVectorSimilarityQuery,
  IndexOrDocValuesQuery,
  IndexSortSortedNumericDocValuesRangeQuery,
  KnnByteVectorQuery,
  KnnFloatVectorQuery,
  LatLonDocValuesBoxQuery,
  LatLonDocValuesQuery,
  LatLonPointDistanceFeatureQuery,
  LatLonPointDistanceQuery,
  LatLonPointQuery,
  MatchAllDocsQuery,
  MatchNoDocsQuery,
  MultiTermQueryDocValuesWrapper,
  MultiTermQueryConstantScoreBlendedWrapper,
  MultiTermQueryConstantScoreWrapper,
  NGramPhraseQuery,
  PhraseQuery,
  PointInSetQuery,
  PointRangeQuery,
  RangeFieldQuery,
  SortedNumericDocValuesRangeQuery,
  SortedNumericDocValuesSetQuery,
  SortedSetDocValuesRangeQuery,
  SynonymQuery,
  TermQuery,
  XYDocValuesPointInGeometryQuery,
  XYPointInGeometryQuery,
);

pub trait QueryBase: Debug + HasIdentity + Accountable {
  fn to_string(&self, field: &str) -> Result<String>;

  fn create_weight<IRC>(
    self,
    _searcher: &IndexSearcher<IRC>,
    _score_mode: &ScoreMode,
    _boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized;

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized;

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Query {
  BinaryRangeFieldRange(BinaryRangeFieldRangeQuery),
  BlendedTerm(BlendedTermQuery),
  Boolean(BooleanQuery),
  Boost(BoostQuery),
  ByteVectorSimilarity(ByteVectorSimilarityQuery),
  ConstantScore(ConstantScoreQuery),
  DisjunctionMax(DisjunctionMaxQuery),
  DocAndScore(DocAndScoreQuery),
  Dummy(DummyQuery),
  FieldExists(FieldExistsQuery),
  FloatVectorSimilarity(FloatVectorSimilarityQuery),
  IndexOrDocValues(IndexOrDocValuesQuery),
  IndexSortSortedNumericDocValuesRange(IndexSortSortedNumericDocValuesRangeQuery),
  KnnByteVector(KnnByteVectorQuery),
  KnnFloatVector(KnnFloatVectorQuery),
  LatLonDocValues(LatLonDocValuesQuery),
  LatLonDocValuesBox(LatLonDocValuesBoxQuery),
  LatLonPoint(LatLonPointQuery),
  LatLonPointDistance(LatLonPointDistanceQuery),
  LatLonPointDistanceFeature(LatLonPointDistanceFeatureQuery),
  MatchAllDocs(MatchAllDocsQuery),
  MatchNoDocs(MatchNoDocsQuery),
  Named(NamedQuery),
  MultiPhrase(MultiPhraseQuery),
  MultiTermQuery(MultiTermQuerySet),
  MultiTermQueryDocValuesWrapper(MultiTermQueryDocValuesWrapper),
  MultiTermQueryConstantScoreBlendedWrapper(MultiTermQueryConstantScoreBlendedWrapper),
  MultiTermQueryConstantScoreWrapper(MultiTermQueryConstantScoreWrapper),
  NGramPhrase(NGramPhraseQuery),
  Phrase(PhraseQuery),
  PointInSet(PointInSetQuery),
  PointRange(PointRangeQuery),
  RangeField(RangeFieldQuery),
  SortedNumericDocValuesRange(SortedNumericDocValuesRangeQuery),
  SortedNumericDocValuesSet(SortedNumericDocValuesSetQuery),
  SortedSetDocValuesRange(SortedSetDocValuesRangeQuery),
  Synonym(SynonymQuery),
  Term(TermQuery),
  XYDocValuesPointInGeometry(XYDocValuesPointInGeometryQuery),
  XYPointInGeometry(XYPointInGeometryQuery),
  #[cfg(test)]
  Asserting(AssertingQuery),
  #[cfg(test)]
  AssertNeedsScores(AssertNeedsScores),
  #[cfg(test)]
  BitSet(BitSetQuery),
  #[cfg(test)]
  BlockScoreQueryWrapper(BlockScoreQueryWrapper),
  #[cfg(test)]
  BrokenExplainTerm(BrokenExplainTermQuery),
  #[cfg(test)]
  Counting(CountingQuery),
  #[cfg(test)]
  CrazyMustUseBulkScorer(CrazyMustUseBulkScorerQuery),
  #[cfg(test)]
  Dummy1(DummyQuery1),
  #[cfg(test)]
  TestLRU(TestLRUQuery),
  #[cfg(test)]
  DVCache(DVCacheQuery),
  #[cfg(test)]
  MaxScoreWrapper(MaxScoreWrapperQuery),
  #[cfg(test)]
  Random(RandomQuery),
  #[cfg(test)]
  RandomApproximation(RandomApproximationQuery),
  #[cfg(test)]
  TestRewrite(TestRewriteQuery),
  #[cfg(test)]
  WANDScorer(WANDScorerQuery),
}

macro_rules! define_query_ref {
  ($($variant:ident => $query:ty),* $(,)?) => {
    /// A borrowed query value used by [`QueryVisitor`].
    #[derive(Clone, Copy, Debug)]
    pub enum QueryRef<'a> {
      $($variant(&'a $query),)*
      #[cfg(test)]
      Asserting(&'a AssertingQuery),
      #[cfg(test)]
      AssertNeedsScores(&'a AssertNeedsScores),
      #[cfg(test)]
      BitSet(&'a BitSetQuery),
      #[cfg(test)]
      BlockScoreQueryWrapper(&'a BlockScoreQueryWrapper),
      #[cfg(test)]
      BoostChecking(&'a BoostCheckingQuery),
      #[cfg(test)]
      BrokenExplainTerm(&'a BrokenExplainTermQuery),
      #[cfg(test)]
      Counting(&'a CountingQuery),
      #[cfg(test)]
      CrazyMustUseBulkScorer(&'a CrazyMustUseBulkScorerQuery),
      #[cfg(test)]
      DumbPrefix(&'a DumbPrefixQuery),
      #[cfg(test)]
      DumbRegexp(&'a DumbRegexpQuery),
      #[cfg(test)]
      Dummy1(&'a DummyQuery1),
      #[cfg(test)]
      TestLRU(&'a TestLRUQuery),
      #[cfg(test)]
      DVCache(&'a DVCacheQuery),
      #[cfg(test)]
      MaxScoreWrapper(&'a MaxScoreWrapperQuery),
      #[cfg(test)]
      Random(&'a RandomQuery),
      #[cfg(test)]
      RandomApproximation(&'a RandomApproximationQuery),
      #[cfg(test)]
      TestRewrite(&'a TestRewriteQuery),
      #[cfg(test)]
      WANDScorer(&'a WANDScorerQuery),
    }

    $(
      impl<'a> From<&'a $query> for QueryRef<'a> {
        fn from(query: &'a $query) -> Self {
          Self::$variant(query)
        }
      }
    )*
  };
}

define_query_ref!(
  BinaryRangeFieldRange => BinaryRangeFieldRangeQuery,
  BlendedTerm => BlendedTermQuery,
  Boolean => BooleanQuery,
  Boost => BoostQuery,
  ByteVectorSimilarity => ByteVectorSimilarityQuery,
  ConstantScore => ConstantScoreQuery,
  DisjunctionMax => DisjunctionMaxQuery,
  DocAndScore => DocAndScoreQuery,
  Dummy => DummyQuery,
  FieldExists => FieldExistsQuery,
  FloatVectorSimilarity => FloatVectorSimilarityQuery,
  IndexOrDocValues => IndexOrDocValuesQuery,
  IndexSortSortedNumericDocValuesRange => IndexSortSortedNumericDocValuesRangeQuery,
  KnnByteVector => KnnByteVectorQuery,
  KnnFloatVector => KnnFloatVectorQuery,
  LatLonDocValues => LatLonDocValuesQuery,
  LatLonDocValuesBox => LatLonDocValuesBoxQuery,
  LatLonPoint => LatLonPointQuery,
  LatLonPointDistance => LatLonPointDistanceQuery,
  LatLonPointDistanceFeature => LatLonPointDistanceFeatureQuery,
  MatchAllDocs => MatchAllDocsQuery,
  MatchNoDocs => MatchNoDocsQuery,
  Named => NamedQuery,
  MultiPhrase => MultiPhraseQuery,
  MultiTermQueryDocValuesWrapper => MultiTermQueryDocValuesWrapper,
  MultiTermQueryConstantScoreBlendedWrapper => MultiTermQueryConstantScoreBlendedWrapper,
  MultiTermQueryConstantScoreWrapper => MultiTermQueryConstantScoreWrapper,
  NGramPhrase => NGramPhraseQuery,
  Phrase => PhraseQuery,
  PointInSet => PointInSetQuery,
  PointRange => PointRangeQuery,
  RangeField => RangeFieldQuery,
  SortedNumericDocValuesRange => SortedNumericDocValuesRangeQuery,
  SortedNumericDocValuesSet => SortedNumericDocValuesSetQuery,
  SortedSetDocValuesRange => SortedSetDocValuesRangeQuery,
  Synonym => SynonymQuery,
  Term => TermQuery,
  XYDocValuesPointInGeometry => XYDocValuesPointInGeometryQuery,
  XYPointInGeometry => XYPointInGeometryQuery,
  Automaton => AutomatonQuery,
  Fuzzy => FuzzyQuery,
  Prefix => PrefixQuery,
  Regexp => RegexpQuery,
  TermInSet => TermInSetQuery,
  TermRange => TermRangeQuery,
  Wildcard => WildcardQuery,
);

#[cfg(test)]
macro_rules! impl_test_query_ref_from {
  ($($variant:ident => $query:ty),* $(,)?) => {
    $(
      impl<'a> From<&'a $query> for QueryRef<'a> {
        fn from(query: &'a $query) -> Self {
          Self::$variant(query)
        }
      }
    )*
  };
}

#[cfg(test)]
impl_test_query_ref_from!(
  Asserting => AssertingQuery,
  AssertNeedsScores => AssertNeedsScores,
  BitSet => BitSetQuery,
  BlockScoreQueryWrapper => BlockScoreQueryWrapper,
  BoostChecking => BoostCheckingQuery,
  BrokenExplainTerm => BrokenExplainTermQuery,
  Counting => CountingQuery,
  CrazyMustUseBulkScorer => CrazyMustUseBulkScorerQuery,
  DumbPrefix => DumbPrefixQuery,
  DumbRegexp => DumbRegexpQuery,
  Dummy1 => DummyQuery1,
  TestLRU => TestLRUQuery,
  DVCache => DVCacheQuery,
  MaxScoreWrapper => MaxScoreWrapperQuery,
  Random => RandomQuery,
  RandomApproximation => RandomApproximationQuery,
  TestRewrite => TestRewriteQuery,
  WANDScorer => WANDScorerQuery,
);

macro_rules! query_variant_name {
    (
        $self:expr;
        normal: [ $( $variant:ident ),* $(,)? ];
        test: [ $( $test_variant:ident ),* $(,)? ]
    ) => {
        match $self {
            $(
                Query::$variant(_) => stringify!($variant),
            )*
            $(
                #[cfg(test)]
                Query::$test_variant(_) => stringify!($test_variant),
            )*
        }
    };
}
impl Query {
  pub fn name(&self) -> &'static str {
    query_variant_name!(
        self;
        normal: [
            BinaryRangeFieldRange,
            BlendedTerm,
            Boolean,
            Boost,
            ByteVectorSimilarity,
            ConstantScore,
            DisjunctionMax,
            DocAndScore,
            Dummy,
            FieldExists,
            FloatVectorSimilarity,
            IndexOrDocValues,
            IndexSortSortedNumericDocValuesRange,
            KnnByteVector,
            KnnFloatVector,
            LatLonDocValues,
            LatLonDocValuesBox,
            LatLonPoint,
            LatLonPointDistance,
            LatLonPointDistanceFeature,
            MatchAllDocs,
            MatchNoDocs,
            Named,
            MultiPhrase,
            MultiTermQuery,
            MultiTermQueryDocValuesWrapper,
            MultiTermQueryConstantScoreBlendedWrapper,
            MultiTermQueryConstantScoreWrapper,
            NGramPhrase,
            Phrase,
            PointInSet,
            PointRange,
            RangeField,
            SortedNumericDocValuesRange,
            SortedNumericDocValuesSet,
            SortedSetDocValuesRange,
            Synonym,
            Term,
            XYDocValuesPointInGeometry,
            XYPointInGeometry,
        ];
        test: [
            Asserting,
            AssertNeedsScores,
            BitSet,
            BlockScoreQueryWrapper,
            BrokenExplainTerm,
            Counting,
            CrazyMustUseBulkScorer,
            Dummy1,
            TestLRU,
            DVCache,
            MaxScoreWrapper,
            Random,
            RandomApproximation,
            TestRewrite,
            WANDScorer,
        ]
    )
  }
}
// for padding
impl Default for Query {
  fn default() -> Self {
    Query::Dummy(DummyQuery::default())
  }
}

impl Accountable for Query {
  fn ram_bytes_used(&self) -> Result<i64> {
    dispatch_query!(self, |q| q.ram_bytes_used())
  }
}

impl HasIdentity for Query {
  fn identity(&self) -> &Identity {
    dispatch_query!(self, |q| q.identity())
  }
}
impl QueryBase for Query {
  fn to_string(&self, field: &str) -> Result<String> {
    dispatch_query!(self, |q| q.to_string(field))
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
    dispatch_query!(self, |q| q.create_weight(searcher, score_mode, boost,))
  }

  fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
  {
    dispatch_query!(self, |q| q.rewrite(searcher))
  }

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    dispatch_query!(self, |query| QueryBase::visit(query, visitor))
  }
}

#[derive(Clone, Debug)]
pub struct IdentityQuery {
  pub(crate) query: Arc<Query>,
}
impl IdentityQuery {
  pub fn new(query: Arc<Query>) -> Self {
    Self { query }
  }
}

impl PartialEq for IdentityQuery {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.query, &other.query)
  }
}
impl Eq for IdentityQuery {}

impl Hash for IdentityQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    Arc::as_ptr(&self.query).hash(state);
  }
}
impl<Q> QueryBase for Arc<Q>
where
  Q: QueryBase,
{
  fn to_string(&self, field: &str) -> Result<String> {
    (**self).to_string(field)
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
    Err(LuceneError::unsupported_operation(format!(
      "Arc<QueryBase> cannot be used to create_weight directly: {}",
      std::any::type_name::<Q>()
    )))
  }

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
  {
    Err(LuceneError::unsupported_operation(format!(
      "Arc<QueryBase> cannot be used to rewrite directly: {}",
      std::any::type_name::<Q>()
    )))
  }

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    (**self).visit(visitor)
  }
}
pub trait IntoBoxQuery {
  fn into_box_query(self) -> Box<Query>;
}
pub trait IntoQuery {
  fn into_query(self) -> Query;
}
impl<T> IntoQuery for T
where
  T: Into<Query>,
{
  fn into_query(self) -> Query {
    self.into()
  }
}
impl IntoBoxQuery for Query {
  fn into_box_query(self) -> Box<Query> {
    Box::new(self)
  }
}
impl<T> IntoBoxQuery for T
where
  T: Into<MultiTermQuerySet>,
{
  fn into_box_query(self) -> Box<Query> {
    Box::new(self.into().into())
  }
}

impl IntoBoxQuery for Box<Query> {
  fn into_box_query(self) -> Box<Query> {
    self
  }
}

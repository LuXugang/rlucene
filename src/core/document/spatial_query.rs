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
use crate::core::document::shape_field::QueryRelation;
use crate::core::geo::component2d::WithinRelation;
use crate::core::geo::geometry::Geometry;
use crate::core::index::index_reader::{Identity, IndexReader};
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::point_values::{IntersectVisitor, PointValues, Relation};
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{
  AllDISI, DocIdSetIterator, DocIdSetIteratorEnum2, EmptyDISI,
};
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{
  Query, QueryWeight, QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer,
};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::Weight;
use crate::core::util::HasIdentity;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::bits::Bits;
use crate::core::util::doc_id_set_builder::DocIdSetBuilder;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Base query data for spatial geometries.
///
/// Java uses `SpatialQuery` as an abstract `Query` subclass. In this port the common immutable
/// state and scorer/visitor machinery live here, while concrete shape queries provide their own
/// [`SpatialVisitor`] implementation.
#[derive(Clone)]
pub struct SpatialQuery<G, C>
where
  C: SpatialQueryBase,
  G: Geometry,
{
  /// field name
  pub(crate) field: String,
  /// query relation
  pub(crate) query_relation: QueryRelation,
  pub(crate) geometries: Vec<G>,
  pub(crate) sub: C,
  id: Identity,
}

impl<G, C> SpatialQuery<G, C>
where
  C: SpatialQueryBase + 'static,
  G: Geometry + 'static,
{
  pub fn new(
    field: String,
    query_relation: QueryRelation,
    geometries: Vec<G>,
    sub: C,
  ) -> Result<Self> {
    Ok(Self {
      field,
      query_relation,
      geometries,
      sub,
      id: Identity::new(),
    })
  }

  /// returns the field name
  pub fn get_field(&self) -> &str {
    &self.field
  }

  /// returns the query relation
  pub fn get_query_relation(&self) -> QueryRelation {
    self.query_relation
  }

  pub fn transpose_relation(r: Relation) -> Relation {
    transpose_relation(r)
  }

  pub(crate) fn to_string(&self, field: &str) -> Result<String> {
    let mut sb = String::new();
    sb.push_str(std::any::type_name::<Self>());
    sb.push(':');
    if self.field != field {
      sb.push_str(" field=");
      sb.push_str(&self.field);
      sb.push(':');
    }
    sb.push('[');
    for geometry in &self.geometries {
      sb.push_str(&geometry.to_string());
      sb.push(',');
    }
    sb.push(']');
    Ok(sb)
  }

  pub(crate) fn inner_create_weight<IRC>(
    self,
    _searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    boost: f32,
    query: Arc<Query>,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    let spatial_visitor = self.sub.get_spatial_visitor()?;
    let spatial_weight = SpatialWeight::new(self, spatial_visitor, boost, *score_mode, query);
    Ok(Box::new(spatial_weight))
  }

  pub(crate) fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
    todo!()
  }
}

impl<G, C> Debug for SpatialQuery<G, C>
where
  C: SpatialQueryBase,
  G: Geometry,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl<G, C> HasIdentity for SpatialQuery<G, C>
where
  C: SpatialQueryBase,
  G: Geometry,
{
  fn identity(&self) -> &Identity {
    &self.id
  }
}
impl<G, C> PartialEq for SpatialQuery<G, C>
where
  G: Geometry,
  C: SpatialQueryBase,
{
  fn eq(&self, other: &Self) -> bool {
    self.field == other.field
      && self.query_relation == other.query_relation
      && self.geometries == other.geometries
  }
}

impl<G, C> Eq for SpatialQuery<G, C>
where
  G: Geometry,
  C: SpatialQueryBase,
{
}

impl<G, C> Hash for SpatialQuery<G, C>
where
  G: Geometry,
  C: SpatialQueryBase,
{
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.field.hash(state);
    self.query_relation.hash(state);
    self.geometries.hash(state);
  }
}
pub struct SpatialWeight<G, C>
where
  C: SpatialQueryBase,
  G: Geometry,
{
  parent_query: Arc<SpatialQuery<G, C>>,
  base: ConstantScoreWeight,
  spatial_visitor: Arc<C::SpatialVisitor>,
  boost: f32,
  score_mode: ScoreMode,
  query_s: Arc<Query>,
}
impl<G, C> SpatialWeight<G, C>
where
  C: SpatialQueryBase,
  G: Geometry,
{
  pub fn new(
    query: SpatialQuery<G, C>,
    spatial_visitor: C::SpatialVisitor,
    boost: f32,
    score_mode: ScoreMode,
    query_s: Arc<Query>,
  ) -> Self {
    let parent_query = Arc::new(query);
    let base = ConstantScoreWeight::new(boost);
    Self {
      parent_query,
      base,
      spatial_visitor: Arc::new(spatial_visitor),
      boost,
      score_mode,
      query_s,
    }
  }
}

impl<G, C, IRC> SegmentCacheable<IRC> for SpatialWeight<G, C>
where
  C: SpatialQueryBase,
  G: Geometry,
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    Ok(true)
  }
}

impl<G, C, IRC> Weight<IRC> for SpatialWeight<G, C>
where
  C: SpatialQueryBase + 'static,
  G: Geometry + 'static,
  IRC: IndexReaderContext,
{
  type Matches = MatchWithNoTerms;

  fn matches(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::Matches>> {
    self.default_matches(context, doc, searcher)
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    let scorer = self.scorer(context, searcher)?;
    self
      .base
      .explain(scorer, doc, self.parent_query.to_string("")?)
  }

  fn get_query(&self) -> Arc<Query> {
    self.query_s.clone()
  }

  type ScorerSupplier = QueryWeightSs<IRC>;

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    let reader = context.reader();
    let field = self.parent_query.field.clone();
    let values = match reader.get_point_values(&field)? {
      Some(values) => values,
      None => return Ok(None),
    };
    let field_infos = reader.get_field_infos()?;
    if field_infos.field_info_by_name(&field).is_none() {
      return Ok(None);
    }
    let query_relation = self.parent_query.get_query_relation();
    let score = self.base.score();
    let min_packed_value = values
      .get_min_packed_value()?
      .ok_or_else(|| LuceneError::illegal_state("min_packed_value is None"))?;
    let max_packed_value = values
      .get_max_packed_value()?
      .ok_or_else(|| LuceneError::illegal_state("max_packed_value is None"))?;
    let rel = self.spatial_visitor.get_inner_relation(
      query_relation,
      min_packed_value.as_ref(),
      max_packed_value.as_ref(),
    )?;
    let max_doc = reader.max_doc()?;

    if rel == Relation::CellOutsideQuery
      || (rel == Relation::CellInsideQuery && query_relation == QueryRelation::Contains)
    {
      Ok(None)
    } else if values.get_doc_count()? == reader.max_doc()? && rel == Relation::CellInsideQuery {
      Ok(Some(Box::new(ScorerSupplierImpl::new(
        score,
        self.score_mode,
        max_doc,
      ))))
    } else {
      if query_relation != QueryRelation::Intersects
        && query_relation != QueryRelation::Contains
        && values.get_doc_count()? != values.size()? as i32
        && !has_any_hits(&self.spatial_visitor, query_relation, &values)?
      {
        // First we check if we have any hits so we are fast in the adversarial case where
        // the shape does not match any documents and we are in the dense case
        return Ok(None);
      }

      Ok(Some(Box::new(RelationScorerSupplier::new(
        values,
        self.spatial_visitor.clone(),
        query_relation,
        field,
        score,
        self.score_mode,
        max_doc,
        rel,
      ))))
    }
  }
}

pub trait SpatialQueryBase {
  type SpatialVisitor: SpatialVisitor;
  fn get_spatial_visitor(&self) -> Result<Self::SpatialVisitor>;
}

/// Visitor used for walking the BKD tree.
pub trait SpatialVisitor {
  /// relates a range of points (internal node) to the query
  fn relate(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation>;

  /// intersects predicate. Called when constructing a scorer.
  fn intersects(&self, packed_value: &[u8]) -> Result<bool>;

  /// within predicate. Called when constructing a scorer.
  fn within(&self, packed_value: &[u8]) -> Result<bool>;

  /// contains function. Called when constructing a scorer.
  fn contains(&self, packed_value: &[u8]) -> Result<WithinRelation>;

  fn contains_predicate(&self, packed_value: &[u8]) -> Result<bool> {
    Ok(self.contains(packed_value)? == WithinRelation::Candidate)
  }

  fn get_inner_relation(
    &self,
    query_relation: QueryRelation,
    min_packed_value: &[u8],
    max_packed_value: &[u8],
  ) -> Result<Relation> {
    let relation = self.relate(min_packed_value, max_packed_value)?;
    if query_relation == QueryRelation::Disjoint {
      Ok(transpose_relation(relation))
    } else {
      Ok(relation)
    }
  }

  fn get_leaf_predicate(&self, query_relation: QueryRelation, packed_value: &[u8]) -> Result<bool> {
    match query_relation {
      QueryRelation::Intersects => self.intersects(packed_value),
      QueryRelation::Within => self.within(packed_value),
      QueryRelation::Disjoint => Ok(!self.intersects(packed_value)?),
      QueryRelation::Contains => self.contains_predicate(packed_value),
    }
  }
}
impl<T> SpatialVisitor for &T
where
  T: SpatialVisitor,
{
  fn relate(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    (*self).relate(min_packed_value, max_packed_value)
  }

  fn intersects(&self, packed_value: &[u8]) -> Result<bool> {
    (*self).intersects(packed_value)
  }

  fn within(&self, packed_value: &[u8]) -> Result<bool> {
    (*self).within(packed_value)
  }

  fn contains(&self, packed_value: &[u8]) -> Result<WithinRelation> {
    (*self).contains(packed_value)
  }

  fn contains_predicate(&self, packed_value: &[u8]) -> Result<bool> {
    (*self).contains_predicate(packed_value)
  }

  fn get_inner_relation(
    &self,
    query_relation: QueryRelation,
    min_packed_value: &[u8],
    max_packed_value: &[u8],
  ) -> Result<Relation> {
    (*self).get_inner_relation(query_relation, min_packed_value, max_packed_value)
  }

  fn get_leaf_predicate(&self, query_relation: QueryRelation, packed_value: &[u8]) -> Result<bool> {
    (*self).get_leaf_predicate(query_relation, packed_value)
  }
}
impl<T> SpatialVisitor for Arc<T>
where
  T: SpatialVisitor,
{
  fn relate(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    (**self).relate(min_packed_value, max_packed_value)
  }

  fn intersects(&self, packed_value: &[u8]) -> Result<bool> {
    (**self).intersects(packed_value)
  }

  fn within(&self, packed_value: &[u8]) -> Result<bool> {
    (**self).within(packed_value)
  }

  fn contains(&self, packed_value: &[u8]) -> Result<WithinRelation> {
    (**self).contains(packed_value)
  }

  fn contains_predicate(&self, packed_value: &[u8]) -> Result<bool> {
    (**self).contains_predicate(packed_value)
  }

  fn get_inner_relation(
    &self,
    query_relation: QueryRelation,
    min_packed_value: &[u8],
    max_packed_value: &[u8],
  ) -> Result<Relation> {
    (**self).get_inner_relation(query_relation, min_packed_value, max_packed_value)
  }

  fn get_leaf_predicate(&self, query_relation: QueryRelation, packed_value: &[u8]) -> Result<bool> {
    (**self).get_leaf_predicate(query_relation, packed_value)
  }
}

pub fn transpose_relation(r: Relation) -> Relation {
  match r {
    Relation::CellInsideQuery => Relation::CellOutsideQuery,
    Relation::CellOutsideQuery => Relation::CellInsideQuery,
    Relation::CellCrossesQuery => Relation::CellCrossesQuery,
  }
}

/// Utility class for implementing constant score logic specific to INTERSECT, WITHIN, DISJOINT
/// and CONTAINS.
pub struct RelationScorerSupplier<PV, V>
where
  PV: PointValues,
  V: SpatialVisitor,
{
  values: PV,
  spatial_visitor: V,
  query_relation: QueryRelation,
  field: String,
  score: f32,
  score_mode: ScoreMode,
  max_doc: i32,
  root_relation: Relation,
  cost: i64,
}

impl<PV, V> RelationScorerSupplier<PV, V>
where
  PV: PointValues,
  V: SpatialVisitor,
{
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    values: PV,
    spatial_visitor: V,
    query_relation: QueryRelation,
    field: String,
    score: f32,
    score_mode: ScoreMode,
    max_doc: i32,
    root_relation: Relation,
  ) -> Self {
    Self {
      values,
      spatial_visitor,
      query_relation,
      field,
      score,
      score_mode,
      max_doc,
      root_relation,
      cost: -1,
    }
  }

  fn get_scorer(&mut self) -> Result<QueryWeightSsScorer> {
    match self.query_relation {
      QueryRelation::Intersects => self.get_sparse_scorer(),
      QueryRelation::Contains => self.get_contains_dense_scorer(),
      QueryRelation::Within | QueryRelation::Disjoint => {
        if self.values.get_doc_count()? == self.values.size()? as i32 {
          self.get_sparse_scorer()
        } else {
          self.get_dense_scorer()
        }
      },
    }
  }

  fn get_sparse_scorer(&mut self) -> Result<QueryWeightSsScorer> {
    if self.query_relation == QueryRelation::Disjoint
      && self.values.get_doc_count()? == self.max_doc
      && self.values.get_doc_count()? == self.values.size()? as i32
      && self.cost_value()? > self.max_doc as i64 / 2
    {
      // If all docs have exactly one value and the cost is greater
      // than half the leaf size then maybe we can make things faster
      // by computing the set of documents that do NOT match the query
      let mut result = FixedBitSet::new(self.max_doc as usize);
      result.set_with_range(0, self.max_doc as usize);
      let cost = {
        let mut visitor = get_inverse_dense_visitor(
          &self.spatial_visitor,
          self.query_relation,
          &mut result,
          self.max_doc as i64,
        );
        self.values.intersect(&mut visitor)?;
        visitor.cost
      };
      let iterator = BitSetIterator::new(result, cost)?;
      Ok(Box::new(ConstantScoreScorer::from_disi(
        self.score,
        self.score_mode,
        iterator,
      )))
    } else if self.values.get_doc_count()? < (self.values.size()? >> 2) as i32 {
      let mut result = FixedBitSet::new(self.max_doc as usize);
      let cost = {
        let mut visitor =
          get_intersect_visitor(&self.spatial_visitor, self.query_relation, &mut result, 0);
        self.values.intersect(&mut visitor)?;
        visitor.cost
      };
      let iterator = if cost == 0 {
        DocIdSetIteratorEnum2::A(EmptyDISI::new())
      } else {
        DocIdSetIteratorEnum2::B(BitSetIterator::new(result, cost)?)
      };
      Ok(Box::new(ConstantScoreScorer::from_disi(
        self.score,
        self.score_mode,
        iterator,
      )))
    } else {
      let mut doc_id_set_builder =
        DocIdSetBuilder::from_point_values(self.max_doc, &self.values, &self.field)?;
      self.values.intersect(&mut get_sparse_visitor(
        &self.spatial_visitor,
        self.query_relation,
        &mut doc_id_set_builder,
      ))?;
      let iterator = doc_id_set_builder.build()?.iterator()?;
      Ok(Box::new(ConstantScoreScorer::from_disi(
        self.score,
        self.score_mode,
        iterator,
      )))
    }
  }

  fn get_dense_scorer(&mut self) -> Result<QueryWeightSsScorer> {
    let mut result = FixedBitSet::new(self.max_doc as usize);
    let cost;
    if self.values.get_doc_count()? == self.max_doc {
      // In this case we can spare one visit to the tree, all documents
      // are potential matches
      result.set_with_range(0, self.max_doc as usize);
      cost = {
        let mut visitor = get_inverse_dense_visitor(
          &self.spatial_visitor,
          self.query_relation,
          &mut result,
          self.values.size()? as i64,
        );
        // Remove false positives
        self.values.intersect(&mut visitor)?;
        visitor.cost
      };
    } else {
      let mut excluded = FixedBitSet::new(self.max_doc as usize);
      cost = {
        let mut visitor = get_dense_visitor(
          &self.spatial_visitor,
          self.query_relation,
          &mut result,
          &mut excluded,
        );
        self.values.intersect(&mut visitor)?;
        visitor.cost
      };
      result.and_not_fixed_bit_set(&excluded);
      let mut visitor =
        get_shallow_inverse_dense_visitor(&self.spatial_visitor, self.query_relation, &mut result);
      // Remove false positives, we only care about the inner nodes as intersecting
      // leaf nodes have been already taken into account. Unfortunately this
      // process still reads the leaf nodes.
      self.values.intersect(&mut visitor)?;
    }
    let iterator = if cost == 0 {
      DocIdSetIteratorEnum2::A(EmptyDISI::new())
    } else {
      DocIdSetIteratorEnum2::B(BitSetIterator::new(result, cost)?)
    };
    Ok(Box::new(ConstantScoreScorer::from_disi(
      self.score,
      self.score_mode,
      iterator,
    )))
  }

  fn get_contains_dense_scorer(&mut self) -> Result<QueryWeightSsScorer> {
    let mut result = FixedBitSet::new(self.max_doc as usize);
    let mut excluded = FixedBitSet::new(self.max_doc as usize);
    let cost = {
      let mut visitor = get_contains_dense_visitor(
        &self.spatial_visitor,
        self.query_relation,
        &mut result,
        &mut excluded,
      );
      self.values.intersect(&mut visitor)?;
      visitor.cost
    };
    result.and_not_fixed_bit_set(&excluded);
    let iterator = if cost == 0 {
      DocIdSetIteratorEnum2::A(EmptyDISI::new())
    } else {
      DocIdSetIteratorEnum2::B(BitSetIterator::new(result, cost)?)
    };
    Ok(Box::new(ConstantScoreScorer::from_disi(
      self.score,
      self.score_mode,
      iterator,
    )))
  }

  fn cost_value(&mut self) -> Result<i64> {
    if self.cost == -1 {
      self.cost = self.values.estimate_doc_count(&get_estimate_visitor(
        &self.spatial_visitor,
        self.query_relation,
      ))?;
      debug_assert!(self.cost >= 0);
    }
    Ok(self.cost)
  }
}

impl<IRC, PV, V> ScorerSupplier<IRC> for RelationScorerSupplier<PV, V>
where
  IRC: IndexReaderContext,
  PV: PointValues,
  V: SpatialVisitor,
{
  type Scorer = QueryWeightSsScorer;
  type BulkScorer = QueryWeightSsBulkScorer;

  fn get(
    &mut self,
    _lead_cost: i64,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::Scorer> {
    self.get_scorer()
  }

  fn bulk_scorer(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::BulkScorer>> {
    Ok(Some(Box::new(self.default_bulk_scorer(context, searcher)?)))
  }

  fn cost(
    &mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<i64> {
    self.cost_value()
  }
}
pub struct ScorerSupplierImpl {
  score: f32,
  score_mode: ScoreMode,
  max_doc: i32,
}
impl ScorerSupplierImpl {
  pub fn new(score: f32, score_mode: ScoreMode, max_doc: i32) -> Self {
    Self {
      score,
      score_mode,
      max_doc,
    }
  }
}
impl<IRC> ScorerSupplier<IRC> for ScorerSupplierImpl
where
  IRC: IndexReaderContext,
{
  type Scorer = QueryWeightSsScorer;
  type BulkScorer = QueryWeightSsBulkScorer;

  fn get(
    &mut self,
    _lead_cost: i64,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::Scorer> {
    Ok(Box::new(ConstantScoreScorer::from_disi(
      self.score,
      self.score_mode,
      AllDISI::new(self.max_doc),
    )))
  }

  fn bulk_scorer(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::BulkScorer>> {
    Ok(Some(Box::new(self.default_bulk_scorer(context, searcher)?)))
  }

  fn cost(
    &mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<i64> {
    Ok(self.max_doc as i64)
  }
}
/// create a visitor for calculating point count estimates for the provided relation
fn get_estimate_visitor(
  spatial_visitor: impl SpatialVisitor,
  query_relation: QueryRelation,
) -> EstimateVisitor<impl SpatialVisitor> {
  EstimateVisitor::new(spatial_visitor, query_relation)
}
struct EstimateVisitor<V> {
  spatial_visitor: V,
  query_relation: QueryRelation,
}

impl<V> EstimateVisitor<V> {
  fn new(spatial_visitor: V, query_relation: QueryRelation) -> Self {
    Self {
      spatial_visitor,
      query_relation,
    }
  }
}

impl<V> IntersectVisitor for EstimateVisitor<V>
where
  V: SpatialVisitor,
{
  fn visit(&mut self, _doc_id: i32) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn visit_with_packed_value(&mut self, _doc_id: i32, _packed_value: &[u8]) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    self
      .spatial_visitor
      .get_inner_relation(self.query_relation, min_packed_value, max_packed_value)
  }
}
/// create a visitor that adds documents that match the query using a sparse bitset.
/// (Used by INTERSECT when the number of docs <= 4 * number of points )
fn get_sparse_visitor(
  spatial_visitor: impl SpatialVisitor,
  query_relation: QueryRelation,
  result: &mut DocIdSetBuilder,
) -> SparseVisitor<'_, impl SpatialVisitor> {
  SparseVisitor::new(spatial_visitor, query_relation, result)
}
struct SparseVisitor<'a, V> {
  spatial_visitor: V,
  query_relation: QueryRelation,
  result: &'a mut DocIdSetBuilder,
}

impl<'a, V> SparseVisitor<'a, V> {
  fn new(
    spatial_visitor: V,
    query_relation: QueryRelation,
    result: &'a mut DocIdSetBuilder,
  ) -> Self {
    Self {
      spatial_visitor,
      query_relation,
      result,
    }
  }
}

impl<V> IntersectVisitor for SparseVisitor<'_, V>
where
  V: SpatialVisitor,
{
  fn grow(&mut self, count: usize) -> Result<()> {
    self.result.grow(count as i32);
    Ok(())
  }

  fn visit(&mut self, doc_id: i32) -> Result<()> {
    self.result.add_doc(doc_id);
    Ok(())
  }

  fn visit_with_iterator(&mut self, iterator: &mut impl DocIdSetIterator) -> Result<()> {
    self.result.add_disi(iterator)
  }

  fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
    if self
      .spatial_visitor
      .get_leaf_predicate(self.query_relation, packed_value)?
    {
      self.visit(doc_id)?;
    }
    Ok(())
  }

  fn visit_iterator_with_packed_value(
    &mut self,
    iterator: &mut impl DocIdSetIterator,
    packed_value: &[u8],
  ) -> Result<()> {
    if self
      .spatial_visitor
      .get_leaf_predicate(self.query_relation, packed_value)?
    {
      self.result.add_disi(iterator)?;
    }
    Ok(())
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    self
      .spatial_visitor
      .get_inner_relation(self.query_relation, min_packed_value, max_packed_value)
  }
}
/// Scorer used for INTERSECTS when the number of points > 4 * number of docs
fn get_intersect_visitor<V>(
  spatial_visitor: V,
  query_relation: QueryRelation,
  result: &mut FixedBitSet,
  cost: i64,
) -> IntersectsDenseVisitor<'_, V> {
  IntersectsDenseVisitor::new(spatial_visitor, query_relation, result, cost)
}
struct IntersectsDenseVisitor<'a, V> {
  spatial_visitor: V,
  query_relation: QueryRelation,
  result: &'a mut FixedBitSet,
  cost: i64,
}

impl<'a, V> IntersectsDenseVisitor<'a, V> {
  fn new(
    spatial_visitor: V,
    query_relation: QueryRelation,
    result: &'a mut FixedBitSet,
    cost: i64,
  ) -> Self {
    Self {
      spatial_visitor,
      query_relation,
      result,
      cost,
    }
  }
}

impl<V> IntersectVisitor for IntersectsDenseVisitor<'_, V>
where
  V: SpatialVisitor,
{
  fn visit(&mut self, doc_id: i32) -> Result<()> {
    self.result.set(doc_id as usize);
    self.cost += 1;
    Ok(())
  }

  fn visit_with_iterator(&mut self, iterator: &mut impl DocIdSetIterator) -> Result<()> {
    BitSet::or(self.result, iterator)?;
    self.cost += iterator.cost()?;
    Ok(())
  }

  fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
    if !self.result.get(doc_id as usize)?
      && self
        .spatial_visitor
        .get_leaf_predicate(self.query_relation, packed_value)?
    {
      self.visit(doc_id)?;
    }
    Ok(())
  }

  fn visit_iterator_with_packed_value(
    &mut self,
    iterator: &mut impl DocIdSetIterator,
    packed_value: &[u8],
  ) -> Result<()> {
    if self
      .spatial_visitor
      .get_leaf_predicate(self.query_relation, packed_value)?
    {
      self.visit_with_iterator(iterator)?;
    }
    Ok(())
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    self
      .spatial_visitor
      .get_inner_relation(self.query_relation, min_packed_value, max_packed_value)
  }
}
/// create a visitor that adds documents that match the query using a dense bitset; used with WITHIN & DISJOINT
fn get_dense_visitor<'a, V>(
  spatial_visitor: V,
  query_relation: QueryRelation,
  result: &'a mut FixedBitSet,
  excluded: &'a mut FixedBitSet,
) -> DenseVisitor<'a, V> {
  DenseVisitor::new(spatial_visitor, query_relation, result, excluded)
}
struct DenseVisitor<'a, V> {
  spatial_visitor: V,
  query_relation: QueryRelation,
  result: &'a mut FixedBitSet,
  excluded: &'a mut FixedBitSet,
  cost: i64,
}

impl<'a, V> DenseVisitor<'a, V> {
  fn new(
    spatial_visitor: V,
    query_relation: QueryRelation,
    result: &'a mut FixedBitSet,
    excluded: &'a mut FixedBitSet,
  ) -> Self {
    Self {
      spatial_visitor,
      query_relation,
      result,
      excluded,
      cost: 0,
    }
  }
}

impl<V> IntersectVisitor for DenseVisitor<'_, V>
where
  V: SpatialVisitor,
{
  fn visit(&mut self, doc_id: i32) -> Result<()> {
    self.result.set(doc_id as usize);
    self.cost += 1;
    Ok(())
  }

  fn visit_with_iterator(&mut self, iterator: &mut impl DocIdSetIterator) -> Result<()> {
    BitSet::or(self.result, iterator)?;
    self.cost += iterator.cost()?;
    Ok(())
  }

  fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
    if !self.excluded.get(doc_id as usize)? {
      if self
        .spatial_visitor
        .get_leaf_predicate(self.query_relation, packed_value)?
      {
        self.visit(doc_id)?;
      } else {
        self.excluded.set(doc_id as usize);
      }
    }
    Ok(())
  }

  fn visit_iterator_with_packed_value(
    &mut self,
    iterator: &mut impl DocIdSetIterator,
    packed_value: &[u8],
  ) -> Result<()> {
    if self
      .spatial_visitor
      .get_leaf_predicate(self.query_relation, packed_value)?
    {
      self.visit_with_iterator(iterator)?;
    } else {
      BitSet::or(self.excluded, iterator)?;
    }
    Ok(())
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    self
      .spatial_visitor
      .get_inner_relation(self.query_relation, min_packed_value, max_packed_value)
  }
}
/// create a visitor that adds documents that match the query using a dense bitset; used with CONTAINS
fn get_contains_dense_visitor<'a, V>(
  spatial_visitor: V,
  query_relation: QueryRelation,
  result: &'a mut FixedBitSet,
  excluded: &'a mut FixedBitSet,
) -> ContainsDenseVisitor<'a, V> {
  ContainsDenseVisitor::new(spatial_visitor, query_relation, result, excluded)
}
struct ContainsDenseVisitor<'a, V> {
  spatial_visitor: V,
  query_relation: QueryRelation,
  result: &'a mut FixedBitSet,
  excluded: &'a mut FixedBitSet,
  cost: i64,
}

impl<'a, V> ContainsDenseVisitor<'a, V> {
  fn new(
    spatial_visitor: V,
    query_relation: QueryRelation,
    result: &'a mut FixedBitSet,
    excluded: &'a mut FixedBitSet,
  ) -> Self {
    Self {
      spatial_visitor,
      query_relation,
      result,
      excluded,
      cost: 0,
    }
  }
}

impl<V> IntersectVisitor for ContainsDenseVisitor<'_, V>
where
  V: SpatialVisitor,
{
  fn visit(&mut self, doc_id: i32) -> Result<()> {
    self.excluded.set(doc_id as usize);
    Ok(())
  }

  fn visit_with_iterator(&mut self, iterator: &mut impl DocIdSetIterator) -> Result<()> {
    BitSet::or(self.excluded, iterator)
  }

  fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
    if !self.excluded.get(doc_id as usize)? {
      match self.spatial_visitor.contains(packed_value)? {
        WithinRelation::Candidate => {
          self.cost += 1;
          self.result.set(doc_id as usize);
        },
        WithinRelation::NotWithin => self.excluded.set(doc_id as usize),
        WithinRelation::Disjoint => {},
      }
    }
    Ok(())
  }

  fn visit_iterator_with_packed_value(
    &mut self,
    iterator: &mut impl DocIdSetIterator,
    packed_value: &[u8],
  ) -> Result<()> {
    let within = self.spatial_visitor.contains(packed_value)?;
    loop {
      let doc_id = iterator.next_doc()?;
      if doc_id == NO_MORE_DOCS {
        break;
      }
      match within {
        WithinRelation::Candidate => {
          self.cost += 1;
          self.result.set(doc_id as usize);
        },
        WithinRelation::NotWithin => self.excluded.set(doc_id as usize),
        WithinRelation::Disjoint => {},
      }
    }
    Ok(())
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    self
      .spatial_visitor
      .get_inner_relation(self.query_relation, min_packed_value, max_packed_value)
  }
}
/// create a visitor that clears documents that do not match the polygon query using a dense bitset; used with WITHIN & DISJOINT
fn get_inverse_dense_visitor<V>(
  spatial_visitor: V,
  query_relation: QueryRelation,
  result: &mut FixedBitSet,
  cost: i64,
) -> InverseDenseVisitor<'_, V> {
  InverseDenseVisitor::new(spatial_visitor, query_relation, result, cost)
}
struct InverseDenseVisitor<'a, V> {
  spatial_visitor: V,
  query_relation: QueryRelation,
  result: &'a mut FixedBitSet,
  cost: i64,
}

impl<'a, V> InverseDenseVisitor<'a, V> {
  fn new(
    spatial_visitor: V,
    query_relation: QueryRelation,
    result: &'a mut FixedBitSet,
    cost: i64,
  ) -> Self {
    Self {
      spatial_visitor,
      query_relation,
      result,
      cost,
    }
  }
}

impl<V> IntersectVisitor for InverseDenseVisitor<'_, V>
where
  V: SpatialVisitor,
{
  fn visit(&mut self, doc_id: i32) -> Result<()> {
    self.result.clear_with_index(doc_id as usize);
    self.cost -= 1;
    Ok(())
  }

  fn visit_with_iterator(&mut self, iterator: &mut impl DocIdSetIterator) -> Result<()> {
    self.result.and_not_iter(iterator)?;
    self.cost = (self.cost - iterator.cost()?).max(0);
    Ok(())
  }

  fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
    if self.result.get(doc_id as usize)?
      && !self
        .spatial_visitor
        .get_leaf_predicate(self.query_relation, packed_value)?
    {
      self.visit(doc_id)?;
    }
    Ok(())
  }

  fn visit_iterator_with_packed_value(
    &mut self,
    iterator: &mut impl DocIdSetIterator,
    packed_value: &[u8],
  ) -> Result<()> {
    if !self
      .spatial_visitor
      .get_leaf_predicate(self.query_relation, packed_value)?
    {
      self.visit_with_iterator(iterator)?;
    }
    Ok(())
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    Ok(transpose_relation(
      self.spatial_visitor.get_inner_relation(
        self.query_relation,
        min_packed_value,
        max_packed_value,
      )?,
    ))
  }
}
/// Create a visitor that clears documents that do not match the polygon query using a dense bitset; used with WITHIN & DISJOINT.
/// This visitor only takes into account inner nodes
fn get_shallow_inverse_dense_visitor<V>(
  spatial_visitor: V,
  query_relation: QueryRelation,
  result: &mut FixedBitSet,
) -> ShallowInverseDenseVisitor<'_, V> {
  ShallowInverseDenseVisitor::new(spatial_visitor, query_relation, result)
}
struct ShallowInverseDenseVisitor<'a, V> {
  spatial_visitor: V,
  query_relation: QueryRelation,
  result: &'a mut FixedBitSet,
}

impl<'a, V> ShallowInverseDenseVisitor<'a, V> {
  fn new(spatial_visitor: V, query_relation: QueryRelation, result: &'a mut FixedBitSet) -> Self {
    Self {
      spatial_visitor,
      query_relation,
      result,
    }
  }
}

impl<V> IntersectVisitor for ShallowInverseDenseVisitor<'_, V>
where
  V: SpatialVisitor,
{
  fn visit(&mut self, doc_id: i32) -> Result<()> {
    self.result.clear_with_index(doc_id as usize);
    Ok(())
  }

  fn visit_with_iterator(&mut self, iterator: &mut impl DocIdSetIterator) -> Result<()> {
    self.result.and_not_iter(iterator)
  }

  fn visit_with_packed_value(&mut self, _doc_id: i32, _packed_value: &[u8]) -> Result<()> {
    Ok(())
  }

  fn visit_iterator_with_packed_value(
    &mut self,
    _iterator: &mut impl DocIdSetIterator,
    _packed_value: &[u8],
  ) -> Result<()> {
    Ok(())
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    Ok(transpose_relation(
      self.spatial_visitor.get_inner_relation(
        self.query_relation,
        min_packed_value,
        max_packed_value,
      )?,
    ))
  }
}

struct HasAnyHitsVisitor<V> {
  spatial_visitor: V,
  query_relation: QueryRelation,
}

impl<V> HasAnyHitsVisitor<V> {
  fn new(spatial_visitor: V, query_relation: QueryRelation) -> Self {
    Self {
      spatial_visitor,
      query_relation,
    }
  }
}

impl<V> IntersectVisitor for HasAnyHitsVisitor<V>
where
  V: SpatialVisitor,
{
  fn visit(&mut self, _doc_id: i32) -> Result<()> {
    Err(LuceneError::collection_terminated(""))
  }

  fn visit_with_packed_value(&mut self, _doc_id: i32, packed_value: &[u8]) -> Result<()> {
    if self
      .spatial_visitor
      .get_leaf_predicate(self.query_relation, packed_value)?
    {
      Err(LuceneError::collection_terminated(""))
    } else {
      Ok(())
    }
  }

  fn visit_iterator_with_packed_value(
    &mut self,
    _iterator: &mut impl DocIdSetIterator,
    packed_value: &[u8],
  ) -> Result<()> {
    if self
      .spatial_visitor
      .get_leaf_predicate(self.query_relation, packed_value)?
    {
      Err(LuceneError::collection_terminated(""))
    } else {
      Ok(())
    }
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    let rel = self.spatial_visitor.get_inner_relation(
      self.query_relation,
      min_packed_value,
      max_packed_value,
    )?;
    if rel == Relation::CellInsideQuery {
      Err(LuceneError::collection_terminated(""))
    } else {
      Ok(rel)
    }
  }
}

fn has_any_hits<PV, V>(
  spatial_visitor: &V,
  query_relation: QueryRelation,
  values: &PV,
) -> Result<bool>
where
  PV: PointValues,
  V: SpatialVisitor,
{
  let mut visitor = HasAnyHitsVisitor::new(spatial_visitor, query_relation);
  match values.intersect(&mut visitor) {
    Err(LuceneError::CollectionTerminated(_)) => Ok(true),
    Err(e) => Err(e),
    Ok(()) => Ok(false),
  }
}

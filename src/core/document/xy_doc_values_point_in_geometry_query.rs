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
use crate::core::geo::component2d::Component2D;
use crate::core::geo::geometry::Geometry;
use crate::core::geo::xy_encoding_utils::XYEncodingUtils;
use crate::core::geo::xy_geometry;
use crate::core::geo::xy_geometry::{XYGeometryEnum, XYGeometryType};
use crate::core::index::doc_values::DocValues;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// XYGeometry query for XY doc values.
///
/// The field must be indexed using XY doc values encoded as `x << 32 | y`.
#[derive(Clone)]
pub struct XYDocValuesPointInGeometryQuery {
  id: Identity,
  field: String,
  geometries: Arc<Vec<XYGeometryEnum>>,
  component2d: Arc<XYGeometryType<<XYGeometryEnum as Geometry>::Component2D>>,
}

impl XYDocValuesPointInGeometryQuery {
  pub(crate) fn new(field: String, geometries: Vec<XYGeometryEnum>) -> Result<Self> {
    if geometries.is_empty() {
      return Err(LuceneError::illegal_argument(
        "geometries must not be empty",
      ));
    }
    let component2d = Arc::new(xy_geometry::create(geometries.as_slice())?);
    let geometries = Arc::new(geometries);
    Ok(Self {
      id: Identity::new(),
      field,
      geometries,
      component2d,
    })
  }
}

impl Debug for XYDocValuesPointInGeometryQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("XYDocValuesPointInGeometryQuery")
      .field("field", &self.field)
      .finish()
  }
}

impl PartialEq for XYDocValuesPointInGeometryQuery {
  fn eq(&self, other: &Self) -> bool {
    self.field == other.field && self.geometries == other.geometries
  }
}

impl Eq for XYDocValuesPointInGeometryQuery {}

impl Hash for XYDocValuesPointInGeometryQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.field.hash(state);
    self.geometries.hash(state);
  }
}

impl HasIdentity for XYDocValuesPointInGeometryQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for XYDocValuesPointInGeometryQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    let mut sb = String::new();
    if self.field != field {
      sb.push_str(&self.field);
      sb.push(':');
    }
    sb.push_str("geometries(");
    for (i, geometry) in self.geometries.iter().enumerate() {
      if i > 0 {
        sb.push_str(", ");
      }
      sb.push_str(&geometry.to_string());
    }
    sb.push(')');
    Ok(sb)
  }

  fn create_weight<IRC>(
    self,
    _searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Ok(Box::new(XYDocValuesPointInGeometryQueryWeight::new(
      self,
      *score_mode,
      boost,
    )))
  }

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Ok(self.into())
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
    todo!()
  }
}

pub struct XYDocValuesPointInGeometryQueryWeight {
  query: XYDocValuesPointInGeometryQuery,
  base: ConstantScoreWeight,
  parent_query: Arc<Query>,
  score_mode: ScoreMode,
  boost: f32,
}

impl XYDocValuesPointInGeometryQueryWeight {
  fn new(query: XYDocValuesPointInGeometryQuery, score_mode: ScoreMode, boost: f32) -> Self {
    let query_clone = query.clone();
    let parent_query = Arc::new(query.into());
    Self {
      query: query_clone,
      base: ConstantScoreWeight::new(boost),
      parent_query,
      score_mode,
      boost,
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for XYDocValuesPointInGeometryQueryWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    let field = vec![self.query.field.clone()];
    DocValues::is_cacheable(ctx, field.as_ref())
  }
}

impl<IRC> Weight<IRC> for XYDocValuesPointInGeometryQueryWeight
where
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
    self.parent_query.clone()
  }

  type ScorerSupplier = QueryWeightSs<IRC>;

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    match context
      .reader()
      .get_sorted_numeric_doc_values(&self.query.field)?
    {
      Some(values) => {
        let iterator = XYDocValuesPointInGeometryTPI::new(values, self.query.component2d.clone());
        let scorer = ConstantScoreScorer::from_tpi(self.boost, self.score_mode, iterator);
        Ok(Some(Box::new(DefaultScorerSupplier::new(scorer))))
      },
      None => Ok(None),
    }
  }
}

pub struct XYDocValuesPointInGeometryTPI<S, C>
where
  S: SortedNumericDocValues,
  C: Component2D,
{
  values: S,
  component2d: C,
}

impl<S, C> XYDocValuesPointInGeometryTPI<S, C>
where
  S: SortedNumericDocValues,
  C: Component2D,
{
  fn new(values: S, component2d: C) -> Self {
    Self {
      values,
      component2d,
    }
  }
}

impl<S, C> TwoPhaseIterator for XYDocValuesPointInGeometryTPI<S, C>
where
  S: SortedNumericDocValues,
  C: Component2D,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.values)
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.values)
  }

  fn matches(&mut self) -> Result<bool> {
    let count = self.values.doc_value_count()?;
    for _ in 0..count {
      let value = self.values.next_value()? as u64;
      let x = XYEncodingUtils::decode((value >> 32) as i32) as f64;
      let y = XYEncodingUtils::decode(value as u32 as i32) as f64;
      if self.component2d.contains(x, y) {
        return Ok(true);
      }
    }
    Ok(false)
  }

  fn match_cost(&self) -> f32 {
    1000f32
  }
}

impl crate::core::util::accountable::Accountable for XYDocValuesPointInGeometryQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}

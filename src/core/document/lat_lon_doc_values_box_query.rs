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
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::index::doc_values::DocValues;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::Result;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Distance query for LatLonDocValuesField.
#[derive(Debug, Clone)]
pub struct LatLonDocValuesBoxQuery {
  id: Identity,
  field: String,
  min_latitude: i32,
  max_latitude: i32,
  min_longitude: i32,
  max_longitude: i32,
  crosses_dateline: bool,
}

impl LatLonDocValuesBoxQuery {
  pub fn new(
    field: String,
    min_latitude: f64,
    max_latitude: f64,
    min_longitude: f64,
    max_longitude: f64,
  ) -> Result<Self> {
    GeoUtils::check_latitude(min_latitude)?;
    GeoUtils::check_latitude(max_latitude)?;
    GeoUtils::check_longitude(min_longitude)?;
    GeoUtils::check_longitude(max_longitude)?;

    let crosses_dateline = min_longitude > max_longitude;

    Ok(Self {
      id: Identity::new(),
      field,
      crosses_dateline,
      min_latitude: GeoEncodingUtils::encode_latitude_ceil(min_latitude)?,
      max_latitude: GeoEncodingUtils::encode_latitude(max_latitude)?,
      min_longitude: GeoEncodingUtils::encode_longitude_ceil(min_longitude)?,
      max_longitude: GeoEncodingUtils::encode_longitude(max_longitude)?,
    })
  }
}

impl PartialEq for LatLonDocValuesBoxQuery {
  fn eq(&self, other: &Self) -> bool {
    self.field == other.field
      && self.crosses_dateline == other.crosses_dateline
      && self.min_latitude == other.min_latitude
      && self.max_latitude == other.max_latitude
      && self.min_longitude == other.min_longitude
      && self.max_longitude == other.max_longitude
  }
}

impl Eq for LatLonDocValuesBoxQuery {}

impl Hash for LatLonDocValuesBoxQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.field.hash(state);
    self.crosses_dateline.hash(state);
    self.min_latitude.hash(state);
    self.max_latitude.hash(state);
    self.min_longitude.hash(state);
    self.max_longitude.hash(state);
  }
}

impl HasIdentity for LatLonDocValuesBoxQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for LatLonDocValuesBoxQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    let mut out = String::new();
    if self.field != field {
      out.push_str(&self.field);
      out.push(':');
    }
    out.push_str("box(minLat=");
    out.push_str(&GeoEncodingUtils::decode_latitude(self.min_latitude).to_string());
    out.push_str(", maxLat=");
    out.push_str(&GeoEncodingUtils::decode_latitude(self.max_latitude).to_string());
    out.push_str(", minLon=");
    out.push_str(&GeoEncodingUtils::decode_longitude(self.min_longitude).to_string());
    out.push_str(", maxLon=");
    out.push_str(&GeoEncodingUtils::decode_longitude(self.max_longitude).to_string());
    out.push(')');
    Ok(out)
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
    Ok(Box::new(LatLonDocValuesBoxQueryWeight::new(
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

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    let query = self.into();
    if visitor.accept_field(&self.field) {
      visitor.visit_leaf(query)?;
    }
    Ok(())
  }
}

pub struct LatLonDocValuesBoxQueryWeight {
  query: LatLonDocValuesBoxQuery,
  base: ConstantScoreWeight,
  parent_query: Arc<Query>,
  score_mode: ScoreMode,
}

impl LatLonDocValuesBoxQueryWeight {
  fn new(query: LatLonDocValuesBoxQuery, score_mode: ScoreMode, boost: f32) -> Self {
    let query_clone = query.clone();
    let parent_query = Arc::new(query.into());
    Self {
      query: query_clone,
      base: ConstantScoreWeight::new(boost),
      parent_query,
      score_mode,
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for LatLonDocValuesBoxQueryWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    let field = vec![self.query.field.clone()];
    DocValues::is_cacheable(ctx, field.as_ref())
  }
}

impl<IRC> Weight<IRC> for LatLonDocValuesBoxQueryWeight
where
  IRC: IndexReaderContext,
{
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
        let iterator = LatLonDocValuesBoxTwoPhaseIterator::new(values, self.query.clone());
        let scorer = ConstantScoreScorer::from_tpi(self.base.score(), self.score_mode, iterator);
        Ok(Some(Box::new(DefaultScorerSupplier::new(scorer))))
      },
      None => Ok(None),
    }
  }
}

struct LatLonDocValuesBoxTwoPhaseIterator<S>
where
  S: SortedNumericDocValues,
{
  values: S,
  query: LatLonDocValuesBoxQuery,
}

impl<S> LatLonDocValuesBoxTwoPhaseIterator<S>
where
  S: SortedNumericDocValues,
{
  fn new(values: S, query: LatLonDocValuesBoxQuery) -> Self {
    Self { values, query }
  }
}

impl<S> TwoPhaseIterator for LatLonDocValuesBoxTwoPhaseIterator<S>
where
  S: SortedNumericDocValues,
{
  fn approximation_mut(
    &mut self,
  ) -> Box<dyn crate::core::search::doc_id_set_iterator::DocIdSetIterator + '_> {
    Box::new(&mut self.values)
  }

  fn approximation(
    &self,
  ) -> Box<dyn crate::core::search::doc_id_set_iterator::DocIdSetIterator + '_> {
    Box::new(&self.values)
  }

  fn matches(&mut self) -> Result<bool> {
    let count = self.values.doc_value_count()?;
    for _ in 0..count {
      let value = self.values.next_value()? as u64;
      let lat = (value >> 32) as i32;
      if lat < self.query.min_latitude || lat > self.query.max_latitude {
        continue;
      }

      let lon = value as u32 as i32;
      if self.query.crosses_dateline {
        if lon > self.query.max_longitude && lon < self.query.min_longitude {
          continue;
        }
      } else if lon < self.query.min_longitude || lon > self.query.max_longitude {
        continue;
      }

      return Ok(true);
    }

    Ok(false)
  }

  fn match_cost(&self) -> f32 {
    5.0
  }
}

impl crate::core::util::accountable::Accountable for LatLonDocValuesBoxQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}

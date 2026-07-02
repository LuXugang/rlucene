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
use crate::core::document::field::Field;
use crate::core::document::shape_field;
use crate::core::document::shape_field::{DecodedTriangle, DecodedTriangleType, QueryRelation};
use crate::core::geo::component2d::{Component2D, WithinRelation};
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::indexable_field::IndexableField;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::Query;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::test_framework::core::search::fixed_bit_set_collector::FixedBitSetCollector;

pub trait BaseSpatialTestCase {
  fn search_index<IRC>(
    &self,
    s: &IndexSearcher<IRC>,
    query: Query,
    max_doc: i32,
  ) -> Result<FixedBitSet>
  where
    IRC: IndexReaderContext + std::marker::Sync,
  {
    s.search_with_collector_manager(query, &FixedBitSetCollector::create_manager(max_doc))
  }
  type Validator: Validator;
  fn get_validator(&self) -> Result<Self::Validator>;
}
pub(crate) trait Encoder {
  fn decode_x(&self, encoded: i32) -> f64;

  fn decode_y(&self, encoded: i32) -> f64;

  fn quantize_x(&self, raw: f64) -> f64;

  fn quantize_x_ceil(&self, raw: f64) -> f64;

  fn quantize_y(&self, raw: f64) -> f64;

  fn quantize_y_ceil(&self, raw: f64) -> f64;
}

pub(crate) trait Validator {
  type Encoder: Encoder;
  fn encoder(&self) -> &Self::Encoder;

  fn query_relation(&self) -> QueryRelation {
    QueryRelation::Intersects
  }

  fn set_relation(&mut self, relation: QueryRelation);

  fn test_component_query_with_shape<T>(&self, line2d: &impl Component2D, shape: &T) -> bool;

  fn test_component_query(&self, query: &impl Component2D, fields: &[Field]) -> Result<bool> {
    let mut decoded_triangle = DecodedTriangle::default();

    for field in fields {
      let (intersects, contains) = match field.binary_value()? {
        Some(binary_value) => {
          shape_field::decode_triangle(&binary_value.as_ref().bytes, &mut decoded_triangle)?;

          match decoded_triangle.type_ {
            DecodedTriangleType::Point => {
              let y = self.encoder().decode_y(decoded_triangle.a_y);
              let x = self.encoder().decode_x(decoded_triangle.a_x);
              let intersects = query.contains(x, y);
              let contains = intersects;
              (intersects, contains)
            },
            DecodedTriangleType::Line => {
              let a_y = self.encoder().decode_y(decoded_triangle.a_y);
              let a_x = self.encoder().decode_x(decoded_triangle.a_x);
              let b_y = self.encoder().decode_y(decoded_triangle.b_y);
              let b_x = self.encoder().decode_x(decoded_triangle.b_x);
              let intersects = query.intersects_line_values(a_x, a_y, b_x, b_y);
              let contains = query.contains_line_values(a_x, a_y, b_x, b_y);
              (intersects, contains)
            },
            DecodedTriangleType::Triangle => {
              let a_y = self.encoder().decode_y(decoded_triangle.a_y);
              let a_x = self.encoder().decode_x(decoded_triangle.a_x);
              let b_y = self.encoder().decode_y(decoded_triangle.b_y);
              let b_x = self.encoder().decode_x(decoded_triangle.b_x);
              let c_y = self.encoder().decode_y(decoded_triangle.c_y);
              let c_x = self.encoder().decode_x(decoded_triangle.c_x);
              let intersects = query.intersects_triangle_values(a_x, a_y, b_x, b_y, c_x, c_y);
              let contains = query.contains_triangle_values(a_x, a_y, b_x, b_y, c_x, c_y);
              (intersects, contains)
            },
          }
        },
        None => {
          return Err(LuceneError::illegal_argument(
            "field.binary_value() is None",
          ));
        },
      };

      assert!((contains == intersects) || (!contains && intersects));

      match self.query_relation() {
        QueryRelation::Disjoint if intersects => return Ok(false),
        QueryRelation::Within if !contains => return Ok(false),
        QueryRelation::Intersects if intersects => return Ok(true),
        _ => {},
      }
    }

    Ok(!matches!(self.query_relation(), QueryRelation::Intersects))
  }

  fn test_within_query(
    &self,
    query: &impl Component2D,
    fields: &[Field],
  ) -> Result<WithinRelation> {
    let mut answer = WithinRelation::Disjoint;
    let mut decoded_triangle = DecodedTriangle::default();

    for field in fields {
      let relation = match field.binary_value()? {
        Some(binary_value) => {
          shape_field::decode_triangle(&binary_value.as_ref().bytes, &mut decoded_triangle)?;

          match decoded_triangle.type_ {
            DecodedTriangleType::Point => {
              let y = self.encoder().decode_y(decoded_triangle.a_y);
              let x = self.encoder().decode_x(decoded_triangle.a_x);
              query.within_point(x, y)?
            },
            DecodedTriangleType::Line => {
              let a_y = self.encoder().decode_y(decoded_triangle.a_y);
              let a_x = self.encoder().decode_x(decoded_triangle.a_x);
              let b_y = self.encoder().decode_y(decoded_triangle.b_y);
              let b_x = self.encoder().decode_x(decoded_triangle.b_x);
              query.within_line_values(a_x, a_y, decoded_triangle.ab, b_x, b_y)?
            },
            DecodedTriangleType::Triangle => {
              let a_y = self.encoder().decode_y(decoded_triangle.a_y);
              let a_x = self.encoder().decode_x(decoded_triangle.a_x);
              let b_y = self.encoder().decode_y(decoded_triangle.b_y);
              let b_x = self.encoder().decode_x(decoded_triangle.b_x);
              let c_y = self.encoder().decode_y(decoded_triangle.c_y);
              let c_x = self.encoder().decode_x(decoded_triangle.c_x);
              query.within_triangle_values(
                a_x,
                a_y,
                decoded_triangle.ab,
                b_x,
                b_y,
                decoded_triangle.bc,
                c_x,
                c_y,
                decoded_triangle.ca,
              )?
            },
          }
        },
        None => {
          return Err(LuceneError::illegal_argument(
            "field.binary_value() is None",
          ));
        },
      };

      if relation == WithinRelation::NotWithin {
        return Ok(relation);
      } else if relation == WithinRelation::Candidate {
        answer = WithinRelation::Candidate;
      }
    }

    Ok(answer)
  }
}

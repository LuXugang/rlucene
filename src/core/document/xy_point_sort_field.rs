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
use crate::core::document::xy_point_distance_comparator::XYPointDistanceComparator;
use crate::core::search::pruning::Pruning;
use crate::core::search::sort_field::{
  IndexSorterEnumSorter, MissingValueEnum, SortField, SortFieldType, SortFiledBase,
};
use crate::core::store::DataOutput;
use crate::core::util::CoreHelper;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
/// Sorts by distance from an origin location.
#[derive(Clone)]
pub struct XYPointSortField {
  x: f32,
  y: f32,
  pub(crate) base: SortField,
}
impl XYPointSortField {
  pub(crate) fn new<T>(field: T, x: f32, y: f32) -> Result<Self>
  where
    T: Into<String>,
  {
    let base = SortField::new(Some(field), SortFieldType::Custom)?;
    Ok(Self { x, y, base })
  }
}

impl Display for XYPointSortField {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let field = match self.base.get_field() {
      Some(f) => f,
      None => return Err(std::fmt::Error),
    };
    write!(f, "<distance:\"{}\" x={} y={}", field, self.x, self.y)?;

    if self.base.missing_value != Some(MissingValueEnum::Double(f64::INFINITY))
      && let Some(missing_value) = &self.base.missing_value
    {
      write!(f, " missingValue={}", missing_value)?;
    }

    write!(f, ">")
  }
}

impl SortFiledBase for XYPointSortField {
  fn set_missing_value<T>(&mut self, missing_value: T) -> Result<()>
  where
    T: Into<MissingValueEnum>,
  {
    self.base.missing_value = Some(missing_value.into());
    Ok(())
  }

  fn needs_scores(&self) -> bool {
    self.base.needs_scores()
  }

  type IndexSort = IndexSorterEnumSorter;

  fn get_index_sorter(&self) -> Result<Option<Self::IndexSort>> {
    self.base.get_index_sorter()
  }

  fn serialize(&self, _out: &mut impl DataOutput) -> Result<()> {
    Ok(())
  }

  type FieldComparator = XYPointDistanceComparator;

  fn get_comparator(&self, num_hits: usize, _pruning: Pruning) -> Result<Self::FieldComparator> {
    let field = self
      .base
      .get_field()
      .ok_or_else(|| LuceneError::illegal_state("field not available"))?;
    Ok(XYPointDistanceComparator::new(
      field.to_string(),
      self.x,
      self.y,
      num_hits,
    ))
  }
}

impl Hash for XYPointSortField {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.base.hash(state);
    CoreHelper::hash_bits_f32_for_primitive_eq(self.x).hash(state);
    CoreHelper::hash_bits_f32_for_primitive_eq(self.y).hash(state);
  }
}

impl PartialEq for XYPointSortField {
  fn eq(&self, other: &Self) -> bool {
    // Java compares distinct instances with primitive `==`; the identity case keeps Rust `Eq`
    // reflexive when a coordinate is NaN.
    std::ptr::eq(self, other) || (self.base == other.base && self.x == other.x && self.y == other.y)
  }
}

impl Eq for XYPointSortField {}

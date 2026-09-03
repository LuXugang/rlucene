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
use crate::core::document::lat_lon_point_distance_comparator::LatLonPointDistanceComparator;
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::search::pruning::Pruning;
use crate::core::search::sort_field::{
  IndexSorterEnumSorter, MissingValueEnum, SortField, SortFieldType, SortFiledBase,
};
use crate::core::store::DataOutput;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

/// Sorts by distance from an origin location.
#[derive(Clone)]
pub struct LatLonPointSortField {
  latitude: f64,
  longitude: f64,
  pub(crate) base: SortField,
}

impl LatLonPointSortField {
  pub(crate) fn new<T>(field: T, latitude: f64, longitude: f64) -> Result<Self>
  where
    T: Into<String>,
  {
    GeoUtils::check_latitude(latitude)?;
    GeoUtils::check_longitude(longitude)?;

    let mut base = SortField::new(Some(field), SortFieldType::Custom)?;
    base.missing_value = Some(MissingValueEnum::Double(f64::INFINITY));

    Ok(Self {
      latitude,
      longitude,
      base,
    })
  }
}

impl Display for LatLonPointSortField {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let field = match self.base.get_field() {
      Some(f) => f,
      None => return Err(std::fmt::Error),
    };
    write!(
      f,
      "<distance:\"{}\" latitude={} longitude={}",
      field, self.latitude, self.longitude
    )?;

    if self.base.missing_value != Some(MissingValueEnum::Double(f64::INFINITY))
      && let Some(missing_value) = &self.base.missing_value
    {
      write!(f, " missingValue={}", missing_value)?;
    }

    write!(f, ">")
  }
}

impl SortFiledBase for LatLonPointSortField {
  fn set_missing_value<T>(&mut self, missing_value: T) -> Result<()>
  where
    T: Into<MissingValueEnum>,
  {
    let missing_value = missing_value.into();
    if missing_value != MissingValueEnum::Double(f64::INFINITY) {
      return Err(LuceneError::illegal_argument(format!(
        "Missing value can only be f64::INFINITY (missing values last), but got {}",
        missing_value
      )));
    }
    self.base.missing_value = Some(missing_value);
    Ok(())
  }

  fn needs_scores(&self) -> bool {
    self.base.needs_scores()
  }

  type IndexSort = IndexSorterEnumSorter;

  fn get_index_sorter(&self) -> Result<Option<Self::IndexSort>> {
    self.base.get_index_sorter()
  }

  fn serialize(&self, out: &mut impl DataOutput) -> Result<()> {
    self.base.serialize(out)
  }

  type FieldComparator = LatLonPointDistanceComparator;

  fn get_comparator(&self, num_hits: usize, _pruning: Pruning) -> Result<Self::FieldComparator> {
    let field = self
      .base
      .get_field()
      .ok_or_else(|| LuceneError::illegal_state("field not available"))?;
    Ok(LatLonPointDistanceComparator::new(
      field.to_string(),
      self.latitude,
      self.longitude,
      num_hits,
    ))
  }
}

impl Hash for LatLonPointSortField {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.base.hash(state);
    self.latitude.to_bits().hash(state);
    self.longitude.to_bits().hash(state);
  }
}

impl PartialEq for LatLonPointSortField {
  fn eq(&self, other: &Self) -> bool {
    self.base == other.base
      && self.latitude.to_bits() == other.latitude.to_bits()
      && self.longitude.to_bits() == other.longitude.to_bits()
  }
}

impl Eq for LatLonPointSortField {}

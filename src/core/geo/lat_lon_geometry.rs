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
use crate::core::geo::component_tree::component_tree_util;
use crate::core::geo::geometry::Geometry;
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Lat/Lon Geometry object.
pub trait LatLonGeometry: Geometry {}
pub fn create<T>(xy_geometries: Vec<T>) -> Result<T::Component2D>
where
  T: LatLonGeometry,
{
  if xy_geometries.is_empty() {
    return Err(LuceneError::illegal_argument(
      "geometries must not be empty",
    ));
  }
  if xy_geometries.len() == 1 {
    return xy_geometries.into_iter().next().unwrap().to_component2d();
  }
  let mut components = Vec::with_capacity(xy_geometries.len());
  for geometry in xy_geometries {
    components.push(geometry.to_component2d()?);
  }
  component_tree_util::create(components)
}

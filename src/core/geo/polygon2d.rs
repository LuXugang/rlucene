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

// pub struct Polygon2D<T> where T:Component2D{
//     /// minimum Y of this geometry's bounding box area
//     min_y: f64,
//
//     /// maximum Y of this geometry's bounding box area
//     max_y: f64,
//
//     /// minimum X of this geometry's bounding box area
//     min_x: f64,
//
//     /// maximum X of this geometry's bounding box area
//     max_x: f64,
//
//     /// tree of holes, or null
//     pub(crate) holes: Option<T>,
//
//     /// Edges of the polygon represented as a 2-d interval tree.
//     tree: EdgeTree,
// }
//
// impl Polygon2D {
//     fn new(
//         min_x: f64,
//         max_x: f64,
//         min_y: f64,
//         max_y: f64,
//         x: Vec<f64>,
//         y: Vec<f64>,
//         holes: Option<Box<dyn Component2D>>,
//     ) -> Self {
//         Self {
//             min_y,
//             max_y,
//             min_x,
//             max_x,
//             holes,
//             tree: EdgeTree::create_tree(&x, &y),
//         }
//     }
//
//     fn from_xy_polygon(polygon: &XYPolygon, holes: Option<Box<dyn Component2D>>) -> Self {
//         Self::new(
//             polygon.min_x as f64,
//             polygon.max_x as f64,
//             polygon.min_y as f64,
//             polygon.max_y as f64,
//             XYEncodingUtils::float_array_to_double_array(polygon.get_poly_x()),
//             XYEncodingUtils::float_array_to_double_array(polygon.get_poly_y()),
//             holes,
//         )
//     }
//
//     fn from_polygon(polygon: &Polygon, holes: Option<Box<dyn Component2D>>) -> Self {
//         Self::new(
//             polygon.min_lon,
//             polygon.max_lon,
//             polygon.min_lat,
//             polygon.max_lat,
//             polygon.get_poly_lons(),
//             polygon.get_poly_lats(),
//             holes,
//         )
//     }
// }

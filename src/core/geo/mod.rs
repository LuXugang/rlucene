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
pub mod circle;
pub(crate) mod circle2d;
pub mod component2d;
mod component_tree;
pub(crate) mod edge_tree;
pub mod geo_encoding_utils;
pub mod geo_utils;
pub mod geometry;
pub mod lat_lon_geometry;
pub mod line;
pub mod line2d;
pub mod point;
pub(crate) mod point2d;
pub mod polygon;
pub(crate) mod polygon2d;
pub mod rectangle;
pub(crate) mod rectangle2d;
pub(crate) mod simple_geo_json_polygon_parser;
mod tessellator;
pub mod xy_circle;
pub mod xy_encoding_utils;
pub mod xy_geometry;
pub mod xy_line;
pub mod xy_point;
pub mod xy_polygon;
pub mod xy_rectangle;

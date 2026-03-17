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
/// Interface for implementations of VectorUtil support.
///
/// Ported from Lucene's VectorUtilSupport.
/// All methods use static dispatch.
pub trait VectorUtilSupport {
  /// Calculates the dot product of the given float arrays.
  fn dot_product_f32(&self, a: &[f32], b: &[f32]) -> f32;

  /// Returns the cosine similarity between the two vectors.
  fn cosine_f32(&self, v1: &[f32], v2: &[f32]) -> f32;

  /// Returns the sum of squared differences of the two vectors.
  fn square_distance_f32(&self, a: &[f32], b: &[f32]) -> f32;

  /// Returns the dot product computed over signed bytes.
  fn dot_product_u8(&self, a: &[u8], b: &[u8]) -> i32;

  /// Returns the dot product over the computed bytes, assuming the values are int4 encoded.
  fn int4_dot_product(&self, a: &[u8], apacked: bool, b: &[u8], bpacked: bool) -> i32;

  /// Returns the cosine similarity between the two byte vectors.
  fn cosine_u8(&self, a: &[u8], b: &[u8]) -> f32;

  /// Returns the sum of squared differences of the two byte vectors.
  fn square_distance_u8(&self, a: &[u8], b: &[u8]) -> i32;

  /// Given an array `buffer` that is sorted between indexes `0` inclusive and `to`
  /// exclusive, find the first array index whose value is greater than or equal
  /// to `target`.
  ///
  /// This index is guaranteed to be at least `from`.
  /// If there is no such array index, `to` is returned.
  fn find_next_geq(&self, buffer: &[i32], target: i32, from: usize, to: usize) -> usize;
}

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
use crate::core::internal::vectorization::vector_util_support::VectorUtilSupport;
#[derive(Default)]
pub struct DefaultVectorUtilSupport;
impl VectorUtilSupport for DefaultVectorUtilSupport {
  fn dot_product_f32(&self, _a: &[f32], _b: &[f32]) -> f32 {
    todo!()
  }

  fn cosine_f32(&self, _v1: &[f32], _v2: &[f32]) -> f32 {
    todo!()
  }

  fn square_distance_f32(&self, _a: &[f32], _b: &[f32]) -> f32 {
    todo!()
  }

  fn dot_product_u8(&self, _a: &[u8], _b: &[u8]) -> i32 {
    todo!()
  }

  fn int4_dot_product(&self, _a: &[u8], _apacked: bool, _b: &[u8], _bpacked: bool) -> i32 {
    todo!()
  }

  fn cosine_u8(&self, _a: &[u8], _b: &[u8]) -> f32 {
    todo!()
  }

  fn square_distance_u8(&self, _a: &[u8], _b: &[u8]) -> i32 {
    todo!()
  }
  #[allow(clippy::needless_range_loop)]
  fn find_next_geq(&self, buffer: &[i32], target: i32, from: usize, to: usize) -> usize {
    for i in from..to {
      if buffer[i] >= target {
        return i;
      }
    }
    to
  }
}

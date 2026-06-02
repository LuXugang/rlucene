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
pub mod core;
mod queries;
mod sandbox;

pub(crate) fn ulp_f64(x: f64) -> f64 {
  if x.is_nan() {
    return f64::NAN;
  }
  if x.is_infinite() {
    return f64::INFINITY;
  }
  if x == 0.0 {
    return f64::from_bits(1);
  }

  let bits = x.to_bits();
  let next_bits = if x > 0.0 { bits + 1 } else { bits - 1 };
  let next = f64::from_bits(next_bits);
  (next - x).abs()
}
pub(crate) fn ulp_f32(x: f32) -> f32 {
  if x.is_nan() {
    return f32::NAN;
  }
  if x.is_infinite() {
    return f32::INFINITY;
  }
  if x == 0.0 {
    return f32::from_bits(1);
  }

  let bits = x.to_bits();
  let next_bits = if x > 0.0 { bits + 1 } else { bits - 1 };
  let next = f32::from_bits(next_bits);
  (next - x).abs()
}

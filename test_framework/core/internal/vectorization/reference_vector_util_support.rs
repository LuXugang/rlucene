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
/// Independent, test-only reference calculations; no SIMD or production helpers.
pub struct ReferenceVectorUtilSupport;

impl ReferenceVectorUtilSupport {
  pub fn dot_product_f32(&self, a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter()
      .zip(b)
      .map(|(&a, &b)| f64::from(a) * f64::from(b))
      .sum::<f64>() as f32
  }

  pub fn square_distance_f32(&self, a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter()
      .zip(b)
      .map(|(&a, &b)| {
        let difference = f64::from(a) - f64::from(b);
        difference * difference
      })
      .sum::<f64>() as f32
  }

  pub fn cosine_f32(&self, a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for (&a, &b) in a.iter().zip(b) {
      let a = f64::from(a);
      let b = f64::from(b);
      dot += a * b;
      norm_a += a * a;
      norm_b += b * b;
    }
    (dot / (norm_a * norm_b).sqrt()) as f32
  }

  pub fn dot_product_u8(&self, a: &[u8], b: &[u8]) -> i32 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).fold(0i32, |sum, (&a, &b)| {
      sum.wrapping_add(i32::from(a as i8) * i32::from(b as i8))
    })
  }

  pub fn square_distance_u8(&self, a: &[u8], b: &[u8]) -> i32 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).fold(0i32, |sum, (&a, &b)| {
      let difference = i32::from(a as i8) - i32::from(b as i8);
      sum.wrapping_add(difference * difference)
    })
  }

  pub fn cosine_u8(&self, a: &[u8], b: &[u8]) -> f32 {
    let dot = f64::from(self.dot_product_u8(a, b));
    let norm_a = f64::from(self.dot_product_u8(a, a));
    let norm_b = f64::from(self.dot_product_u8(b, b));
    (dot / (norm_a * norm_b).sqrt()) as f32
  }

  pub fn int4_dot_product(&self, a: &[u8], apacked: bool, b: &[u8], bpacked: bool) -> i32 {
    assert!(!(apacked && bpacked));
    let unpack = |values: &[u8], packed: bool| {
      if packed {
        values
          .iter()
          .map(|v| v >> 4)
          .chain(values.iter().map(|v| v & 15))
          .collect::<Vec<_>>()
      } else {
        values.to_vec()
      }
    };
    self.dot_product_u8(&unpack(a, apacked), &unpack(b, bpacked))
  }
}

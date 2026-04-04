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
  fn dot_product_f32(&self, a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());

    let mut res = 0.0f32;
    let mut i = 0;

    if a.len() > 32 {
      let mut acc1 = 0.0f32;
      let mut acc2 = 0.0f32;
      let mut acc3 = 0.0f32;
      let mut acc4 = 0.0f32;
      let upper_bound = a.len() & !(4 - 1);
      while i < upper_bound {
        acc1 = fma_f32(a[i], b[i], acc1);
        acc2 = fma_f32(a[i + 1], b[i + 1], acc2);
        acc3 = fma_f32(a[i + 2], b[i + 2], acc3);
        acc4 = fma_f32(a[i + 3], b[i + 3], acc4);
        i += 4;
      }
      res += acc1 + acc2 + acc3 + acc4;
    }

    while i < a.len() {
      res = fma_f32(a[i], b[i], res);
      i += 1;
    }

    res
  }

  fn cosine_f32(&self, a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());

    let mut sum = 0.0f32;
    let mut norm1 = 0.0f32;
    let mut norm2 = 0.0f32;
    let mut i = 0;

    if a.len() > 32 {
      let mut sum1 = 0.0f32;
      let mut sum2 = 0.0f32;
      let mut norm1_1 = 0.0f32;
      let mut norm1_2 = 0.0f32;
      let mut norm2_1 = 0.0f32;
      let mut norm2_2 = 0.0f32;

      let upper_bound = a.len() & !(2 - 1);
      while i < upper_bound {
        sum1 = fma_f32(a[i], b[i], sum1);
        norm1_1 = fma_f32(a[i], a[i], norm1_1);
        norm2_1 = fma_f32(b[i], b[i], norm2_1);

        sum2 = fma_f32(a[i + 1], b[i + 1], sum2);
        norm1_2 = fma_f32(a[i + 1], a[i + 1], norm1_2);
        norm2_2 = fma_f32(b[i + 1], b[i + 1], norm2_2);

        i += 2;
      }

      sum += sum1 + sum2;
      norm1 += norm1_1 + norm1_2;
      norm2 += norm2_1 + norm2_2;
    }

    while i < a.len() {
      sum = fma_f32(a[i], b[i], sum);
      norm1 = fma_f32(a[i], a[i], norm1);
      norm2 = fma_f32(b[i], b[i], norm2);
      i += 1;
    }

    (sum as f64 / ((norm1 as f64) * (norm2 as f64)).sqrt()) as f32
  }

  fn square_distance_f32(&self, a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());

    let mut res = 0.0f32;
    let mut i = 0;

    if a.len() > 32 {
      let mut acc1 = 0.0f32;
      let mut acc2 = 0.0f32;
      let mut acc3 = 0.0f32;
      let mut acc4 = 0.0f32;
      let upper_bound = a.len() & !(4 - 1);

      while i < upper_bound {
        let diff1 = a[i] - b[i];
        acc1 = fma_f32(diff1, diff1, acc1);

        let diff2 = a[i + 1] - b[i + 1];
        acc2 = fma_f32(diff2, diff2, acc2);

        let diff3 = a[i + 2] - b[i + 2];
        acc3 = fma_f32(diff3, diff3, acc3);

        let diff4 = a[i + 3] - b[i + 3];
        acc4 = fma_f32(diff4, diff4, acc4);

        i += 4;
      }

      res += acc1 + acc2 + acc3 + acc4;
    }

    while i < a.len() {
      let diff = a[i] - b[i];
      res = fma_f32(diff, diff, res);
      i += 1;
    }

    res
  }

  fn dot_product_u8(&self, a: &[u8], b: &[u8]) -> i32 {
    debug_assert_eq!(a.len(), b.len());

    let mut total = 0;
    for i in 0..a.len() {
      total += signed_byte(a[i]) * signed_byte(b[i]);
    }
    total
  }

  fn int4_dot_product(&self, a: &[u8], apacked: bool, b: &[u8], bpacked: bool) -> i32 {
    debug_assert!(!(apacked && bpacked));

    if apacked || bpacked {
      let (packed, unpacked) = if apacked { (a, b) } else { (b, a) };
      let mut total = 0;

      for i in 0..packed.len() {
        let packed_byte = packed[i];
        let unpacked1 = signed_byte(unpacked[i]);
        let unpacked2 = signed_byte(unpacked[i + packed.len()]);
        total += i32::from(packed_byte & 0x0F) * unpacked2;
        total += i32::from(packed_byte >> 4) * unpacked1;
      }

      return total;
    }

    self.dot_product_u8(a, b)
  }

  fn cosine_u8(&self, a: &[u8], b: &[u8]) -> f32 {
    debug_assert_eq!(a.len(), b.len());

    let mut sum = 0;
    let mut norm1 = 0;
    let mut norm2 = 0;

    for i in 0..a.len() {
      let elem1 = signed_byte(a[i]);
      let elem2 = signed_byte(b[i]);
      sum += elem1 * elem2;
      norm1 += elem1 * elem1;
      norm2 += elem2 * elem2;
    }

    (sum as f64 / ((norm1 as f64) * (norm2 as f64)).sqrt()) as f32
  }

  fn square_distance_u8(&self, a: &[u8], b: &[u8]) -> i32 {
    debug_assert_eq!(a.len(), b.len());

    let mut square_sum = 0;
    for i in 0..a.len() {
      let diff = signed_byte(a[i]) - signed_byte(b[i]);
      square_sum += diff * diff;
    }
    square_sum
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
#[inline]
fn fma_f32(a: f32, b: f32, c: f32) -> f32 {
  a.mul_add(b, c)
}

#[inline]
fn signed_byte(v: u8) -> i32 {
  i32::from(v as i8)
}

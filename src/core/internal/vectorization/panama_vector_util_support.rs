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
use wide::{i16x8, i16x16, i16x32, i32x4, i32x8, i32x16, u8x16};

// This standalone calculation backend uses the features of the compilation target.
// Runtime CPU/provider selection belongs to the provider integration, not this class.
#[cfg(not(any(
  target_feature = "avx2",
  all(target_feature = "avx512f", target_feature = "avx512bw")
)))]
use wide::{f32x4 as FloatVector, i32x4 as IntVector};
#[cfg(all(
  target_feature = "avx2",
  not(all(target_feature = "avx512f", target_feature = "avx512bw"))
))]
use wide::{f32x8 as FloatVector, i32x8 as IntVector};
#[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
use wide::{f32x16 as FloatVector, i32x16 as IntVector};

/// Array calculations corresponding to Lucene 10.1's `PanamaVectorUtilSupport`.
///
/// Uses safe `wide` operations. Byte slices also represent the data consumed by
/// Java's MemorySegment overloads; acquiring mmap slices is a separate concern.
/// This implementation is not yet selected by the production provider.
#[derive(Default)]
pub struct PanamaVectorUtilSupport;

impl PanamaVectorUtilSupport {
  /// Compile-time vector width, not a runtime CPU capability probe.
  pub const VECTOR_BITSIZE: usize = size_of::<FloatVector>() * 8;
  const LANES: usize = Self::VECTOR_BITSIZE / 32;
  // Match Java's avoidance of 128-bit integer vectors on x86-64.
  const HAS_FAST_INTEGER_VECTORS: bool =
    !cfg!(target_arch = "x86_64") || Self::VECTOR_BITSIZE >= 256;
  // Conservative compile-time FMA policy. Full CPU/vendor policy is deferred to
  // provider integration. In particular, Java 10.1 avoids FMA on Apple Silicon.
  const HAS_FAST_FMA: bool = cfg!(target_feature = "fma")
    || cfg!(all(
      target_arch = "aarch64",
      target_feature = "neon",
      not(target_os = "macos")
    ));

  fn fma_vector(a: FloatVector, b: FloatVector, c: FloatVector) -> FloatVector {
    if Self::HAS_FAST_FMA {
      a.mul_add(b, c)
    } else {
      a * b + c
    }
  }

  fn fma_scalar(a: f32, b: f32, c: f32) -> f32 {
    if Self::HAS_FAST_FMA {
      a.mul_add(b, c)
    } else {
      a * b + c
    }
  }

  #[allow(
    clippy::chunks_exact_to_as_chunks,
    reason = "Keep the benchmarked chunk iteration; array-chunk codegen needs separate validation"
  )]
  fn dot_product_body(a: &[f32], b: &[f32], limit: usize) -> f32 {
    let mut i = 0;
    let mut acc1 = FloatVector::splat(0.0);
    let mut acc2 = FloatVector::splat(0.0);
    let mut acc3 = FloatVector::splat(0.0);
    let mut acc4 = FloatVector::splat(0.0);
    // Complete paired blocks make the per-vector bounds known. The tail below
    // still checks every remaining vector, including when b is too short.
    for (a_block, b_block) in a[..limit]
      .chunks_exact(4 * Self::LANES)
      .zip(b.chunks_exact(4 * Self::LANES))
    {
      let va = FloatVector::from(&a_block[..Self::LANES]);
      let vb = FloatVector::from(&b_block[..Self::LANES]);
      acc1 = Self::fma_vector(va, vb, acc1);
      let va = FloatVector::from(&a_block[Self::LANES..2 * Self::LANES]);
      let vb = FloatVector::from(&b_block[Self::LANES..2 * Self::LANES]);
      acc2 = Self::fma_vector(va, vb, acc2);
      let va = FloatVector::from(&a_block[2 * Self::LANES..3 * Self::LANES]);
      let vb = FloatVector::from(&b_block[2 * Self::LANES..3 * Self::LANES]);
      acc3 = Self::fma_vector(va, vb, acc3);
      let va = FloatVector::from(&a_block[3 * Self::LANES..4 * Self::LANES]);
      let vb = FloatVector::from(&b_block[3 * Self::LANES..4 * Self::LANES]);
      acc4 = Self::fma_vector(va, vb, acc4);
      i += 4 * Self::LANES;
    }
    while i < limit {
      let va = FloatVector::from(&a[i..i + Self::LANES]);
      let vb = FloatVector::from(&b[i..i + Self::LANES]);
      acc1 = Self::fma_vector(va, vb, acc1);
      i += Self::LANES;
    }
    ((acc1 + acc2) + (acc3 + acc4)).reduce_add()
  }

  #[allow(
    clippy::chunks_exact_to_as_chunks,
    reason = "Keep the benchmarked chunk iteration; array-chunk codegen needs separate validation"
  )]
  fn square_distance_body(a: &[f32], b: &[f32], limit: usize) -> f32 {
    let mut i = 0;
    let mut acc1 = FloatVector::splat(0.0);
    let mut acc2 = FloatVector::splat(0.0);
    let mut acc3 = FloatVector::splat(0.0);
    let mut acc4 = FloatVector::splat(0.0);
    // Complete paired blocks make the per-vector bounds known. The tail below
    // still checks every remaining vector, including when b is too short.
    for (a_block, b_block) in a[..limit]
      .chunks_exact(4 * Self::LANES)
      .zip(b.chunks_exact(4 * Self::LANES))
    {
      let va = FloatVector::from(&a_block[..Self::LANES]);
      let vb = FloatVector::from(&b_block[..Self::LANES]);
      let diff = va - vb;
      acc1 = Self::fma_vector(diff, diff, acc1);
      let va = FloatVector::from(&a_block[Self::LANES..2 * Self::LANES]);
      let vb = FloatVector::from(&b_block[Self::LANES..2 * Self::LANES]);
      let diff = va - vb;
      acc2 = Self::fma_vector(diff, diff, acc2);
      let va = FloatVector::from(&a_block[2 * Self::LANES..3 * Self::LANES]);
      let vb = FloatVector::from(&b_block[2 * Self::LANES..3 * Self::LANES]);
      let diff = va - vb;
      acc3 = Self::fma_vector(diff, diff, acc3);
      let va = FloatVector::from(&a_block[3 * Self::LANES..4 * Self::LANES]);
      let vb = FloatVector::from(&b_block[3 * Self::LANES..4 * Self::LANES]);
      let diff = va - vb;
      acc4 = Self::fma_vector(diff, diff, acc4);
      i += 4 * Self::LANES;
    }
    while i < limit {
      let va = FloatVector::from(&a[i..i + Self::LANES]);
      let vb = FloatVector::from(&b[i..i + Self::LANES]);
      let diff = va - vb;
      acc1 = Self::fma_vector(diff, diff, acc1);
      i += Self::LANES;
    }
    ((acc1 + acc2) + (acc3 + acc4)).reduce_add()
  }

  #[allow(
    clippy::chunks_exact_to_as_chunks,
    reason = "Keep the benchmarked chunk iteration; array-chunk codegen needs separate validation"
  )]
  fn cosine_body(a: &[f32], b: &[f32], limit: usize) -> [f32; 3] {
    let mut i = 0;
    let mut sum1 = FloatVector::splat(0.0);
    let mut sum2 = FloatVector::splat(0.0);
    let mut norm1_1 = FloatVector::splat(0.0);
    let mut norm1_2 = FloatVector::splat(0.0);
    let mut norm2_1 = FloatVector::splat(0.0);
    let mut norm2_2 = FloatVector::splat(0.0);
    // Complete paired blocks make the per-vector bounds known. The tail below
    // still checks every remaining vector, including when b is too short.
    for (a_block, b_block) in a[..limit]
      .chunks_exact(2 * Self::LANES)
      .zip(b.chunks_exact(2 * Self::LANES))
    {
      let va = FloatVector::from(&a_block[..Self::LANES]);
      let vb = FloatVector::from(&b_block[..Self::LANES]);
      sum1 = Self::fma_vector(va, vb, sum1);
      norm1_1 = Self::fma_vector(va, va, norm1_1);
      norm2_1 = Self::fma_vector(vb, vb, norm2_1);
      let vc = FloatVector::from(&a_block[Self::LANES..2 * Self::LANES]);
      let vd = FloatVector::from(&b_block[Self::LANES..2 * Self::LANES]);
      sum2 = Self::fma_vector(vc, vd, sum2);
      norm1_2 = Self::fma_vector(vc, vc, norm1_2);
      norm2_2 = Self::fma_vector(vd, vd, norm2_2);
      i += 2 * Self::LANES;
    }
    while i < limit {
      let va = FloatVector::from(&a[i..i + Self::LANES]);
      let vb = FloatVector::from(&b[i..i + Self::LANES]);
      sum1 = Self::fma_vector(va, vb, sum1);
      norm1_1 = Self::fma_vector(va, va, norm1_1);
      norm2_1 = Self::fma_vector(vb, vb, norm2_1);
      i += Self::LANES;
    }
    [
      (sum1 + sum2).reduce_add(),
      (norm1_1 + norm1_2).reduce_add(),
      (norm2_1 + norm2_2).reduce_add(),
    ]
  }

  fn dot_product_body_128(a: &[u8], b: &[u8], limit: usize) -> i32 {
    let mut sum = i32x4::splat(0);
    for i in (0..limit).step_by(4) {
      let a = &a[i..i + 8];
      let b = &b[i..i + 8];
      let va = i16x8::new(std::array::from_fn(|j| i16::from(a[j] as i8)));
      let vb = i16x8::new(std::array::from_fn(|j| i16::from(b[j] as i8)));
      let product = (va * vb).to_array();
      sum += i32x4::new(std::array::from_fn(|j| i32::from(product[j])));
    }
    sum.reduce_add()
  }

  fn cosine_body_128(a: &[u8], b: &[u8], limit: usize) -> [f32; 3] {
    let mut sum = i32x4::splat(0);
    let mut norm1 = i32x4::splat(0);
    let mut norm2 = i32x4::splat(0);
    for i in (0..limit).step_by(4) {
      let a = &a[i..i + 8];
      let b = &b[i..i + 8];
      let va = i16x8::new(std::array::from_fn(|j| i16::from(a[j] as i8)));
      let vb = i16x8::new(std::array::from_fn(|j| i16::from(b[j] as i8)));
      let product = (va * vb).to_array();
      sum += i32x4::new(std::array::from_fn(|j| i32::from(product[j])));
      let product = (va * va).to_array();
      norm1 += i32x4::new(std::array::from_fn(|j| i32::from(product[j])));
      let product = (vb * vb).to_array();
      norm2 += i32x4::new(std::array::from_fn(|j| i32::from(product[j])));
    }
    [
      sum.reduce_add() as f32,
      norm1.reduce_add() as f32,
      norm2.reduce_add() as f32,
    ]
  }

  fn dot_product_body_256(a: &[u8], b: &[u8], limit: usize) -> i32 {
    let mut sum = i32x8::splat(0);
    for i in (0..limit).step_by(8) {
      let a = &a[i..i + 8];
      let b = &b[i..i + 8];
      let va = i32x8::new(std::array::from_fn(|j| i32::from(a[j] as i8)));
      let vb = i32x8::new(std::array::from_fn(|j| i32::from(b[j] as i8)));
      sum += va * vb;
    }
    sum.reduce_add()
  }

  fn cosine_body_256(a: &[u8], b: &[u8], limit: usize) -> [f32; 3] {
    let mut sum = i32x8::splat(0);
    let mut norm1 = i32x8::splat(0);
    let mut norm2 = i32x8::splat(0);
    for i in (0..limit).step_by(8) {
      let a = &a[i..i + 8];
      let b = &b[i..i + 8];
      let va = i32x8::new(std::array::from_fn(|j| i32::from(a[j] as i8)));
      let vb = i32x8::new(std::array::from_fn(|j| i32::from(b[j] as i8)));
      sum += va * vb;
      norm1 += va * va;
      norm2 += vb * vb;
    }
    [
      sum.reduce_add() as f32,
      norm1.reduce_add() as f32,
      norm2.reduce_add() as f32,
    ]
  }

  fn dot_product_body_512(a: &[u8], b: &[u8], limit: usize) -> i32 {
    let mut sum = i32x16::splat(0);
    for i in (0..limit).step_by(16) {
      let a = &a[i..i + 16];
      let b = &b[i..i + 16];
      let va = i16x16::new(std::array::from_fn(|j| i16::from(a[j] as i8)));
      let vb = i16x16::new(std::array::from_fn(|j| i16::from(b[j] as i8)));
      let product = (va * vb).to_array();
      sum += i32x16::new(std::array::from_fn(|j| i32::from(product[j])));
    }
    sum.reduce_add()
  }

  fn cosine_body_512(a: &[u8], b: &[u8], limit: usize) -> [f32; 3] {
    let mut sum = i32x16::splat(0);
    let mut norm1 = i32x16::splat(0);
    let mut norm2 = i32x16::splat(0);
    for i in (0..limit).step_by(16) {
      let a = &a[i..i + 16];
      let b = &b[i..i + 16];
      let va = i16x16::new(std::array::from_fn(|j| i16::from(a[j] as i8)));
      let vb = i16x16::new(std::array::from_fn(|j| i16::from(b[j] as i8)));
      let product = (va * vb).to_array();
      sum += i32x16::new(std::array::from_fn(|j| i32::from(product[j])));
      let product = (va * va).to_array();
      norm1 += i32x16::new(std::array::from_fn(|j| i32::from(product[j])));
      let product = (vb * vb).to_array();
      norm2 += i32x16::new(std::array::from_fn(|j| i32::from(product[j])));
    }
    [
      sum.reduce_add() as f32,
      norm1.reduce_add() as f32,
      norm2.reduce_add() as f32,
    ]
  }

  fn dot_product_body_128_int4_packed(unpacked: &[u8], packed: &[u8], limit: usize) -> i32 {
    const LOW_NIBBLE_MASK: u8x16 = u8x16::splat(0x0f);

    let mut sum = 0i32;
    for i in (0..limit).step_by(1024) {
      let mut acc0 = i16x8::splat(0);
      let mut acc1 = i16x8::splat(0);
      let inner_limit = (limit - i).min(1024);
      let mut j = 0;
      while j + 16 <= inner_limit {
        // Keep the original sequence of 8-byte bounds checks before combining
        // two adjacent groups into a full byte vector.
        let packed0 = &packed[i + j..i + j + 8];
        let lower0 = &unpacked[i + j + packed.len()..i + j + packed.len() + 8];
        let upper0 = &unpacked[i + j..i + j + 8];
        let packed1 = &packed[i + j + 8..i + j + 16];
        let lower1 = &unpacked[i + j + packed.len() + 8..i + j + packed.len() + 16];
        let upper1 = &unpacked[i + j + 8..i + j + 16];
        let vb = u8x16::new(std::array::from_fn(|k| {
          if k < 8 { packed0[k] } else { packed1[k - 8] }
        }));
        let va = u8x16::new(std::array::from_fn(|k| {
          if k < 8 { lower0[k] } else { lower1[k - 8] }
        }));
        let vc = u8x16::new(std::array::from_fn(|k| {
          if k < 8 { upper0[k] } else { upper1[k - 8] }
        }));
        // Java multiplies bytes before zero-extending, including inputs
        // outside the unsigned int4 range.
        let low_product = (vb & LOW_NIBBLE_MASK) * va;
        let high_product = (vb >> 4u32) * vc;
        acc0 += i16x8::from_u8x16_low(low_product);
        acc0 += i16x8::from_u8x16_high(low_product);
        acc1 += i16x8::from_u8x16_low(high_product);
        acc1 += i16x8::from_u8x16_high(high_product);
        j += 16;
      }
      if j < inner_limit {
        let packed_block = &packed[i + j..i + j + 8];
        let lower = &unpacked[i + j + packed.len()..i + j + packed.len() + 8];
        let upper = &unpacked[i + j..i + j + 8];
        let vb = u8x16::from(packed_block);
        acc0 += i16x8::from_u8x16_low((vb & LOW_NIBBLE_MASK) * u8x16::from(lower));
        acc1 += i16x8::from_u8x16_low((vb >> 4u32) * u8x16::from(upper));
      }
      let a = acc0.to_array();
      let b = acc1.to_array();
      let int_acc0 = i32x4::new(std::array::from_fn(|j| i32::from(a[j])));
      let int_acc1 = i32x4::new(std::array::from_fn(|j| i32::from(a[j + 4])));
      let int_acc2 = i32x4::new(std::array::from_fn(|j| i32::from(b[j])));
      let int_acc3 = i32x4::new(std::array::from_fn(|j| i32::from(b[j + 4])));
      sum = sum.wrapping_add((int_acc0 + int_acc1 + int_acc2 + int_acc3).reduce_add());
    }
    sum
  }

  fn dot_product_body_256_int4_packed(unpacked: &[u8], packed: &[u8], limit: usize) -> i32 {
    let mut sum = 0i32;
    for i in (0..limit).step_by(2048) {
      let mut acc0 = i16x16::splat(0);
      let mut acc1 = i16x16::splat(0);
      let inner_limit = (limit - i).min(2048);
      for j in (0..inner_limit).step_by(16) {
        let packed_block = &packed[i + j..i + j + 16];
        let lower = &unpacked[i + j + packed.len()..i + j + packed.len() + 16];
        let upper = &unpacked[i + j..i + j + 16];
        let vb = i16x16::new(std::array::from_fn(|k| i16::from(packed_block[k])));
        let va = i16x16::new(std::array::from_fn(|k| i16::from(lower[k] as i8)));
        let vc = i16x16::new(std::array::from_fn(|k| i16::from(upper[k] as i8)));
        // Java multiplies bytes before zero-extending. Keep that truncation even
        // for inputs outside the usual unsigned 4-bit value range.
        acc0 += ((vb & i16x16::splat(0x0f)) * va) & i16x16::splat(0xff);
        acc1 += ((vb >> 4u32) * vc) & i16x16::splat(0xff);
      }
      let a = acc0.to_array();
      let b = acc1.to_array();
      let int_acc0 = i32x8::new(std::array::from_fn(|j| i32::from(a[j])));
      let int_acc1 = i32x8::new(std::array::from_fn(|j| i32::from(a[j + 8])));
      let int_acc2 = i32x8::new(std::array::from_fn(|j| i32::from(b[j])));
      let int_acc3 = i32x8::new(std::array::from_fn(|j| i32::from(b[j + 8])));
      sum = sum.wrapping_add((int_acc0 + int_acc1 + int_acc2 + int_acc3).reduce_add());
    }
    sum
  }

  fn dot_product_body_512_int4_packed(unpacked: &[u8], packed: &[u8], limit: usize) -> i32 {
    let mut sum = 0i32;
    for i in (0..limit).step_by(4096) {
      let mut acc0 = i16x32::splat(0);
      let mut acc1 = i16x32::splat(0);
      let inner_limit = (limit - i).min(4096);
      for j in (0..inner_limit).step_by(32) {
        let packed_block = &packed[i + j..i + j + 32];
        let lower = &unpacked[i + j + packed.len()..i + j + packed.len() + 32];
        let upper = &unpacked[i + j..i + j + 32];
        let vb = i16x32::new(std::array::from_fn(|k| i16::from(packed_block[k])));
        let va = i16x32::new(std::array::from_fn(|k| i16::from(lower[k] as i8)));
        let vc = i16x32::new(std::array::from_fn(|k| i16::from(upper[k] as i8)));
        // Java multiplies bytes before zero-extending. Keep that truncation even
        // for inputs outside the usual unsigned 4-bit value range.
        acc0 += ((vb & i16x32::splat(0x0f)) * va) & i16x32::splat(0xff);
        acc1 += ((vb >> 4u32) * vc) & i16x32::splat(0xff);
      }
      let a = acc0.to_array();
      let b = acc1.to_array();
      let int_acc0 = i32x16::new(std::array::from_fn(|j| i32::from(a[j])));
      let int_acc1 = i32x16::new(std::array::from_fn(|j| i32::from(a[j + 16])));
      let int_acc2 = i32x16::new(std::array::from_fn(|j| i32::from(b[j])));
      let int_acc3 = i32x16::new(std::array::from_fn(|j| i32::from(b[j + 16])));
      sum = sum.wrapping_add((int_acc0 + int_acc1 + int_acc2 + int_acc3).reduce_add());
    }
    sum
  }

  fn int4_dot_product_body_128(a: &[u8], b: &[u8], limit: usize) -> i32 {
    let mut sum = 0i32;
    for i in (0..limit).step_by(1024) {
      let mut acc0 = i16x8::splat(0);
      let mut acc1 = i16x8::splat(0);
      let inner_limit = (limit - i).min(1024);
      for j in (0..inner_limit).step_by(16) {
        let a = &a[i + j..i + j + 16];
        let b = &b[i + j..i + j + 16];
        // Byte multiplication keeps the low 8 bits, matching Java before
        // zero-extension, including values outside the unsigned int4 range.
        let product = u8x16::from(a) * u8x16::from(b);
        acc0 += i16x8::from_u8x16_low(product);
        acc1 += i16x8::from_u8x16_high(product);
      }
      let a = acc0.to_array();
      let b = acc1.to_array();
      let int_acc0 = i32x4::new(std::array::from_fn(|j| i32::from(a[j])));
      let int_acc1 = i32x4::new(std::array::from_fn(|j| i32::from(a[j + 4])));
      let int_acc2 = i32x4::new(std::array::from_fn(|j| i32::from(b[j])));
      let int_acc3 = i32x4::new(std::array::from_fn(|j| i32::from(b[j + 4])));
      sum = sum.wrapping_add((int_acc0 + int_acc1 + int_acc2 + int_acc3).reduce_add());
    }
    sum
  }

  // Java's 256-bit body also serves 512-bit vectors using the preferred species.
  fn square_distance_body_256(a: &[u8], b: &[u8], limit: usize) -> i32 {
    let mut acc = IntVector::splat(0);
    for i in (0..limit).step_by(Self::LANES) {
      let a = &a[i..i + Self::LANES];
      let b = &b[i..i + Self::LANES];
      let va = IntVector::new(std::array::from_fn(|j| i32::from(a[j] as i8)));
      let vb = IntVector::new(std::array::from_fn(|j| i32::from(b[j] as i8)));
      let diff = va - vb;
      acc += diff * diff;
    }
    acc.reduce_add()
  }

  fn square_distance_body_128(a: &[u8], b: &[u8], limit: usize) -> i32 {
    let mut acc1 = i32x4::splat(0);
    let mut acc2 = i32x4::splat(0);
    for i in (0..limit).step_by(8) {
      let a = &a[i..i + 8];
      let b = &b[i..i + 8];
      let va = i16x8::new(std::array::from_fn(|j| i16::from(a[j] as i8)));
      let vb = i16x8::new(std::array::from_fn(|j| i16::from(b[j] as i8)));
      let diff = (va - vb).to_array();
      let diff1 = i32x4::new(std::array::from_fn(|j| i32::from(diff[j])));
      let diff2 = i32x4::new(std::array::from_fn(|j| i32::from(diff[j + 4])));
      acc1 += diff1 * diff1;
      acc2 += diff2 * diff2;
    }
    (acc1 + acc2).reduce_add()
  }

  /// Java MemorySegment overload; accepts borrowed array or mapped bytes.
  pub fn dot_product_memory_segment(a: &[u8], b: &[u8]) -> i32 {
    debug_assert_eq!(a.len(), b.len());
    let mut i = 0;
    let mut res = 0i32;
    if a.len() >= 16 && Self::HAS_FAST_INTEGER_VECTORS {
      if Self::VECTOR_BITSIZE >= 512 {
        i = a.len() / 16 * 16;
        res = res.wrapping_add(Self::dot_product_body_512(a, b, i));
      } else if Self::VECTOR_BITSIZE == 256 {
        i = a.len() / 8 * 8;
        res = res.wrapping_add(Self::dot_product_body_256(a, b, i));
      } else {
        i = (a.len() - 8) / 8 * 8;
        res = res.wrapping_add(Self::dot_product_body_128(a, b, i));
      }
    }
    while i < a.len() {
      res = res.wrapping_add(i32::from(b[i] as i8) * i32::from(a[i] as i8));
      i += 1;
    }
    res
  }

  /// Java MemorySegment overload; accepts borrowed array or mapped bytes.
  pub fn cosine_memory_segment(a: &[u8], b: &[u8]) -> f32 {
    let mut i = 0;
    let mut sum = 0i32;
    let mut norm1 = 0i32;
    let mut norm2 = 0i32;
    if a.len() >= 16 && Self::HAS_FAST_INTEGER_VECTORS {
      let ret = if Self::VECTOR_BITSIZE >= 512 {
        i = a.len() / 16 * 16;
        Self::cosine_body_512(a, b, i)
      } else if Self::VECTOR_BITSIZE == 256 {
        i = a.len() / 8 * 8;
        Self::cosine_body_256(a, b, i)
      } else {
        i = (a.len() - 8) / 8 * 8;
        Self::cosine_body_128(a, b, i)
      };
      // Java returns float[] from the body, then applies int += float. Preserve
      // both the rounding to float and the saturating narrowing back to int.
      sum = (sum as f32 + ret[0]) as i32;
      norm1 = (norm1 as f32 + ret[1]) as i32;
      norm2 = (norm2 as f32 + ret[2]) as i32;
    }
    while i < a.len() {
      let a = i32::from(a[i] as i8);
      let b = i32::from(b[i] as i8);
      sum = sum.wrapping_add(a * b);
      norm1 = norm1.wrapping_add(a * a);
      norm2 = norm2.wrapping_add(b * b);
      i += 1;
    }
    (f64::from(sum) / (f64::from(norm1) * f64::from(norm2)).sqrt()) as f32
  }

  /// Java MemorySegment overload; accepts borrowed array or mapped bytes.
  pub fn square_distance_memory_segment(a: &[u8], b: &[u8]) -> i32 {
    debug_assert_eq!(a.len(), b.len());
    let mut i = 0;
    let mut res = 0i32;
    if a.len() >= 16 && Self::HAS_FAST_INTEGER_VECTORS {
      if Self::VECTOR_BITSIZE >= 256 {
        i = a.len() / Self::LANES * Self::LANES;
        res = res.wrapping_add(Self::square_distance_body_256(a, b, i));
      } else {
        i = a.len() / 8 * 8;
        res = res.wrapping_add(Self::square_distance_body_128(a, b, i));
      }
    }
    while i < a.len() {
      let diff = i32::from(a[i] as i8) - i32::from(b[i] as i8);
      res = res.wrapping_add(diff * diff);
      i += 1;
    }
    res
  }
}

impl VectorUtilSupport for PanamaVectorUtilSupport {
  fn dot_product_f32(&self, a: &[f32], b: &[f32]) -> f32 {
    let mut i = 0;
    let mut res = 0.0;
    if a.len() > 2 * Self::LANES {
      i = a.len() / Self::LANES * Self::LANES;
      res += Self::dot_product_body(a, b, i);
    }
    while i < a.len() {
      res = Self::fma_scalar(a[i], b[i], res);
      i += 1;
    }
    res
  }

  fn square_distance_f32(&self, a: &[f32], b: &[f32]) -> f32 {
    let mut i = 0;
    let mut res = 0.0;
    if a.len() > 2 * Self::LANES {
      i = a.len() / Self::LANES * Self::LANES;
      res += Self::square_distance_body(a, b, i);
    }
    while i < a.len() {
      let diff = a[i] - b[i];
      res = Self::fma_scalar(diff, diff, res);
      i += 1;
    }
    res
  }

  fn cosine_f32(&self, a: &[f32], b: &[f32]) -> f32 {
    let mut i = 0;
    let mut sum = 0.0;
    let mut norm1 = 0.0;
    let mut norm2 = 0.0;
    if a.len() > 2 * Self::LANES {
      i = a.len() / Self::LANES * Self::LANES;
      let ret = Self::cosine_body(a, b, i);
      sum += ret[0];
      norm1 += ret[1];
      norm2 += ret[2];
    }
    while i < a.len() {
      sum = Self::fma_scalar(a[i], b[i], sum);
      norm1 = Self::fma_scalar(a[i], a[i], norm1);
      norm2 = Self::fma_scalar(b[i], b[i], norm2);
      i += 1;
    }
    (f64::from(sum) / (f64::from(norm1) * f64::from(norm2)).sqrt()) as f32
  }

  fn dot_product_u8(&self, a: &[u8], b: &[u8]) -> i32 {
    Self::dot_product_memory_segment(a, b)
  }

  fn int4_dot_product(&self, a: &[u8], apacked: bool, b: &[u8], bpacked: bool) -> i32 {
    debug_assert!(!(apacked && bpacked));
    let mut i = 0;
    let mut res = 0i32;
    if apacked || bpacked {
      let (packed, unpacked) = if apacked { (a, b) } else { (b, a) };
      if packed.len() >= 32 {
        if Self::VECTOR_BITSIZE >= 512 {
          i = packed.len() / 32 * 32;
          res = res.wrapping_add(Self::dot_product_body_512_int4_packed(unpacked, packed, i));
        } else if Self::VECTOR_BITSIZE == 256 {
          i = packed.len() / 16 * 16;
          res = res.wrapping_add(Self::dot_product_body_256_int4_packed(unpacked, packed, i));
        } else if Self::HAS_FAST_INTEGER_VECTORS {
          i = packed.len() / 8 * 8;
          res = res.wrapping_add(Self::dot_product_body_128_int4_packed(unpacked, packed, i));
        }
      }
      while i < packed.len() {
        let packed_byte = packed[i];
        let unpacked1 = i32::from(unpacked[i] as i8);
        let unpacked2 = i32::from(unpacked[i + packed.len()] as i8);
        res = res.wrapping_add(i32::from(packed_byte & 0x0f) * unpacked2);
        res = res.wrapping_add(i32::from(packed_byte >> 4) * unpacked1);
        i += 1;
      }
    } else {
      if Self::VECTOR_BITSIZE >= 256 {
        return self.dot_product_u8(a, b);
      } else if a.len() >= 32 && Self::HAS_FAST_INTEGER_VECTORS {
        i = a.len() / 16 * 16;
        res = res.wrapping_add(Self::int4_dot_product_body_128(a, b, i));
      }
      while i < a.len() {
        res = res.wrapping_add(i32::from(b[i] as i8) * i32::from(a[i] as i8));
        i += 1;
      }
    }
    res
  }

  fn cosine_u8(&self, a: &[u8], b: &[u8]) -> f32 {
    Self::cosine_memory_segment(a, b)
  }

  fn square_distance_u8(&self, a: &[u8], b: &[u8]) -> i32 {
    Self::square_distance_memory_segment(a, b)
  }

  #[allow(clippy::needless_range_loop)]
  #[allow(
    clippy::chunks_exact_to_as_chunks,
    reason = "Keep the benchmarked chunk iteration; array-chunk codegen needs separate validation"
  )]
  fn find_next_geq(&self, buffer: &[i32], target: i32, mut from: usize, to: usize) -> usize {
    if Self::LANES >= 8 {
      while from + Self::LANES < to {
        if buffer[from + Self::LANES] >= target {
          let vector = IntVector::from(&buffer[from..from + Self::LANES]);
          let mask = vector.simd_lt(IntVector::splat(target));
          return from + mask.to_bitmask().count_ones() as usize;
        }
        from += Self::LANES + 1;
      }
    } else if from < to {
      if buffer[from] >= target {
        return from;
      }
      from += 1;
      // Only scan readable elements here. An invalid upper bound must still
      // allow an earlier match, or fail at the first unreadable scalar access.
      let end = to.min(buffer.len());
      let target_vector = i32x4::splat(target);
      // Combine two comparisons before testing for a match, then preserve
      // block order when locating the first matching lane.
      let mut blocks = buffer[from..end].chunks_exact(8);
      for block in &mut blocks {
        let first = i32x4::from(&block[..4]).simd_ge(target_vector);
        let second = i32x4::from(&block[4..]).simd_ge(target_vector);
        if (first | second).any() {
          let mask = first.to_bitmask();
          if mask != 0 {
            return from + mask.trailing_zeros() as usize;
          }
          return from + 4 + second.to_bitmask().trailing_zeros() as usize;
        }
        from += 8;
      }
      for block in blocks.remainder().chunks_exact(4) {
        let mask = i32x4::from(block).simd_ge(target_vector);
        if mask.any() {
          return from + mask.to_bitmask().trailing_zeros() as usize;
        }
        from += 4;
      }
    }
    for i in from..to {
      if buffer[i] >= target {
        return i;
      }
    }
    to
  }
}

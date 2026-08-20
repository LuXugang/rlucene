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

use crate::core::store::IndexInput;
use crate::core::util::error::lucene_error::Result;
use wide::i32x8;

/// Utility struct to decode postings.
pub struct PostingDecodingUtil<I> {
  /// The wrapped [`IndexInput`].
  pub input: I,
}

impl<I: IndexInput> PostingDecodingUtil<I> {
  /// Creates a new instance for use by implementations.
  pub fn new(input: I) -> Self {
    PostingDecodingUtil { input }
  }

  /// Core method for decoding blocks of docs / freqs / positions / offsets:
  ///
  /// - Read `count` longs into `c[c_index..]`
  /// - For all `i >= 0` such that `b_shift - i * dec > 0`:
  ///   - Apply shift `b_shift - i * dec` to each value in `c`
  ///   - Store the result in `b` at offset `count * i`
  /// - Apply mask `c_mask` to each value in `c` starting at `c_index`
  #[allow(clippy::too_many_arguments)]
  pub fn split_ints_same(
    &mut self,
    count: i32,
    b_and_c: &mut [i32],
    b_shift: i32,
    dec: i32,
    b_mask: i32,
    c_index: i32,
    c_mask: i32,
  ) -> Result<()> {
    self
      .input
      .read_ints(b_and_c, c_index as usize, count as usize)?;

    let count = count as usize;
    let c_index = c_index as usize;
    let max_iter = (b_shift - 1) / dec;
    let mask = i32x8::splat(b_mask);
    let c_mask_simd = i32x8::splat(c_mask);
    let mut i = 0;
    while i + 8 <= count {
      let values = i32x8::from(&b_and_c[c_index + i..c_index + i + 8]);
      for j in 0..=max_iter {
        let shift = b_shift - j * dec;
        if shift > 0 {
          let values: [i32; 8] = ((values >> shift as u32) & mask).into();
          let start = count * j as usize + i;
          b_and_c[start..start + 8].copy_from_slice(&values);
        }
      }
      let values: [i32; 8] = (values & c_mask_simd).into();
      b_and_c[c_index + i..c_index + i + 8].copy_from_slice(&values);
      i += 8;
    }
    while i < count {
      for j in 0..=max_iter {
        let shift = b_shift - j * dec;
        if shift > 0 {
          b_and_c[count * j as usize + i] =
            ((b_and_c[c_index + i] as u64) >> shift) as i32 & b_mask;
        }
      }
      b_and_c[c_index + i] &= c_mask;
      i += 1;
    }

    Ok(())
  }

  #[allow(clippy::too_many_arguments)]
  pub fn split_ints_diff(
    &mut self,
    count: i32,
    b: &mut [i32],
    b_shift: i32,
    dec: i32,
    b_mask: i32,
    c: &mut [i32],
    c_index: i32,
    c_mask: i32,
  ) -> Result<()> {
    let count = count as usize;
    let c_index = c_index as usize;
    self.input.read_ints(c, c_index, count)?;
    let max_iter = (b_shift - 1) / dec;
    let mask = i32x8::splat(b_mask);
    let c_mask_simd = i32x8::splat(c_mask);
    let mut i = 0;
    while i + 8 <= count {
      let values = i32x8::from(&c[c_index + i..c_index + i + 8]);
      for j in 0..=max_iter {
        let shift = b_shift - j * dec;
        if shift > 0 {
          let values: [i32; 8] = ((values >> shift as u32) & mask).into();
          let start = count * j as usize + i;
          b[start..start + 8].copy_from_slice(&values);
        }
      }
      let values: [i32; 8] = (values & c_mask_simd).into();
      c[c_index + i..c_index + i + 8].copy_from_slice(&values);
      i += 8;
    }
    while i < count {
      for j in 0..=max_iter {
        let shift = b_shift - j * dec;
        if shift > 0 {
          b[count * j as usize + i] = ((c[c_index + i] as u64) >> shift) as i32 & b_mask;
        }
      }
      c[c_index + i] &= c_mask;
      i += 1;
    }

    Ok(())
  }
}

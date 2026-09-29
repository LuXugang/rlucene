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
use wide::{i32x4, i32x8};

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
  /// - Read `count` integers into `c[c_index..]`
  /// - For all `i >= 0` such that `b_shift - i * dec > 0`:
  ///   - Apply shift `b_shift - i * dec` to each value in `c`
  ///   - Store the result in `b` at offset `count * i`
  /// - Apply mask `c_mask` to each value in `c` starting at `c_index`
  #[allow(clippy::too_many_arguments)]
  pub fn split_ints_same(
    &mut self,
    count: usize,
    b_and_c: &mut [i32],
    b_shift: i32,
    dec: i32,
    b_mask: i32,
    c_index: usize,
    c_mask: i32,
  ) -> Result<()> {
    self.input.read_ints(b_and_c, c_index, count)?;

    if count == 4
      && b_shift == 7
      && dec == 1
      && b_and_c.len() >= 28
      && c_index >= 28
      && c_index
        .checked_add(4)
        .is_some_and(|end| end <= b_and_c.len())
    {
      let values = i32x4::from(&b_and_c[c_index..c_index + 4]);
      let mask = i32x4::splat(b_mask);
      let decoded: [i32; 4] = ((values >> 7u32) & mask).into();
      b_and_c[0..4].copy_from_slice(&decoded);
      let decoded: [i32; 4] = ((values >> 6u32) & mask).into();
      b_and_c[4..8].copy_from_slice(&decoded);
      let decoded: [i32; 4] = ((values >> 5u32) & mask).into();
      b_and_c[8..12].copy_from_slice(&decoded);
      let decoded: [i32; 4] = ((values >> 4u32) & mask).into();
      b_and_c[12..16].copy_from_slice(&decoded);
      let decoded: [i32; 4] = ((values >> 3u32) & mask).into();
      b_and_c[16..20].copy_from_slice(&decoded);
      let decoded: [i32; 4] = ((values >> 2u32) & mask).into();
      b_and_c[20..24].copy_from_slice(&decoded);
      let decoded: [i32; 4] = ((values >> 1u32) & mask).into();
      b_and_c[24..28].copy_from_slice(&decoded);
      let decoded: [i32; 4] = (values & i32x4::splat(c_mask)).into();
      b_and_c[c_index..c_index + 4].copy_from_slice(&decoded);
      return Ok(());
    }
    // The eight-input, three-plane layout also has fixed shifts and offsets.
    if count == 8
      && b_shift == 6
      && dec == 2
      && b_and_c.len() >= 24
      && c_index >= 24
      && c_index
        .checked_add(8)
        .is_some_and(|end| end <= b_and_c.len())
    {
      let values = i32x8::from(&b_and_c[c_index..c_index + 8]);
      let mask = i32x8::splat(b_mask);
      let decoded: [i32; 8] = ((values >> 6u32) & mask).into();
      b_and_c[0..8].copy_from_slice(&decoded);
      let decoded: [i32; 8] = ((values >> 4u32) & mask).into();
      b_and_c[8..16].copy_from_slice(&decoded);
      let decoded: [i32; 8] = ((values >> 2u32) & mask).into();
      b_and_c[16..24].copy_from_slice(&decoded);
      let decoded: [i32; 8] = (values & i32x8::splat(c_mask)).into();
      b_and_c[c_index..c_index + 8].copy_from_slice(&decoded);
      return Ok(());
    }
    let max_iter = (b_shift - 1) / dec;
    // A single output plane permits one contiguous pass over input and output.
    if max_iter == 0
      && (1..32).contains(&b_shift)
      && dec > 0
      && (max_iter as usize + 1)
        .checked_mul(count)
        .is_some_and(|end| end <= b_and_c.len() && end <= c_index)
      && c_index
        .checked_add(count)
        .is_some_and(|end| end <= b_and_c.len())
    {
      let (b, c) = b_and_c.split_at_mut(c_index);
      let c = &mut c[..count];
      let shift = b_shift as u32;
      for (target, value) in b[..count].iter_mut().zip(c.iter_mut()) {
        let raw = *value;
        *target = (raw >> shift) & b_mask;
        *value = raw & c_mask;
      }
      return Ok(());
    }
    // Reuse each input vector across two or three output planes without an
    // inner loop. The original path handles other layouts and invalid ranges.
    if count.is_multiple_of(4)
      && (1..32).contains(&b_shift)
      && dec > 0
      && (max_iter as usize + 1)
        .checked_mul(count)
        .is_some_and(|end| end <= b_and_c.len() && end <= c_index)
      && c_index
        .checked_add(count)
        .is_some_and(|end| end <= b_and_c.len())
    {
      let mut i = 0;
      match max_iter {
        1 => {
          while i + 8 <= count {
            let mask = i32x8::splat(b_mask);
            let cmask = i32x8::splat(c_mask);
            let values = i32x8::from(&b_and_c[c_index + i..c_index + i + 8]);
            let decoded: [i32; 8] = ((values >> b_shift as u32) & mask).into();
            let start = i;
            b_and_c[start..start + 8].copy_from_slice(&decoded);
            let decoded: [i32; 8] = ((values >> (b_shift - dec) as u32) & mask).into();
            let start = count + i;
            b_and_c[start..start + 8].copy_from_slice(&decoded);
            let decoded: [i32; 8] = (values & cmask).into();
            b_and_c[c_index + i..c_index + i + 8].copy_from_slice(&decoded);
            i += 8;
          }
          if i < count {
            let mask = i32x4::splat(b_mask);
            let cmask = i32x4::splat(c_mask);
            let values = i32x4::from(&b_and_c[c_index + i..c_index + i + 4]);
            let decoded: [i32; 4] = ((values >> b_shift as u32) & mask).into();
            let start = i;
            b_and_c[start..start + 4].copy_from_slice(&decoded);
            let decoded: [i32; 4] = ((values >> (b_shift - dec) as u32) & mask).into();
            let start = count + i;
            b_and_c[start..start + 4].copy_from_slice(&decoded);
            let decoded: [i32; 4] = (values & cmask).into();
            b_and_c[c_index + i..c_index + i + 4].copy_from_slice(&decoded);
          }
          return Ok(());
        },
        2 => {
          while i + 8 <= count {
            let mask = i32x8::splat(b_mask);
            let cmask = i32x8::splat(c_mask);
            let values = i32x8::from(&b_and_c[c_index + i..c_index + i + 8]);
            let decoded: [i32; 8] = ((values >> b_shift as u32) & mask).into();
            let start = i;
            b_and_c[start..start + 8].copy_from_slice(&decoded);
            let decoded: [i32; 8] = ((values >> (b_shift - dec) as u32) & mask).into();
            let start = count + i;
            b_and_c[start..start + 8].copy_from_slice(&decoded);
            let decoded: [i32; 8] = ((values >> (b_shift - 2 * dec) as u32) & mask).into();
            let start = count * 2 + i;
            b_and_c[start..start + 8].copy_from_slice(&decoded);
            let decoded: [i32; 8] = (values & cmask).into();
            b_and_c[c_index + i..c_index + i + 8].copy_from_slice(&decoded);
            i += 8;
          }
          if i < count {
            let mask = i32x4::splat(b_mask);
            let cmask = i32x4::splat(c_mask);
            let values = i32x4::from(&b_and_c[c_index + i..c_index + i + 4]);
            let decoded: [i32; 4] = ((values >> b_shift as u32) & mask).into();
            let start = i;
            b_and_c[start..start + 4].copy_from_slice(&decoded);
            let decoded: [i32; 4] = ((values >> (b_shift - dec) as u32) & mask).into();
            let start = count + i;
            b_and_c[start..start + 4].copy_from_slice(&decoded);
            let decoded: [i32; 4] = ((values >> (b_shift - 2 * dec) as u32) & mask).into();
            let start = count * 2 + i;
            b_and_c[start..start + 4].copy_from_slice(&decoded);
            let decoded: [i32; 4] = (values & cmask).into();
            b_and_c[c_index + i..c_index + i + 4].copy_from_slice(&decoded);
          }
          return Ok(());
        },
        _ => {},
      }
    }
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
    if count - i >= 4
      && (1..32).contains(&b_shift)
      && dec > 0
      && (max_iter as usize + 1)
        .checked_mul(count)
        .is_some_and(|end| end <= b_and_c.len() && end <= c_index)
      && c_index
        .checked_add(count)
        .is_some_and(|end| end <= b_and_c.len())
    {
      let mask = i32x4::splat(b_mask);
      let c_mask_simd = i32x4::splat(c_mask);
      let values = i32x4::from(&b_and_c[c_index + i..c_index + i + 4]);
      for j in 0..=max_iter {
        let shift = b_shift - j * dec;
        let decoded: [i32; 4] = ((values >> shift as u32) & mask).into();
        let start = count * j as usize + i;
        b_and_c[start..start + 4].copy_from_slice(&decoded);
      }
      let decoded: [i32; 4] = (values & c_mask_simd).into();
      b_and_c[c_index + i..c_index + i + 4].copy_from_slice(&decoded);
      i += 4;
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
    count: usize,
    b: &mut [i32],
    b_shift: i32,
    dec: i32,
    b_mask: i32,
    c: &mut [i32],
    c_index: usize,
    c_mask: i32,
  ) -> Result<()> {
    self.input.read_ints(c, c_index, count)?;
    if count == 4
      && b_shift == 7
      && dec == 1
      && b.len() >= 28
      && c_index.checked_add(4).is_some_and(|end| end <= c.len())
    {
      let values = i32x4::from(&c[c_index..c_index + 4]);
      let mask = i32x4::splat(b_mask);
      let decoded: [i32; 4] = ((values >> 7u32) & mask).into();
      b[0..4].copy_from_slice(&decoded);
      let decoded: [i32; 4] = ((values >> 6u32) & mask).into();
      b[4..8].copy_from_slice(&decoded);
      let decoded: [i32; 4] = ((values >> 5u32) & mask).into();
      b[8..12].copy_from_slice(&decoded);
      let decoded: [i32; 4] = ((values >> 4u32) & mask).into();
      b[12..16].copy_from_slice(&decoded);
      let decoded: [i32; 4] = ((values >> 3u32) & mask).into();
      b[16..20].copy_from_slice(&decoded);
      let decoded: [i32; 4] = ((values >> 2u32) & mask).into();
      b[20..24].copy_from_slice(&decoded);
      let decoded: [i32; 4] = ((values >> 1u32) & mask).into();
      b[24..28].copy_from_slice(&decoded);
      let decoded: [i32; 4] = (values & i32x4::splat(c_mask)).into();
      c[c_index..c_index + 4].copy_from_slice(&decoded);
      return Ok(());
    }
    // The eight-input, three-plane layout also has fixed shifts and offsets.
    if count == 8
      && b_shift == 6
      && dec == 2
      && b.len() >= 24
      && c_index.checked_add(8).is_some_and(|end| end <= c.len())
    {
      let values = i32x8::from(&c[c_index..c_index + 8]);
      let mask = i32x8::splat(b_mask);
      let decoded: [i32; 8] = ((values >> 6u32) & mask).into();
      b[0..8].copy_from_slice(&decoded);
      let decoded: [i32; 8] = ((values >> 4u32) & mask).into();
      b[8..16].copy_from_slice(&decoded);
      let decoded: [i32; 8] = ((values >> 2u32) & mask).into();
      b[16..24].copy_from_slice(&decoded);
      let decoded: [i32; 8] = (values & i32x8::splat(c_mask)).into();
      c[c_index..c_index + 8].copy_from_slice(&decoded);
      return Ok(());
    }
    let max_iter = (b_shift - 1) / dec;
    // A single output plane permits one contiguous pass over input and output.
    if max_iter == 0
      && (1..32).contains(&b_shift)
      && dec > 0
      && (max_iter as usize + 1)
        .checked_mul(count)
        .is_some_and(|end| end <= b.len())
      && c_index.checked_add(count).is_some_and(|end| end <= c.len())
    {
      let c = &mut c[c_index..c_index + count];
      let shift = b_shift as u32;
      for (target, value) in b[..count].iter_mut().zip(c.iter_mut()) {
        let raw = *value;
        *target = (raw >> shift) & b_mask;
        *value = raw & c_mask;
      }
      return Ok(());
    }
    // Reuse each input vector across two or three output planes without an
    // inner loop. The original path handles other layouts and invalid ranges.
    if count.is_multiple_of(4)
      && (1..32).contains(&b_shift)
      && dec > 0
      && (max_iter as usize + 1)
        .checked_mul(count)
        .is_some_and(|end| end <= b.len())
      && c_index.checked_add(count).is_some_and(|end| end <= c.len())
    {
      let mut i = 0;
      match max_iter {
        1 => {
          while i + 8 <= count {
            let mask = i32x8::splat(b_mask);
            let cmask = i32x8::splat(c_mask);
            let values = i32x8::from(&c[c_index + i..c_index + i + 8]);
            let decoded: [i32; 8] = ((values >> b_shift as u32) & mask).into();
            let start = i;
            b[start..start + 8].copy_from_slice(&decoded);
            let decoded: [i32; 8] = ((values >> (b_shift - dec) as u32) & mask).into();
            let start = count + i;
            b[start..start + 8].copy_from_slice(&decoded);
            let decoded: [i32; 8] = (values & cmask).into();
            c[c_index + i..c_index + i + 8].copy_from_slice(&decoded);
            i += 8;
          }
          if i < count {
            let mask = i32x4::splat(b_mask);
            let cmask = i32x4::splat(c_mask);
            let values = i32x4::from(&c[c_index + i..c_index + i + 4]);
            let decoded: [i32; 4] = ((values >> b_shift as u32) & mask).into();
            let start = i;
            b[start..start + 4].copy_from_slice(&decoded);
            let decoded: [i32; 4] = ((values >> (b_shift - dec) as u32) & mask).into();
            let start = count + i;
            b[start..start + 4].copy_from_slice(&decoded);
            let decoded: [i32; 4] = (values & cmask).into();
            c[c_index + i..c_index + i + 4].copy_from_slice(&decoded);
          }
          return Ok(());
        },
        2 => {
          while i + 8 <= count {
            let mask = i32x8::splat(b_mask);
            let cmask = i32x8::splat(c_mask);
            let values = i32x8::from(&c[c_index + i..c_index + i + 8]);
            let decoded: [i32; 8] = ((values >> b_shift as u32) & mask).into();
            let start = i;
            b[start..start + 8].copy_from_slice(&decoded);
            let decoded: [i32; 8] = ((values >> (b_shift - dec) as u32) & mask).into();
            let start = count + i;
            b[start..start + 8].copy_from_slice(&decoded);
            let decoded: [i32; 8] = ((values >> (b_shift - 2 * dec) as u32) & mask).into();
            let start = count * 2 + i;
            b[start..start + 8].copy_from_slice(&decoded);
            let decoded: [i32; 8] = (values & cmask).into();
            c[c_index + i..c_index + i + 8].copy_from_slice(&decoded);
            i += 8;
          }
          if i < count {
            let mask = i32x4::splat(b_mask);
            let cmask = i32x4::splat(c_mask);
            let values = i32x4::from(&c[c_index + i..c_index + i + 4]);
            let decoded: [i32; 4] = ((values >> b_shift as u32) & mask).into();
            let start = i;
            b[start..start + 4].copy_from_slice(&decoded);
            let decoded: [i32; 4] = ((values >> (b_shift - dec) as u32) & mask).into();
            let start = count + i;
            b[start..start + 4].copy_from_slice(&decoded);
            let decoded: [i32; 4] = ((values >> (b_shift - 2 * dec) as u32) & mask).into();
            let start = count * 2 + i;
            b[start..start + 4].copy_from_slice(&decoded);
            let decoded: [i32; 4] = (values & cmask).into();
            c[c_index + i..c_index + i + 4].copy_from_slice(&decoded);
          }
          return Ok(());
        },
        _ => {},
      }
    }
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
    if count - i >= 4
      && (1..32).contains(&b_shift)
      && dec > 0
      && (max_iter as usize + 1)
        .checked_mul(count)
        .is_some_and(|end| end <= b.len())
      && c_index.checked_add(count).is_some_and(|end| end <= c.len())
    {
      let mask = i32x4::splat(b_mask);
      let c_mask_simd = i32x4::splat(c_mask);
      let values = i32x4::from(&c[c_index + i..c_index + i + 4]);
      for j in 0..=max_iter {
        let shift = b_shift - j * dec;
        let decoded: [i32; 4] = ((values >> shift as u32) & mask).into();
        let start = count * j as usize + i;
        b[start..start + 4].copy_from_slice(&decoded);
      }
      let decoded: [i32; 4] = (values & c_mask_simd).into();
      c[c_index + i..c_index + i + 4].copy_from_slice(&decoded);
      i += 4;
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

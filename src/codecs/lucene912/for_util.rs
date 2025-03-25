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

use crate::internal::vectorization::posting_decoding_util::PostingDecodingUtil;
use crate::store::{DataOutput, IndexInput};
use crate::util::error::lucene_error::Result;
/// Inspired by https://fulmicoton.com/posts/bitpacking/
///
/// Encodes multiple integers into a `long` to achieve SIMD-like speedups.
///
/// - If `bits_per_value <= 8`, then 8 integers are packed into each `long`.
/// - If `bits_per_value <= 16`, then 4 integers per `long`.
/// - Otherwise, 2 integers per `long`.
pub struct ForUtil {
    tmp: Vec<i64>,
}
impl ForUtil {
    pub(crate) fn new() -> Self {
        Self {
            tmp: vec![0i64; Self::BLOCK_SIZE],
        }
    }
    pub const BLOCK_SIZE: usize = 128;
    pub const BLOCK_SIZE_LOG2: usize = 7;

    const fn expand_mask32(mask32: i64) -> i64 {
        mask32 | (mask32 << 32)
    }

    const fn expand_mask16(mask16: i64) -> i64 {
        Self::expand_mask32(mask16 | (mask16 << 16))
    }

    const fn expand_mask8(mask8: i64) -> i64 {
        Self::expand_mask16(mask8 | (mask8 << 8))
    }

    const fn mask32(bits_per_value: i32) -> i64 {
        Self::expand_mask32((1i64 << bits_per_value) - 1)
    }

    const fn mask16(bits_per_value: i32) -> i64 {
        Self::expand_mask16((1i64 << bits_per_value) - 1)
    }

    const fn mask8(bits_per_value: i32) -> i64 {
        Self::expand_mask8((1i64 << bits_per_value) - 1)
    }
    fn expand8(arr: &mut [i64]) {
        for i in 0..16 {
            let l = arr[i] as usize;
            arr[i] = ((l >> 56) & 0xFF) as i64;
            arr[16 + i] = ((l >> 48) & 0xFF) as i64;
            arr[32 + i] = ((l >> 40) & 0xFF) as i64;
            arr[48 + i] = ((l >> 32) & 0xFF) as i64;
            arr[64 + i] = ((l >> 24) & 0xFF) as i64;
            arr[80 + i] = ((l >> 16) & 0xFF) as i64;
            arr[96 + i] = ((l >> 8) & 0xFF) as i64;
            arr[112 + i] = (l & 0xFF) as i64;
        }
    }
    fn collapse8(arr: &mut [i64]) {
        for i in 0..16 {
            arr[i] = (arr[i] << 56)
                | (arr[16 + i] << 48)
                | (arr[32 + i] << 40)
                | (arr[48 + i] << 32)
                | (arr[64 + i] << 24)
                | (arr[80 + i] << 16)
                | (arr[96 + i] << 8)
                | arr[112 + i];
        }
    }

    fn expand16(arr: &mut [i64]) {
        for i in 0..32 {
            let l = arr[i] as usize;
            arr[i] = ((l >> 48) & 0xFFFF) as i64;
            arr[32 + i] = ((l >> 32) & 0xFFFF) as i64;
            arr[64 + i] = ((l >> 16) & 0xFFFF) as i64;
            arr[96 + i] = (l & 0xFFFF) as i64;
        }
    }

    fn collapse16(arr: &mut [i64]) {
        for i in 0..32 {
            arr[i] = (arr[i] << 48) | (arr[32 + i] << 32) | (arr[64 + i] << 16) | arr[96 + i];
        }
    }

    fn expand32(arr: &mut [i64]) {
        for i in 0..64 {
            let l = arr[i] as u64;
            arr[i] = (l >> 32) as i64;
            arr[64 + i] = (l & 0xFFFFFFFF) as i64;
        }
    }

    fn collapse32(arr: &mut [i64]) {
        for i in 0..64 {
            arr[i] = (arr[i] << 32) | arr[64 + i];
        }
    }
    /// Encode 128 integers from `longs` into out`.
    pub(crate) fn encode(
        &mut self,
        longs: &mut [i64],
        bits_per_value: i32,
        out: &mut impl DataOutput,
    ) -> Result<()> {
        let next_primitive = if bits_per_value <= 8 {
            Self::collapse8(longs);
            8
        } else if bits_per_value <= 16 {
            Self::collapse16(longs);
            16
        } else {
            Self::collapse32(longs);
            32
        };

        Self::encode_with_tmp(longs, bits_per_value, next_primitive, out, &mut self.tmp)
    }

    fn encode_with_tmp(
        longs: &[i64],
        bits_per_value: i32,
        primitive_size: i32,
        out: &mut impl DataOutput,
        tmp: &mut [i64],
    ) -> Result<()> {
        let num_longs = Self::BLOCK_SIZE * (primitive_size as usize) / i64::BITS as usize;
        let num_longs_per_shift = (bits_per_value * 2) as usize;

        let mut idx = 0;
        let mut shift = primitive_size - bits_per_value;
        for (t, l) in tmp.iter_mut().take(num_longs_per_shift).zip(&longs[idx..]) {
            *t = *l << shift;
        }
        idx += num_longs_per_shift;

        shift -= bits_per_value;
        while shift >= 0 {
            for (t, l) in tmp.iter_mut().take(num_longs_per_shift).zip(&longs[idx..]) {
                *t |= *l << shift;
            }
            idx += num_longs_per_shift;
            shift -= bits_per_value;
        }

        let remaining_bits_per_long = shift + bits_per_value;
        let mask_remaining_bits_per_long = match primitive_size {
            8 => Self::MASKS8[remaining_bits_per_long as usize],
            16 => Self::MASKS16[remaining_bits_per_long as usize],
            _ => Self::MASKS32[remaining_bits_per_long as usize],
        };

        let mut tmp_idx = 0;
        let mut remaining_bits_per_value = bits_per_value;
        while idx < num_longs {
            if remaining_bits_per_value >= remaining_bits_per_long {
                remaining_bits_per_value -= remaining_bits_per_long;
                tmp[tmp_idx] |= (longs[idx] as u64 >> remaining_bits_per_value) as i64
                    & mask_remaining_bits_per_long;
                if remaining_bits_per_value == 0 {
                    idx += 1;
                    remaining_bits_per_value = bits_per_value;
                }
                tmp_idx += 1;
            } else {
                let remaining_bits_per_value_index = remaining_bits_per_value as usize;
                let remaining_bits_per_long_index = remaining_bits_per_long as usize;
                let (mask1, mask2) = match primitive_size {
                    8 => (
                        Self::MASKS8[remaining_bits_per_value_index],
                        Self::MASKS8
                            [remaining_bits_per_long_index - remaining_bits_per_value_index],
                    ),
                    16 => (
                        Self::MASKS16[remaining_bits_per_value_index],
                        Self::MASKS16
                            [remaining_bits_per_long_index - remaining_bits_per_value_index],
                    ),
                    _ => (
                        Self::MASKS32[remaining_bits_per_value_index],
                        Self::MASKS32
                            [remaining_bits_per_long_index - remaining_bits_per_value_index],
                    ),
                };

                tmp[tmp_idx] |=
                    (longs[idx] & mask1) << (remaining_bits_per_long - remaining_bits_per_value);
                idx += 1;
                remaining_bits_per_value += bits_per_value - remaining_bits_per_long;
                tmp[tmp_idx] |= (longs[idx] as u64 >> remaining_bits_per_value) as i64 & mask2;
                tmp_idx += 1;
            }
        }
        for &val in tmp.iter().take(num_longs_per_shift) {
            out.write_long(val)?;
        }

        Ok(())
    }
    /// Number of bytes required to encode 128 integers of `bitsPerValue` bits per value.
    pub(crate) fn num_bytes(bits_per_value: i32) -> i32 {
        bits_per_value << (Self::BLOCK_SIZE_LOG2 - 3)
    }

    fn decode_slow<I: IndexInput>(
        bits_per_value: i32,
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        let num_longs = bits_per_value << 1;
        let mask = Self::MASKS32[bits_per_value as usize];
        pdu.split_longs_diff(num_longs, longs, 32 - bits_per_value, 32, mask, tmp, 0, -1)?;

        let remaining_bits_per_long = (32 - bits_per_value) as usize;
        let mask32_remaining_bits_per_long = Self::MASKS32[remaining_bits_per_long];

        let mut tmp_idx = 0;
        let mut remaining_bits = remaining_bits_per_long;
        #[allow(clippy::needless_range_loop)]
        for longs_idx in num_longs as usize..(Self::BLOCK_SIZE / 2) {
            let mut b = bits_per_value as usize - remaining_bits;
            let mut l = (tmp[tmp_idx] & Self::MASKS32[remaining_bits]) << b;
            tmp_idx += 1;

            while b >= remaining_bits_per_long {
                b -= remaining_bits_per_long;
                l |= (tmp[tmp_idx] & mask32_remaining_bits_per_long) << b;
                tmp_idx += 1;
            }

            if b > 0 {
                l |= (tmp[tmp_idx] >> (remaining_bits_per_long - b)) & Self::MASKS32[b];
                remaining_bits = remaining_bits_per_long - b;
            } else {
                remaining_bits = remaining_bits_per_long;
            }

            longs[longs_idx] = l;
        }

        Ok(())
    }

    const MASKS8: [i64; 8] = {
        let mut masks = [0i64; 8];
        let mut i = 0;
        while i < 8 {
            masks[i] = Self::mask8(i as i32);
            i += 1;
        }
        masks
    };

    const MASKS16: [i64; 16] = {
        let mut masks = [0i64; 16];
        let mut i = 0;
        while i < 16 {
            masks[i] = Self::mask16(i as i32);
            i += 1;
        }
        masks
    };

    const MASKS32: [i64; 32] = {
        let mut masks = [0i64; 32];
        let mut i = 0;
        while i < 32 {
            masks[i] = Self::mask32(i as i32);
            i += 1;
        }
        masks
    };

    pub const MASK8_1: i64 = Self::MASKS8[1];
    pub const MASK8_2: i64 = Self::MASKS8[2];
    pub const MASK8_3: i64 = Self::MASKS8[3];
    pub const MASK8_4: i64 = Self::MASKS8[4];
    pub const MASK8_5: i64 = Self::MASKS8[5];
    pub const MASK8_6: i64 = Self::MASKS8[6];
    pub const MASK8_7: i64 = Self::MASKS8[7];

    pub const MASK16_1: i64 = Self::MASKS16[1];
    pub const MASK16_2: i64 = Self::MASKS16[2];
    pub const MASK16_3: i64 = Self::MASKS16[3];
    pub const MASK16_4: i64 = Self::MASKS16[4];
    pub const MASK16_5: i64 = Self::MASKS16[5];
    pub const MASK16_6: i64 = Self::MASKS16[6];
    pub const MASK16_7: i64 = Self::MASKS16[7];
    pub const MASK16_8: i64 = Self::MASKS16[8];
    pub const MASK16_9: i64 = Self::MASKS16[9];
    pub const MASK16_10: i64 = Self::MASKS16[10];
    pub const MASK16_11: i64 = Self::MASKS16[11];
    pub const MASK16_12: i64 = Self::MASKS16[12];
    pub const MASK16_13: i64 = Self::MASKS16[13];
    pub const MASK16_14: i64 = Self::MASKS16[14];
    pub const MASK16_15: i64 = Self::MASKS16[15];

    pub const MASK32_1: i64 = Self::MASKS32[1];
    pub const MASK32_2: i64 = Self::MASKS32[2];
    pub const MASK32_3: i64 = Self::MASKS32[3];
    pub const MASK32_4: i64 = Self::MASKS32[4];
    pub const MASK32_5: i64 = Self::MASKS32[5];
    pub const MASK32_6: i64 = Self::MASKS32[6];
    pub const MASK32_7: i64 = Self::MASKS32[7];
    pub const MASK32_8: i64 = Self::MASKS32[8];
    pub const MASK32_9: i64 = Self::MASKS32[9];
    pub const MASK32_10: i64 = Self::MASKS32[10];
    pub const MASK32_11: i64 = Self::MASKS32[11];
    pub const MASK32_12: i64 = Self::MASKS32[12];
    pub const MASK32_13: i64 = Self::MASKS32[13];
    pub const MASK32_14: i64 = Self::MASKS32[14];
    pub const MASK32_15: i64 = Self::MASKS32[15];
    pub const MASK32_16: i64 = Self::MASKS32[16];
    pub const MASK32_17: i64 = Self::MASKS32[17];
    pub const MASK32_18: i64 = Self::MASKS32[18];
    pub const MASK32_19: i64 = Self::MASKS32[19];
    pub const MASK32_20: i64 = Self::MASKS32[20];
    pub const MASK32_21: i64 = Self::MASKS32[21];
    pub const MASK32_22: i64 = Self::MASKS32[22];
    pub const MASK32_23: i64 = Self::MASKS32[23];
    pub const MASK32_24: i64 = Self::MASKS32[24];
    pub(crate) fn decode<I: IndexInput>(
        &mut self,
        bits_per_value: i32,
        pdu: &mut PostingDecodingUtil<I>,
        longs: &mut [i64],
    ) -> Result<()> {
        match bits_per_value {
            1 => {
                Self::decode1(pdu, longs)?;
                Self::expand8(longs);
            }
            2 => {
                Self::decode2(pdu, longs)?;
                Self::expand8(longs);
            }
            3 => {
                Self::decode3(pdu, &mut self.tmp, longs)?;
                Self::expand8(longs);
            }
            4 => {
                Self::decode4(pdu, longs)?;
                Self::expand8(longs);
            }
            5 => {
                Self::decode5(pdu, &mut self.tmp, longs)?;
                Self::expand8(longs);
            }
            6 => {
                Self::decode6(pdu, &mut self.tmp, longs)?;
                Self::expand8(longs);
            }
            7 => {
                Self::decode7(pdu, &mut self.tmp, longs)?;
                Self::expand8(longs);
            }
            8 => {
                Self::decode8(pdu, longs)?;
                Self::expand8(longs);
            }
            9 => {
                Self::decode9(pdu, &mut self.tmp, longs)?;
                Self::expand16(longs);
            }
            10 => {
                Self::decode10(pdu, &mut self.tmp, longs)?;
                Self::expand16(longs);
            }
            11 => {
                Self::decode11(pdu, &mut self.tmp, longs)?;
                Self::expand16(longs);
            }
            12 => {
                Self::decode12(pdu, &mut self.tmp, longs)?;
                Self::expand16(longs);
            }
            13 => {
                Self::decode13(pdu, &mut self.tmp, longs)?;
                Self::expand16(longs);
            }
            14 => {
                Self::decode14(pdu, &mut self.tmp, longs)?;
                Self::expand16(longs);
            }
            15 => {
                Self::decode15(pdu, &mut self.tmp, longs)?;
                Self::expand16(longs);
            }
            16 => {
                Self::decode16(pdu, longs)?;
                Self::expand16(longs);
            }
            17 => {
                Self::decode17(pdu, &mut self.tmp, longs)?;
                Self::expand32(longs);
            }
            18 => {
                Self::decode18(pdu, &mut self.tmp, longs)?;
                Self::expand32(longs);
            }
            19 => {
                Self::decode19(pdu, &mut self.tmp, longs)?;
                Self::expand32(longs);
            }
            20 => {
                Self::decode20(pdu, &mut self.tmp, longs)?;
                Self::expand32(longs);
            }
            21 => {
                Self::decode21(pdu, &mut self.tmp, longs)?;
                Self::expand32(longs);
            }
            22 => {
                Self::decode22(pdu, &mut self.tmp, longs)?;
                Self::expand32(longs);
            }
            23 => {
                Self::decode23(pdu, &mut self.tmp, longs)?;
                Self::expand32(longs);
            }
            24 => {
                Self::decode24(pdu, &mut self.tmp, longs)?;
                Self::expand32(longs);
            }
            _ => {
                Self::decode_slow(bits_per_value, pdu, &mut self.tmp, longs)?;
                Self::expand32(longs);
            }
        }
        Ok(())
    }

    fn decode1<I: IndexInput>(pdu: &mut PostingDecodingUtil<I>, longs: &mut [i64]) -> Result<()> {
        pdu.split_longs_same(2, longs, 7, 1, Self::MASK8_1, 14, Self::MASK8_1)
    }
    fn decode2<I: IndexInput>(pdu: &mut PostingDecodingUtil<I>, longs: &mut [i64]) -> Result<()> {
        pdu.split_longs_same(4, longs, 6, 2, Self::MASK8_2, 12, Self::MASK8_2)
    }

    fn decode3<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(6, longs, 5, 3, Self::MASK8_3, tmp, 0, Self::MASK8_2)?;

        let mut iter = 0;
        let mut tmp_idx = 0;
        let mut longs_idx = 12;

        while iter < 2 {
            let mut l0 = tmp[tmp_idx] << 1;
            l0 |= ((tmp[tmp_idx + 1] as u64) >> 1) as i64 & Self::MASK8_1;
            longs[longs_idx] = l0;
            let mut l1 = (tmp[tmp_idx + 1] & Self::MASK8_1) << 2;
            l1 |= tmp[tmp_idx + 2];
            longs[longs_idx + 1] = l1;
            iter += 1;
            tmp_idx += 3;
            longs_idx += 2;
        }
        Ok(())
    }
    fn decode4<I: IndexInput>(pdu: &mut PostingDecodingUtil<I>, longs: &mut [i64]) -> Result<()> {
        pdu.split_longs_same(8, longs, 4, 4, Self::MASK8_4, 8, Self::MASK8_4)
    }
    fn decode5<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(10, longs, 3, 5, Self::MASK8_5, tmp, 0, Self::MASK8_3)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 10;
        for _ in 0..2 {
            let mut l0 = tmp[tmp_idx] << 2;
            l0 |= ((tmp[tmp_idx + 1] as u64) >> 1) as i64 & Self::MASK8_2;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & Self::MASK8_1) << 4;
            l1 |= tmp[tmp_idx + 2] << 1;
            l1 |= ((tmp[tmp_idx + 3] as u64) >> 2) as i64 & Self::MASK8_1;
            longs[longs_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 3] & Self::MASK8_2) << 3;
            l2 |= tmp[tmp_idx + 4];
            longs[longs_idx + 2] = l2;

            tmp_idx += 5;
            longs_idx += 3;
        }
        Ok(())
    }
    fn decode6<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(12, longs, 2, 6, Self::MASK8_6, tmp, 0, Self::MASK8_2)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 12;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 4;
            l0 |= tmp[tmp_idx + 1] << 2;
            l0 |= tmp[tmp_idx + 2];
            longs[longs_idx] = l0;

            tmp_idx += 3;
            longs_idx += 1;
        }
        Ok(())
    }

    fn decode7<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(14, longs, 1, 7, Self::MASK8_7, tmp, 0, Self::MASK8_1)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 14;
        for _ in 0..2 {
            let mut l0 = tmp[tmp_idx] << 6;
            l0 |= tmp[tmp_idx + 1] << 5;
            l0 |= tmp[tmp_idx + 2] << 4;
            l0 |= tmp[tmp_idx + 3] << 3;
            l0 |= tmp[tmp_idx + 4] << 2;
            l0 |= tmp[tmp_idx + 5] << 1;
            l0 |= tmp[tmp_idx + 6];
            longs[longs_idx] = l0;

            tmp_idx += 7;
            longs_idx += 1;
        }
        Ok(())
    }
    fn decode8<I: IndexInput>(pdu: &mut PostingDecodingUtil<I>, longs: &mut [i64]) -> Result<()> {
        pdu.input.borrow_mut().read_longs(longs, 0, 16)
    }

    fn decode9<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(18, longs, 7, 9, Self::MASK16_9, tmp, 0, Self::MASK16_7)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 18;
        for _ in 0..2 {
            let mut l0 = tmp[tmp_idx] << 2;
            l0 |= ((tmp[tmp_idx + 1] as u64) >> 5) as i64 & Self::MASK16_2;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & Self::MASK16_5) << 4;
            l1 |= ((tmp[tmp_idx + 2] as u64) >> 3) as i64 & Self::MASK16_4;
            longs[longs_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 2] & Self::MASK16_3) << 6;
            l2 |= ((tmp[tmp_idx + 3] as u64) >> 1) as i64 & Self::MASK16_6;
            longs[longs_idx + 2] = l2;

            let mut l3 = (tmp[tmp_idx + 3] & Self::MASK16_1) << 8;
            l3 |= tmp[tmp_idx + 4] << 1;
            l3 |= ((tmp[tmp_idx + 5] as u64) >> 6) as i64 & Self::MASK16_1;
            longs[longs_idx + 3] = l3;

            let mut l4 = (tmp[tmp_idx + 5] & Self::MASK16_6) << 3;
            l4 |= ((tmp[tmp_idx + 6] as u64) >> 4) as i64 & Self::MASK16_3;
            longs[longs_idx + 4] = l4;

            let mut l5 = (tmp[tmp_idx + 6] & Self::MASK16_4) << 5;
            l5 |= ((tmp[tmp_idx + 7] as u64) >> 2) as i64 & Self::MASK16_5;
            longs[longs_idx + 5] = l5;

            let mut l6 = (tmp[tmp_idx + 7] & Self::MASK16_2) << 7;
            l6 |= tmp[tmp_idx + 8];
            longs[longs_idx + 6] = l6;

            tmp_idx += 9;
            longs_idx += 7;
        }
        Ok(())
    }

    fn decode10<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(20, longs, 6, 10, Self::MASK16_10, tmp, 0, Self::MASK16_6)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 20;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 4;
            l0 |= ((tmp[tmp_idx + 1] as u64) >> 2) as i64 & Self::MASK16_4;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & Self::MASK16_2) << 8;
            l1 |= tmp[tmp_idx + 2] << 2;
            l1 |= ((tmp[tmp_idx + 3] as u64) >> 4) as i64 & Self::MASK16_2;
            longs[longs_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 3] & Self::MASK16_4) << 6;
            l2 |= tmp[tmp_idx + 4];
            longs[longs_idx + 2] = l2;

            tmp_idx += 5;
            longs_idx += 3;
        }
        Ok(())
    }

    fn decode11<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(22, longs, 5, 11, Self::MASK16_11, tmp, 0, Self::MASK16_5)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 22;
        for _ in 0..2 {
            let mut l0 = tmp[tmp_idx] << 6;
            l0 |= tmp[tmp_idx + 1] << 1;
            l0 |= ((tmp[tmp_idx + 2] as u64) >> 4) as i64 & Self::MASK16_1;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 2] & Self::MASK16_4) << 7;
            l1 |= tmp[tmp_idx + 3] << 2;
            l1 |= ((tmp[tmp_idx + 4] as u64) >> 3) as i64 & Self::MASK16_2;
            longs[longs_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 4] & Self::MASK16_3) << 8;
            l2 |= tmp[tmp_idx + 5] << 3;
            l2 |= ((tmp[tmp_idx + 6] as u64) >> 2) as i64 & Self::MASK16_3;
            longs[longs_idx + 2] = l2;

            let mut l3 = (tmp[tmp_idx + 6] & Self::MASK16_2) << 9;
            l3 |= tmp[tmp_idx + 7] << 4;
            l3 |= ((tmp[tmp_idx + 8] as u64) >> 1) as i64 & Self::MASK16_4;
            longs[longs_idx + 3] = l3;

            let mut l4 = (tmp[tmp_idx + 8] & Self::MASK16_1) << 10;
            l4 |= tmp[tmp_idx + 9] << 5;
            l4 |= tmp[tmp_idx + 10];
            longs[longs_idx + 4] = l4;

            tmp_idx += 11;
            longs_idx += 5;
        }
        Ok(())
    }
    fn decode12<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(24, longs, 4, 12, Self::MASK16_12, tmp, 0, Self::MASK16_4)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 24;
        for _ in 0..8 {
            let l0 = (tmp[tmp_idx] << 8) | (tmp[tmp_idx + 1] << 4) | tmp[tmp_idx + 2];
            longs[longs_idx] = l0;
            tmp_idx += 3;
            longs_idx += 1;
        }
        Ok(())
    }

    fn decode13<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(26, longs, 3, 13, Self::MASK16_13, tmp, 0, Self::MASK16_3)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 26;
        for _ in 0..2 {
            let mut l0 = tmp[tmp_idx] << 10;
            l0 |= tmp[tmp_idx + 1] << 7;
            l0 |= tmp[tmp_idx + 2] << 4;
            l0 |= tmp[tmp_idx + 3] << 1;
            l0 |= ((tmp[tmp_idx + 4] as u64) >> 2) as i64 & Self::MASK16_1;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 4] & Self::MASK16_2) << 11;
            l1 |= tmp[tmp_idx + 5] << 8;
            l1 |= tmp[tmp_idx + 6] << 5;
            l1 |= tmp[tmp_idx + 7] << 2;
            l1 |= ((tmp[tmp_idx + 8] as u64) >> 1) as i64 & Self::MASK16_2;
            longs[longs_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 8] & Self::MASK16_1) << 12;
            l2 |= tmp[tmp_idx + 9] << 9;
            l2 |= tmp[tmp_idx + 10] << 6;
            l2 |= tmp[tmp_idx + 11] << 3;
            l2 |= tmp[tmp_idx + 12];
            longs[longs_idx + 2] = l2;

            tmp_idx += 13;
            longs_idx += 3;
        }
        Ok(())
    }

    fn decode14<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(28, longs, 2, 14, Self::MASK16_14, tmp, 0, Self::MASK16_2)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 28;
        for _ in 0..4 {
            let l0 = (tmp[tmp_idx] << 12)
                | (tmp[tmp_idx + 1] << 10)
                | (tmp[tmp_idx + 2] << 8)
                | (tmp[tmp_idx + 3] << 6)
                | (tmp[tmp_idx + 4] << 4)
                | (tmp[tmp_idx + 5] << 2)
                | tmp[tmp_idx + 6];
            longs[longs_idx] = l0;
            tmp_idx += 7;
            longs_idx += 1;
        }
        Ok(())
    }

    fn decode15<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(30, longs, 1, 15, Self::MASK16_15, tmp, 0, Self::MASK16_1)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 30;
        for _ in 0..2 {
            let l0 = (tmp[tmp_idx] << 14)
                | (tmp[tmp_idx + 1] << 13)
                | (tmp[tmp_idx + 2] << 12)
                | (tmp[tmp_idx + 3] << 11)
                | (tmp[tmp_idx + 4] << 10)
                | (tmp[tmp_idx + 5] << 9)
                | (tmp[tmp_idx + 6] << 8)
                | (tmp[tmp_idx + 7] << 7)
                | (tmp[tmp_idx + 8] << 6)
                | (tmp[tmp_idx + 9] << 5)
                | (tmp[tmp_idx + 10] << 4)
                | (tmp[tmp_idx + 11] << 3)
                | (tmp[tmp_idx + 12] << 2)
                | (tmp[tmp_idx + 13] << 1)
                | tmp[tmp_idx + 14];
            longs[longs_idx] = l0;
            tmp_idx += 15;
            longs_idx += 1;
        }
        Ok(())
    }

    fn decode16<I: IndexInput>(pdu: &mut PostingDecodingUtil<I>, longs: &mut [i64]) -> Result<()> {
        pdu.input.borrow_mut().read_longs(longs, 0, 32)
    }
    fn decode17<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(34, longs, 15, 17, Self::MASK32_17, tmp, 0, Self::MASK32_15)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 34;
        for _ in 0..2 {
            let mut l0 = tmp[tmp_idx] << 2;
            l0 |= ((tmp[tmp_idx + 1] as u64) >> 13) as i64 & Self::MASK32_2;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & Self::MASK32_13) << 4;
            l1 |= ((tmp[tmp_idx + 2] as u64) >> 11) as i64 & Self::MASK32_4;
            longs[longs_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 2] & Self::MASK32_11) << 6;
            l2 |= ((tmp[tmp_idx + 3] as u64) >> 9) as i64 & Self::MASK32_6;
            longs[longs_idx + 2] = l2;

            let mut l3 = (tmp[tmp_idx + 3] & Self::MASK32_9) << 8;
            l3 |= ((tmp[tmp_idx + 4] as u64) >> 7) as i64 & Self::MASK32_8;
            longs[longs_idx + 3] = l3;

            let mut l4 = (tmp[tmp_idx + 4] & Self::MASK32_7) << 10;
            l4 |= ((tmp[tmp_idx + 5] as u64) >> 5) as i64 & Self::MASK32_10;
            longs[longs_idx + 4] = l4;

            let mut l5 = (tmp[tmp_idx + 5] & Self::MASK32_5) << 12;
            l5 |= ((tmp[tmp_idx + 6] as u64) >> 3) as i64 & Self::MASK32_12;
            longs[longs_idx + 5] = l5;

            let mut l6 = (tmp[tmp_idx + 6] & Self::MASK32_3) << 14;
            l6 |= ((tmp[tmp_idx + 7] as u64) >> 1) as i64 & Self::MASK32_14;
            longs[longs_idx + 6] = l6;

            let mut l7 = (tmp[tmp_idx + 7] & Self::MASK32_1) << 16;
            l7 |= tmp[tmp_idx + 8] << 1;
            l7 |= ((tmp[tmp_idx + 9] as u64) >> 14) as i64 & Self::MASK32_1;
            longs[longs_idx + 7] = l7;

            let mut l8 = (tmp[tmp_idx + 9] & Self::MASK32_14) << 3;
            l8 |= ((tmp[tmp_idx + 10] as u64) >> 12) as i64 & Self::MASK32_3;
            longs[longs_idx + 8] = l8;

            let mut l9 = (tmp[tmp_idx + 10] & Self::MASK32_12) << 5;
            l9 |= ((tmp[tmp_idx + 11] as u64) >> 10) as i64 & Self::MASK32_5;
            longs[longs_idx + 9] = l9;

            let mut l10 = (tmp[tmp_idx + 11] & Self::MASK32_10) << 7;
            l10 |= ((tmp[tmp_idx + 12] as u64) >> 8) as i64 & Self::MASK32_7;
            longs[longs_idx + 10] = l10;

            let mut l11 = (tmp[tmp_idx + 12] & Self::MASK32_8) << 9;
            l11 |= ((tmp[tmp_idx + 13] as u64) >> 6) as i64 & Self::MASK32_9;
            longs[longs_idx + 11] = l11;

            let mut l12 = (tmp[tmp_idx + 13] & Self::MASK32_6) << 11;
            l12 |= ((tmp[tmp_idx + 14] as u64) >> 4) as i64 & Self::MASK32_11;
            longs[longs_idx + 12] = l12;

            let mut l13 = (tmp[tmp_idx + 14] & Self::MASK32_4) << 13;
            l13 |= ((tmp[tmp_idx + 15] as u64) >> 2) as i64 & Self::MASK32_13;
            longs[longs_idx + 13] = l13;

            let mut l14 = (tmp[tmp_idx + 15] & Self::MASK32_2) << 15;
            l14 |= tmp[tmp_idx + 16];
            longs[longs_idx + 14] = l14;

            tmp_idx += 17;
            longs_idx += 15;
        }
        Ok(())
    }

    fn decode18<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(36, longs, 14, 18, Self::MASK32_18, tmp, 0, Self::MASK32_14)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 36;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 4;
            l0 |= ((tmp[tmp_idx + 1] as u64) >> 10) as i64 & Self::MASK32_4;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & Self::MASK32_10) << 8;
            l1 |= ((tmp[tmp_idx + 2] as u64) >> 6) as i64 & Self::MASK32_8;
            longs[longs_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 2] & Self::MASK32_6) << 12;
            l2 |= ((tmp[tmp_idx + 3] as u64) >> 2) as i64 & Self::MASK32_12;
            longs[longs_idx + 2] = l2;

            let mut l3 = (tmp[tmp_idx + 3] & Self::MASK32_2) << 16;
            l3 |= tmp[tmp_idx + 4] << 2;
            l3 |= ((tmp[tmp_idx + 5] as u64) >> 12) as i64 & Self::MASK32_2;
            longs[longs_idx + 3] = l3;

            let mut l4 = (tmp[tmp_idx + 5] & Self::MASK32_12) << 6;
            l4 |= ((tmp[tmp_idx + 6] as u64) >> 8) as i64 & Self::MASK32_6;
            longs[longs_idx + 4] = l4;

            let mut l5 = (tmp[tmp_idx + 6] & Self::MASK32_8) << 10;
            l5 |= ((tmp[tmp_idx + 7] as u64) >> 4) as i64 & Self::MASK32_10;
            longs[longs_idx + 5] = l5;

            let mut l6 = (tmp[tmp_idx + 7] & Self::MASK32_4) << 14;
            l6 |= tmp[tmp_idx + 8];
            longs[longs_idx + 6] = l6;

            tmp_idx += 9;
            longs_idx += 7;
        }
        Ok(())
    }
    fn decode19<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(38, longs, 13, 19, Self::MASK32_19, tmp, 0, Self::MASK32_13)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 38;
        for _ in 0..2 {
            let mut l0 = tmp[tmp_idx] << 6;
            l0 |= ((tmp[tmp_idx + 1] as u64) >> 7) as i64 & Self::MASK32_6;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & Self::MASK32_7) << 12;
            l1 |= ((tmp[tmp_idx + 2] as u64) >> 1) as i64 & Self::MASK32_12;
            longs[longs_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 2] & Self::MASK32_1) << 18;
            l2 |= tmp[tmp_idx + 3] << 5;
            l2 |= ((tmp[tmp_idx + 4] as u64) >> 8) as i64 & Self::MASK32_5;
            longs[longs_idx + 2] = l2;

            let mut l3 = (tmp[tmp_idx + 4] & Self::MASK32_8) << 11;
            l3 |= ((tmp[tmp_idx + 5] as u64) >> 2) as i64 & Self::MASK32_11;
            longs[longs_idx + 3] = l3;

            let mut l4 = (tmp[tmp_idx + 5] & Self::MASK32_2) << 17;
            l4 |= tmp[tmp_idx + 6] << 4;
            l4 |= ((tmp[tmp_idx + 7] as u64) >> 9) as i64 & Self::MASK32_4;
            longs[longs_idx + 4] = l4;

            let mut l5 = (tmp[tmp_idx + 7] & Self::MASK32_9) << 10;
            l5 |= ((tmp[tmp_idx + 8] as u64) >> 3) as i64 & Self::MASK32_10;
            longs[longs_idx + 5] = l5;

            let mut l6 = (tmp[tmp_idx + 8] & Self::MASK32_3) << 16;
            l6 |= tmp[tmp_idx + 9] << 3;
            l6 |= ((tmp[tmp_idx + 10] as u64) >> 10) as i64 & Self::MASK32_3;
            longs[longs_idx + 6] = l6;

            let mut l7 = (tmp[tmp_idx + 10] & Self::MASK32_10) << 9;
            l7 |= ((tmp[tmp_idx + 11] as u64) >> 4) as i64 & Self::MASK32_9;
            longs[longs_idx + 7] = l7;

            let mut l8 = (tmp[tmp_idx + 11] & Self::MASK32_4) << 15;
            l8 |= tmp[tmp_idx + 12] << 2;
            l8 |= ((tmp[tmp_idx + 13] as u64) >> 11) as i64 & Self::MASK32_2;
            longs[longs_idx + 8] = l8;

            let mut l9 = (tmp[tmp_idx + 13] & Self::MASK32_11) << 8;
            l9 |= ((tmp[tmp_idx + 14] as u64) >> 5) as i64 & Self::MASK32_8;
            longs[longs_idx + 9] = l9;

            let mut l10 = (tmp[tmp_idx + 14] & Self::MASK32_5) << 14;
            l10 |= tmp[tmp_idx + 15] << 1;
            l10 |= ((tmp[tmp_idx + 16] as u64) >> 12) as i64 & Self::MASK32_1;
            longs[longs_idx + 10] = l10;

            let mut l11 = (tmp[tmp_idx + 16] & Self::MASK32_12) << 7;
            l11 |= ((tmp[tmp_idx + 17] as u64) >> 6) as i64 & Self::MASK32_7;
            longs[longs_idx + 11] = l11;

            let l12 = (tmp[tmp_idx + 17] & Self::MASK32_6) << 13 | tmp[tmp_idx + 18];
            longs[longs_idx + 12] = l12;

            tmp_idx += 19;
            longs_idx += 13;
        }
        Ok(())
    }

    fn decode20<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(40, longs, 12, 20, Self::MASK32_20, tmp, 0, Self::MASK32_12)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 40;
        for _ in 0..8 {
            let mut l0 = tmp[tmp_idx] << 8;
            l0 |= ((tmp[tmp_idx + 1] as u64) >> 4) as i64 & Self::MASK32_8;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & Self::MASK32_4) << 16;
            l1 |= tmp[tmp_idx + 2] << 4;
            l1 |= ((tmp[tmp_idx + 3] as u64) >> 8) as i64 & Self::MASK32_4;
            longs[longs_idx + 1] = l1;

            let l2 = (tmp[tmp_idx + 3] & Self::MASK32_8) << 12 | tmp[tmp_idx + 4];
            longs[longs_idx + 2] = l2;

            tmp_idx += 5;
            longs_idx += 3;
        }
        Ok(())
    }

    fn decode21<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(42, longs, 11, 21, Self::MASK32_21, tmp, 0, Self::MASK32_11)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 42;
        for _ in 0..2 {
            let mut l0 = tmp[tmp_idx] << 10;
            l0 |= ((tmp[tmp_idx + 1] as u64) >> 1) as i64 & Self::MASK32_10;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & Self::MASK32_1) << 20;
            l1 |= tmp[tmp_idx + 2] << 9;
            l1 |= ((tmp[tmp_idx + 3] as u64) >> 2) as i64 & Self::MASK32_9;
            longs[longs_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 3] & Self::MASK32_2) << 19;
            l2 |= tmp[tmp_idx + 4] << 8;
            l2 |= ((tmp[tmp_idx + 5] as u64) >> 3) as i64 & Self::MASK32_8;
            longs[longs_idx + 2] = l2;

            let mut l3 = (tmp[tmp_idx + 5] & Self::MASK32_3) << 18;
            l3 |= tmp[tmp_idx + 6] << 7;
            l3 |= ((tmp[tmp_idx + 7] as u64) >> 4) as i64 & Self::MASK32_7;
            longs[longs_idx + 3] = l3;

            let mut l4 = (tmp[tmp_idx + 7] & Self::MASK32_4) << 17;
            l4 |= tmp[tmp_idx + 8] << 6;
            l4 |= ((tmp[tmp_idx + 9] as u64) >> 5) as i64 & Self::MASK32_6;
            longs[longs_idx + 4] = l4;

            let mut l5 = (tmp[tmp_idx + 9] & Self::MASK32_5) << 16;
            l5 |= tmp[tmp_idx + 10] << 5;
            l5 |= ((tmp[tmp_idx + 11] as u64) >> 6) as i64 & Self::MASK32_5;
            longs[longs_idx + 5] = l5;

            let mut l6 = (tmp[tmp_idx + 11] & Self::MASK32_6) << 15;
            l6 |= tmp[tmp_idx + 12] << 4;
            l6 |= ((tmp[tmp_idx + 13] as u64) >> 7) as i64 & Self::MASK32_4;
            longs[longs_idx + 6] = l6;

            let mut l7 = (tmp[tmp_idx + 13] & Self::MASK32_7) << 14;
            l7 |= tmp[tmp_idx + 14] << 3;
            l7 |= ((tmp[tmp_idx + 15] as u64) >> 8) as i64 & Self::MASK32_3;
            longs[longs_idx + 7] = l7;

            let mut l8 = (tmp[tmp_idx + 15] & Self::MASK32_8) << 13;
            l8 |= tmp[tmp_idx + 16] << 2;
            l8 |= ((tmp[tmp_idx + 17] as u64) >> 9) as i64 & Self::MASK32_2;
            longs[longs_idx + 8] = l8;

            let mut l9 = (tmp[tmp_idx + 17] & Self::MASK32_9) << 12;
            l9 |= tmp[tmp_idx + 18] << 1;
            l9 |= ((tmp[tmp_idx + 19] as u64) >> 10) as i64 & Self::MASK32_1;
            longs[longs_idx + 9] = l9;

            let l10 = (tmp[tmp_idx + 19] & Self::MASK32_10) << 11 | tmp[tmp_idx + 20];
            longs[longs_idx + 10] = l10;

            tmp_idx += 21;
            longs_idx += 11;
        }
        Ok(())
    }
    fn decode22<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(44, longs, 10, 22, Self::MASK32_22, tmp, 0, Self::MASK32_10)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 44;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 12;
            l0 |= tmp[tmp_idx + 1] << 2;
            l0 |= ((tmp[tmp_idx + 2] as u64) >> 8) as i64 & Self::MASK32_2;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 2] & Self::MASK32_8) << 14;
            l1 |= tmp[tmp_idx + 3] << 4;
            l1 |= ((tmp[tmp_idx + 4] as u64) >> 6) as i64 & Self::MASK32_4;
            longs[longs_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 4] & Self::MASK32_6) << 16;
            l2 |= tmp[tmp_idx + 5] << 6;
            l2 |= ((tmp[tmp_idx + 6] as u64) >> 4) as i64 & Self::MASK32_6;
            longs[longs_idx + 2] = l2;

            let mut l3 = (tmp[tmp_idx + 6] & Self::MASK32_4) << 18;
            l3 |= tmp[tmp_idx + 7] << 8;
            l3 |= ((tmp[tmp_idx + 8] as u64) >> 2) as i64 & Self::MASK32_8;
            longs[longs_idx + 3] = l3;

            let l4 = (tmp[tmp_idx + 8] & Self::MASK32_2) << 20
                | tmp[tmp_idx + 9] << 10
                | tmp[tmp_idx + 10];
            longs[longs_idx + 4] = l4;

            tmp_idx += 11;
            longs_idx += 5;
        }
        Ok(())
    }

    fn decode23<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(46, longs, 9, 23, Self::MASK32_23, tmp, 0, Self::MASK32_9)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 46;
        for _ in 0..2 {
            let mut l0 = tmp[tmp_idx] << 14;
            l0 |= tmp[tmp_idx + 1] << 5;
            l0 |= ((tmp[tmp_idx + 2] as u64) >> 4) as i64 & Self::MASK32_5;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 2] & Self::MASK32_4) << 19;
            l1 |= tmp[tmp_idx + 3] << 10;
            l1 |= tmp[tmp_idx + 4] << 1;
            l1 |= ((tmp[tmp_idx + 5] as u64) >> 8) as i64 & Self::MASK32_1;
            longs[longs_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 5] & Self::MASK32_8) << 15;
            l2 |= tmp[tmp_idx + 6] << 6;
            l2 |= ((tmp[tmp_idx + 7] as u64) >> 3) as i64 & Self::MASK32_6;
            longs[longs_idx + 2] = l2;

            let mut l3 = (tmp[tmp_idx + 7] & Self::MASK32_3) << 20;
            l3 |= tmp[tmp_idx + 8] << 11;
            l3 |= tmp[tmp_idx + 9] << 2;
            l3 |= ((tmp[tmp_idx + 10] as u64) >> 7) as i64 & Self::MASK32_2;
            longs[longs_idx + 3] = l3;

            let mut l4 = (tmp[tmp_idx + 10] & Self::MASK32_7) << 16;
            l4 |= tmp[tmp_idx + 11] << 7;
            l4 |= ((tmp[tmp_idx + 12] as u64) >> 2) as i64 & Self::MASK32_7;
            longs[longs_idx + 4] = l4;

            let mut l5 = (tmp[tmp_idx + 12] & Self::MASK32_2) << 21;
            l5 |= tmp[tmp_idx + 13] << 12;
            l5 |= tmp[tmp_idx + 14] << 3;
            l5 |= ((tmp[tmp_idx + 15] as u64) >> 6) as i64 & Self::MASK32_3;
            longs[longs_idx + 5] = l5;

            let mut l6 = (tmp[tmp_idx + 15] & Self::MASK32_6) << 17;
            l6 |= tmp[tmp_idx + 16] << 8;
            l6 |= ((tmp[tmp_idx + 17] as u64) >> 1) as i64 & Self::MASK32_8;
            longs[longs_idx + 6] = l6;

            let mut l7 = (tmp[tmp_idx + 17] & Self::MASK32_1) << 22;
            l7 |= tmp[tmp_idx + 18] << 13;
            l7 |= tmp[tmp_idx + 19] << 4;
            l7 |= ((tmp[tmp_idx + 20] as u64) >> 5) as i64 & Self::MASK32_4;
            longs[longs_idx + 7] = l7;

            let l8 = (tmp[tmp_idx + 20] & Self::MASK32_5) << 18
                | tmp[tmp_idx + 21] << 9
                | tmp[tmp_idx + 22];
            longs[longs_idx + 8] = l8;

            tmp_idx += 23;
            longs_idx += 9;
        }
        Ok(())
    }

    fn decode24<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(48, longs, 8, 24, Self::MASK32_24, tmp, 0, Self::MASK32_8)?;
        let mut tmp_idx = 0;
        let mut longs_idx = 48;
        for _ in 0..16 {
            let l0 = tmp[tmp_idx] << 16 | tmp[tmp_idx + 1] << 8 | tmp[tmp_idx + 2];
            longs[longs_idx] = l0;
            tmp_idx += 3;
            longs_idx += 1;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use crate::codecs::lucene912::for_util::ForUtil;
    use crate::internal::vectorization::posting_decoding_util::PostingDecodingUtil;
    use crate::store::directory::Directory;
    use crate::store::{DataInput, DataOutput, IOContext, IndexInput, IndexOutput};
    use crate::test::util::lucene_test_case::{new_directory, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::array_util::ArrayUtil;
    use crate::util::error::lucene_error::Result;
    use crate::util::packed::PackedInts;
    use rand::Rng;
    use std::cell::RefCell;
    use std::rc::Rc;
    #[allow(dead_code)] // for quick search
    struct TestForUtil;

    #[test]
    fn test_encode_decode() -> Result<()> {
        let mut random = random();
        let iterations = random.random_range(50..=1000);
        let mut values = vec![0i32; iterations * ForUtil::BLOCK_SIZE];

        for i in 0..iterations {
            let bpv = TestUtil::next_int(&mut random, 1, 31);
            for j in 0..ForUtil::BLOCK_SIZE {
                let idx = i * ForUtil::BLOCK_SIZE + j;
                values[idx] = random.random_range(0..PackedInts::max_value(bpv) as i32);
            }
        }

        // TODO:  ByteBuffersDirectory not Implemented
        let mut dir = new_directory(&mut random)?;
        let end_pointer;

        // encode
        {
            let mut out = dir.create_output("test.bin", &IOContext::default_io_context()?)?;
            let mut for_util = ForUtil::new();

            for i in 0..iterations {
                let mut source = [0i64; ForUtil::BLOCK_SIZE];
                let mut or = 0;
                for j in 0..ForUtil::BLOCK_SIZE {
                    let v = values[i * ForUtil::BLOCK_SIZE + j] as i64;
                    source[j] = v;
                    or |= v;
                }
                let bpv = PackedInts::bits_required(or)?;
                out.write_byte(bpv as u8)?;
                for_util.encode(&mut source, bpv, &mut out)?;
            }
            end_pointer = out.get_file_pointer();
        }

        // decode
        {
            let input = Rc::new(RefCell::new(
                dir.open_input("test.bin", &IOContext::read_once_io_context()?)?,
            ));
            let mut pdu = PostingDecodingUtil::new(input.clone());
            let mut for_util = ForUtil::new();

            for i in 0..iterations {
                let bits_per_value = input.borrow_mut().read_byte()? as i32;
                let current_fp = input.borrow().get_file_pointer();
                let mut restored = [0i64; ForUtil::BLOCK_SIZE];
                for_util.decode(bits_per_value, &mut pdu, &mut restored)?;

                let ints: Vec<i32> = restored.iter().map(|&v| v as i32).collect();
                let expected = ArrayUtil::copy_of_sub_array(
                    &values,
                    (i * ForUtil::BLOCK_SIZE) as i32,
                    ((i + 1) * ForUtil::BLOCK_SIZE) as i32,
                );
                assert_eq!(
                    ints, expected,
                    "Mismatch at iteration {}: {:?} vs {:?}",
                    i, ints, expected
                );

                let expected_fp = current_fp + ForUtil::num_bytes(bits_per_value) as i64;
                assert_eq!(
                    input.borrow().get_file_pointer(),
                    expected_fp,
                    "Unexpected file pointer after decode at iteration {}",
                    i
                );
            }

            assert_eq!(
                input.borrow().get_file_pointer(),
                end_pointer,
                "Final file pointer mismatch"
            );
        }
        Ok(())
    }
}

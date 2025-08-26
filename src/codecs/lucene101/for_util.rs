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
/// Inspired by [bitpacking](https://fulmicoton.com/posts/bitpacking/)
///
/// Encodes multiple integers into a `long` to achieve SIMD-like speedups.
///
/// - If `bits_per_value <= 8`, then 8 integers are packed into each `long`.
/// - If `bits_per_value <= 16`, then 4 integers per `long`.
/// - Otherwise, 2 integers per `long`.
pub struct ForUtil {
    tmp: Vec<i32>,
}
impl ForUtil {
    pub(crate) fn new() -> Self {
        Self {
            tmp: vec![0i32; Self::BLOCK_SIZE],
        }
    }
    pub const BLOCK_SIZE: usize = 128;
    pub const BLOCK_SIZE_LOG2: usize = 7;

    const fn expand_mask16(mask16: i32) -> i32 {
        mask16 | (mask16 << 16)
    }

    const fn expand_mask8(mask8: i32) -> i32 {
        Self::expand_mask16(mask8 | (mask8 << 8))
    }

    const fn mask32(bits_per_value: i32) -> i32 {
        (1i32 << bits_per_value) - 1
    }

    const fn mask16(bits_per_value: i32) -> i32 {
        Self::expand_mask16((1i32 << bits_per_value) - 1)
    }

    const fn mask8(bits_per_value: i32) -> i32 {
        Self::expand_mask8((1i32 << bits_per_value) - 1)
    }
    pub(crate) fn expand8(arr: &mut [i32]) {
        for i in 0..32 {
            let l = arr[i] as u32;
            arr[i] = ((l >> 24) & 0xFF) as i32;
            arr[32 + i] = ((l >> 16) & 0xFF) as i32;
            arr[64 + i] = ((l >> 8) & 0xFF) as i32;
            arr[96 + i] = (l & 0xFF) as i32;
        }
    }
    pub(crate) fn collapse8(arr: &mut [i32]) {
        for i in 0..32 {
            arr[i] = (arr[i] << 24) | (arr[32 + i] << 16) | (arr[64 + i] << 8) | arr[96 + i];
        }
    }

    pub(crate) fn expand16(arr: &mut [i32]) {
        for i in 0..64 {
            let l = arr[i] as u32;
            arr[i] = ((l >> 16) & 0xFFFF) as i32;
            arr[64 + i] = (l & 0xFFFF) as i32;
        }
    }

    pub(crate) fn collapse16(arr: &mut [i32]) {
        for i in 0..64 {
            arr[i] = (arr[i] << 16) | (arr[64 + i]);
        }
    }

    /// Encode 128 integers from `ints` into out`.
    pub(crate) fn encode(
        &mut self,
        ints: &mut [i32],
        bits_per_value: i32,
        out: &mut impl DataOutput,
    ) -> Result<()> {
        let next_primitive = if bits_per_value <= 8 {
            Self::collapse8(ints);
            8
        } else if bits_per_value <= 16 {
            Self::collapse16(ints);
            16
        } else {
            32
        };
        Self::encode_with_tmp(ints, bits_per_value, next_primitive, out, &mut self.tmp)
    }

    pub(crate) fn encode_with_tmp(
        ints: &[i32],
        bits_per_value: i32,
        primitive_size: i32,
        out: &mut impl DataOutput,
        tmp: &mut [i32],
    ) -> Result<()> {
        let num_ints = Self::BLOCK_SIZE * (primitive_size as usize) / i32::BITS as usize;
        let num_ints_per_shift = (bits_per_value * 4) as usize;

        let mut idx = 0;
        let mut shift = primitive_size - bits_per_value;
        for (t, l) in tmp.iter_mut().take(num_ints_per_shift).zip(&ints[idx..]) {
            *t = *l << shift;
        }
        idx += num_ints_per_shift;

        shift -= bits_per_value;
        while shift >= 0 {
            for (t, l) in tmp.iter_mut().take(num_ints_per_shift).zip(&ints[idx..]) {
                *t |= *l << shift;
            }
            idx += num_ints_per_shift;
            shift -= bits_per_value;
        }

        let remaining_bits_per_int = shift + bits_per_value;
        let mask_remaining_bits_per_int = match primitive_size {
            8 => Self::MASKS8[remaining_bits_per_int as usize],
            16 => Self::MASKS16[remaining_bits_per_int as usize],
            _ => Self::MASKS32[remaining_bits_per_int as usize],
        };

        let mut tmp_idx = 0;
        let mut remaining_bits_per_value = bits_per_value;
        while idx < num_ints {
            if remaining_bits_per_value >= remaining_bits_per_int {
                remaining_bits_per_value -= remaining_bits_per_int;
                tmp[tmp_idx] |= (ints[idx] as u32 >> remaining_bits_per_value) as i32
                    & mask_remaining_bits_per_int;
                if remaining_bits_per_value == 0 {
                    idx += 1;
                    remaining_bits_per_value = bits_per_value;
                }
                tmp_idx += 1;
            } else {
                let remaining_bits_per_value_index = remaining_bits_per_value as usize;
                let remaining_bits_per_int_index = remaining_bits_per_int as usize;
                let (mask1, mask2) = match primitive_size {
                    8 => (
                        Self::MASKS8[remaining_bits_per_value_index],
                        Self::MASKS8[remaining_bits_per_int_index - remaining_bits_per_value_index],
                    ),
                    16 => (
                        Self::MASKS16[remaining_bits_per_value_index],
                        Self::MASKS16
                            [remaining_bits_per_int_index - remaining_bits_per_value_index],
                    ),
                    _ => (
                        Self::MASKS32[remaining_bits_per_value_index],
                        Self::MASKS32
                            [remaining_bits_per_int_index - remaining_bits_per_value_index],
                    ),
                };

                tmp[tmp_idx] |=
                    (ints[idx] & mask1) << (remaining_bits_per_int - remaining_bits_per_value);
                idx += 1;
                remaining_bits_per_value += bits_per_value - remaining_bits_per_int;
                tmp[tmp_idx] |= (ints[idx] as u32 >> remaining_bits_per_value) as i32 & mask2;
                tmp_idx += 1;
            }
        }
        for &val in tmp.iter().take(num_ints_per_shift) {
            out.write_int(val)?;
        }

        Ok(())
    }
    /// Number of bytes required to encode 128 integers of `bitsPerValue` bits
    /// per value.
    pub(crate) fn num_bytes(bits_per_value: i32) -> i32 {
        bits_per_value << (Self::BLOCK_SIZE_LOG2 - 3)
    }

    pub(crate) fn decode_slow<I: IndexInput>(
        bits_per_value: i32,
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        let num_ints = bits_per_value << 2;
        let mask = Self::MASKS32[bits_per_value as usize];
        pdu.split_ints_diff(num_ints, ints, 32 - bits_per_value, 32, mask, tmp, 0, -1)?;

        let remaining_bits_per_int = (32 - bits_per_value) as usize;
        let mask32_remaining_bits_per_int = Self::MASKS32[remaining_bits_per_int];

        let mut tmp_idx = 0;
        let mut remaining_bits = remaining_bits_per_int;
        #[allow(clippy::needless_range_loop)]
        for ints_idx in num_ints as usize..(Self::BLOCK_SIZE) {
            let mut b = bits_per_value as usize - remaining_bits;
            let mut l = (tmp[tmp_idx] & Self::MASKS32[remaining_bits]) << b;
            tmp_idx += 1;

            while b >= remaining_bits_per_int {
                b -= remaining_bits_per_int;
                l |= (tmp[tmp_idx] & mask32_remaining_bits_per_int) << b;
                tmp_idx += 1;
            }

            if b > 0 {
                l |= (tmp[tmp_idx] >> (remaining_bits_per_int - b)) & Self::MASKS32[b];
                remaining_bits = remaining_bits_per_int - b;
            } else {
                remaining_bits = remaining_bits_per_int;
            }

            ints[ints_idx] = l;
        }

        Ok(())
    }

    const MASKS8: [i32; 8] = {
        let mut masks = [0i32; 8];
        let mut i = 0;
        while i < 8 {
            masks[i] = Self::mask8(i as i32);
            i += 1;
        }
        masks
    };

    const MASKS16: [i32; 16] = {
        let mut masks = [0i32; 16];
        let mut i = 0;
        while i < 16 {
            masks[i] = Self::mask16(i as i32);
            i += 1;
        }
        masks
    };

    const MASKS32: [i32; 32] = {
        let mut masks = [0i32; 32];
        let mut i = 0;
        while i < 32 {
            masks[i] = Self::mask32(i as i32);
            i += 1;
        }
        masks
    };

    pub const MASK8_1: i32 = Self::MASKS8[1];
    pub const MASK8_2: i32 = Self::MASKS8[2];
    pub const MASK8_3: i32 = Self::MASKS8[3];
    pub const MASK8_4: i32 = Self::MASKS8[4];
    pub const MASK8_5: i32 = Self::MASKS8[5];
    pub const MASK8_6: i32 = Self::MASKS8[6];
    pub const MASK8_7: i32 = Self::MASKS8[7];

    pub const MASK16_1: i32 = Self::MASKS16[1];
    pub const MASK16_2: i32 = Self::MASKS16[2];
    pub const MASK16_3: i32 = Self::MASKS16[3];
    pub const MASK16_4: i32 = Self::MASKS16[4];
    pub const MASK16_5: i32 = Self::MASKS16[5];
    pub const MASK16_6: i32 = Self::MASKS16[6];
    pub const MASK16_7: i32 = Self::MASKS16[7];
    pub const MASK16_8: i32 = Self::MASKS16[8];
    pub const MASK16_9: i32 = Self::MASKS16[9];
    pub const MASK16_10: i32 = Self::MASKS16[10];
    pub const MASK16_11: i32 = Self::MASKS16[11];
    pub const MASK16_12: i32 = Self::MASKS16[12];
    pub const MASK16_13: i32 = Self::MASKS16[13];
    pub const MASK16_14: i32 = Self::MASKS16[14];
    pub const MASK16_15: i32 = Self::MASKS16[15];

    pub const MASK32_1: i32 = Self::MASKS32[1];
    pub const MASK32_2: i32 = Self::MASKS32[2];
    pub const MASK32_3: i32 = Self::MASKS32[3];
    pub const MASK32_4: i32 = Self::MASKS32[4];
    pub const MASK32_5: i32 = Self::MASKS32[5];
    pub const MASK32_6: i32 = Self::MASKS32[6];
    pub const MASK32_7: i32 = Self::MASKS32[7];
    pub const MASK32_8: i32 = Self::MASKS32[8];
    pub const MASK32_9: i32 = Self::MASKS32[9];
    pub const MASK32_10: i32 = Self::MASKS32[10];
    pub const MASK32_11: i32 = Self::MASKS32[11];
    pub const MASK32_12: i32 = Self::MASKS32[12];
    pub const MASK32_13: i32 = Self::MASKS32[13];
    pub const MASK32_14: i32 = Self::MASKS32[14];
    pub const MASK32_15: i32 = Self::MASKS32[15];
    pub const MASK32_16: i32 = Self::MASKS32[16];
    /// Decode 128 integers into `[i32]`.
    pub(crate) fn decode<I: IndexInput>(
        &mut self,
        bits_per_value: i32,
        pdu: &mut PostingDecodingUtil<I>,
        ints: &mut [i32],
    ) -> Result<()> {
        match bits_per_value {
            1 => {
                Self::decode1(pdu, ints)?;
                Self::expand8(ints);
            },
            2 => {
                Self::decode2(pdu, ints)?;
                Self::expand8(ints);
            },
            3 => {
                Self::decode3(pdu, &mut self.tmp, ints)?;
                Self::expand8(ints);
            },
            4 => {
                Self::decode4(pdu, ints)?;
                Self::expand8(ints);
            },
            5 => {
                Self::decode5(pdu, &mut self.tmp, ints)?;
                Self::expand8(ints);
            },
            6 => {
                Self::decode6(pdu, &mut self.tmp, ints)?;
                Self::expand8(ints);
            },
            7 => {
                Self::decode7(pdu, &mut self.tmp, ints)?;
                Self::expand8(ints);
            },
            8 => {
                Self::decode8(pdu, ints)?;
                Self::expand8(ints);
            },
            9 => {
                Self::decode9(pdu, &mut self.tmp, ints)?;
                Self::expand16(ints);
            },
            10 => {
                Self::decode10(pdu, &mut self.tmp, ints)?;
                Self::expand16(ints);
            },
            11 => {
                Self::decode11(pdu, &mut self.tmp, ints)?;
                Self::expand16(ints);
            },
            12 => {
                Self::decode12(pdu, &mut self.tmp, ints)?;
                Self::expand16(ints);
            },
            13 => {
                Self::decode13(pdu, &mut self.tmp, ints)?;
                Self::expand16(ints);
            },
            14 => {
                Self::decode14(pdu, &mut self.tmp, ints)?;
                Self::expand16(ints);
            },
            15 => {
                Self::decode15(pdu, &mut self.tmp, ints)?;
                Self::expand16(ints);
            },
            16 => {
                Self::decode16(pdu, ints)?;
                Self::expand16(ints);
            },
            _ => {
                Self::decode_slow(bits_per_value, pdu, &mut self.tmp, ints)?;
            },
        }
        Ok(())
    }

    pub(crate) fn decode1<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_same(4, ints, 7, 1, Self::MASK8_1, 28, Self::MASK8_1)
    }
    pub(crate) fn decode2<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_same(8, ints, 6, 2, Self::MASK8_2, 24, Self::MASK8_2)
    }

    pub(crate) fn decode3<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(12, ints, 5, 3, Self::MASK8_3, tmp, 0, Self::MASK8_2)?;

        let mut iter = 0;
        let mut tmp_idx = 0;
        let mut ints_idx = 24;

        while iter < 4 {
            let mut l0 = tmp[tmp_idx] << 1;
            l0 |= ((tmp[tmp_idx + 1] as u32) >> 1) as i32 & Self::MASK8_1;
            ints[ints_idx] = l0;
            let mut l1 = (tmp[tmp_idx + 1] & Self::MASK8_1) << 2;
            l1 |= tmp[tmp_idx + 2];
            ints[ints_idx + 1] = l1;
            iter += 1;
            tmp_idx += 3;
            ints_idx += 2;
        }
        Ok(())
    }
    pub(crate) fn decode4<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_same(16, ints, 4, 4, Self::MASK8_4, 16, Self::MASK8_4)
    }
    fn decode5<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(20, ints, 3, 5, Self::MASK8_5, tmp, 0, Self::MASK8_3)?;
        let mut tmp_idx = 0;
        let mut ints_idx = 20;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 2;
            l0 |= ((tmp[tmp_idx + 1] as u32) >> 1) as i32 & Self::MASK8_2;
            ints[ints_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & Self::MASK8_1) << 4;
            l1 |= tmp[tmp_idx + 2] << 1;
            l1 |= ((tmp[tmp_idx + 3] as u32) >> 2) as i32 & Self::MASK8_1;
            ints[ints_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 3] & Self::MASK8_2) << 3;
            l2 |= tmp[tmp_idx + 4];
            ints[ints_idx + 2] = l2;

            tmp_idx += 5;
            ints_idx += 3;
        }
        Ok(())
    }
    fn decode6<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(24, ints, 2, 6, Self::MASK8_6, tmp, 0, Self::MASK8_2)?;
        let mut tmp_idx = 0;
        let mut ints_idx = 24;
        for _ in 0..8 {
            let mut l0 = tmp[tmp_idx] << 4;
            l0 |= tmp[tmp_idx + 1] << 2;
            l0 |= tmp[tmp_idx + 2];
            ints[ints_idx] = l0;

            tmp_idx += 3;
            ints_idx += 1;
        }
        Ok(())
    }

    fn decode7<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(28, ints, 1, 7, Self::MASK8_7, tmp, 0, Self::MASK8_1)?;
        let mut tmp_idx = 0;
        let mut ints_idx = 28;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 6;
            l0 |= tmp[tmp_idx + 1] << 5;
            l0 |= tmp[tmp_idx + 2] << 4;
            l0 |= tmp[tmp_idx + 3] << 3;
            l0 |= tmp[tmp_idx + 4] << 2;
            l0 |= tmp[tmp_idx + 5] << 1;
            l0 |= tmp[tmp_idx + 6];
            ints[ints_idx] = l0;

            tmp_idx += 7;
            ints_idx += 1;
        }
        Ok(())
    }
    fn decode8<I: IndexInput>(pdu: &mut PostingDecodingUtil<I>, ints: &mut [i32]) -> Result<()> {
        pdu.input.borrow_mut().read_ints(ints, 0, 32)
    }

    pub(crate) fn decode9<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(36, ints, 7, 9, Self::MASK16_9, tmp, 0, Self::MASK16_7)?;
        let mut tmp_idx = 0;
        let mut ints_idx = 36;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 2;
            l0 |= ((tmp[tmp_idx + 1] as u32) >> 5) as i32 & Self::MASK16_2;
            ints[ints_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & Self::MASK16_5) << 4;
            l1 |= ((tmp[tmp_idx + 2] as u32) >> 3) as i32 & Self::MASK16_4;
            ints[ints_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 2] & Self::MASK16_3) << 6;
            l2 |= ((tmp[tmp_idx + 3] as u32) >> 1) as i32 & Self::MASK16_6;
            ints[ints_idx + 2] = l2;

            let mut l3 = (tmp[tmp_idx + 3] & Self::MASK16_1) << 8;
            l3 |= tmp[tmp_idx + 4] << 1;
            l3 |= ((tmp[tmp_idx + 5] as u32) >> 6) as i32 & Self::MASK16_1;
            ints[ints_idx + 3] = l3;

            let mut l4 = (tmp[tmp_idx + 5] & Self::MASK16_6) << 3;
            l4 |= ((tmp[tmp_idx + 6] as u32) >> 4) as i32 & Self::MASK16_3;
            ints[ints_idx + 4] = l4;

            let mut l5 = (tmp[tmp_idx + 6] & Self::MASK16_4) << 5;
            l5 |= ((tmp[tmp_idx + 7] as u32) >> 2) as i32 & Self::MASK16_5;
            ints[ints_idx + 5] = l5;

            let mut l6 = (tmp[tmp_idx + 7] & Self::MASK16_2) << 7;
            l6 |= tmp[tmp_idx + 8];
            ints[ints_idx + 6] = l6;

            tmp_idx += 9;
            ints_idx += 7;
        }
        Ok(())
    }

    pub(crate) fn decode10<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(40, ints, 6, 10, Self::MASK16_10, tmp, 0, Self::MASK16_6)?;
        let mut tmp_idx = 0;
        let mut ints_idx = 40;
        for _ in 0..8 {
            let mut l0 = tmp[tmp_idx] << 4;
            l0 |= ((tmp[tmp_idx + 1] as u32) >> 2) as i32 & Self::MASK16_4;
            ints[ints_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & Self::MASK16_2) << 8;
            l1 |= tmp[tmp_idx + 2] << 2;
            l1 |= ((tmp[tmp_idx + 3] as u32) >> 4) as i32 & Self::MASK16_2;
            ints[ints_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 3] & Self::MASK16_4) << 6;
            l2 |= tmp[tmp_idx + 4];
            ints[ints_idx + 2] = l2;

            tmp_idx += 5;
            ints_idx += 3;
        }
        Ok(())
    }

    pub(crate) fn decode11<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(44, ints, 5, 11, Self::MASK16_11, tmp, 0, Self::MASK16_5)?;
        let mut tmp_idx = 0;
        let mut ints_idx = 44;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 6;
            l0 |= tmp[tmp_idx + 1] << 1;
            l0 |= ((tmp[tmp_idx + 2] as u32) >> 4) as i32 & Self::MASK16_1;
            ints[ints_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 2] & Self::MASK16_4) << 7;
            l1 |= tmp[tmp_idx + 3] << 2;
            l1 |= ((tmp[tmp_idx + 4] as u32) >> 3) as i32 & Self::MASK16_2;
            ints[ints_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 4] & Self::MASK16_3) << 8;
            l2 |= tmp[tmp_idx + 5] << 3;
            l2 |= ((tmp[tmp_idx + 6] as u32) >> 2) as i32 & Self::MASK16_3;
            ints[ints_idx + 2] = l2;

            let mut l3 = (tmp[tmp_idx + 6] & Self::MASK16_2) << 9;
            l3 |= tmp[tmp_idx + 7] << 4;
            l3 |= ((tmp[tmp_idx + 8] as u32) >> 1) as i32 & Self::MASK16_4;
            ints[ints_idx + 3] = l3;

            let mut l4 = (tmp[tmp_idx + 8] & Self::MASK16_1) << 10;
            l4 |= tmp[tmp_idx + 9] << 5;
            l4 |= tmp[tmp_idx + 10];
            ints[ints_idx + 4] = l4;

            tmp_idx += 11;
            ints_idx += 5;
        }
        Ok(())
    }
    fn decode12<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(48, ints, 4, 12, Self::MASK16_12, tmp, 0, Self::MASK16_4)?;
        let mut tmp_idx = 0;
        let mut ints_idx = 48;
        for _ in 0..16 {
            let l0 = (tmp[tmp_idx] << 8) | (tmp[tmp_idx + 1] << 4) | tmp[tmp_idx + 2];
            ints[ints_idx] = l0;
            tmp_idx += 3;
            ints_idx += 1;
        }
        Ok(())
    }

    fn decode13<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(52, ints, 3, 13, Self::MASK16_13, tmp, 0, Self::MASK16_3)?;
        let mut tmp_idx = 0;
        let mut ints_idx = 52;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 10;
            l0 |= tmp[tmp_idx + 1] << 7;
            l0 |= tmp[tmp_idx + 2] << 4;
            l0 |= tmp[tmp_idx + 3] << 1;
            l0 |= ((tmp[tmp_idx + 4] as u32) >> 2) as i32 & Self::MASK16_1;
            ints[ints_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 4] & Self::MASK16_2) << 11;
            l1 |= tmp[tmp_idx + 5] << 8;
            l1 |= tmp[tmp_idx + 6] << 5;
            l1 |= tmp[tmp_idx + 7] << 2;
            l1 |= ((tmp[tmp_idx + 8] as u32) >> 1) as i32 & Self::MASK16_2;
            ints[ints_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 8] & Self::MASK16_1) << 12;
            l2 |= tmp[tmp_idx + 9] << 9;
            l2 |= tmp[tmp_idx + 10] << 6;
            l2 |= tmp[tmp_idx + 11] << 3;
            l2 |= tmp[tmp_idx + 12];
            ints[ints_idx + 2] = l2;

            tmp_idx += 13;
            ints_idx += 3;
        }
        Ok(())
    }

    fn decode14<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(56, ints, 2, 14, Self::MASK16_14, tmp, 0, Self::MASK16_2)?;
        let mut tmp_idx = 0;
        let mut ints_idx = 56;
        for _ in 0..8 {
            let l0 = (tmp[tmp_idx] << 12)
                | (tmp[tmp_idx + 1] << 10)
                | (tmp[tmp_idx + 2] << 8)
                | (tmp[tmp_idx + 3] << 6)
                | (tmp[tmp_idx + 4] << 4)
                | (tmp[tmp_idx + 5] << 2)
                | tmp[tmp_idx + 6];
            ints[ints_idx] = l0;
            tmp_idx += 7;
            ints_idx += 1;
        }
        Ok(())
    }

    fn decode15<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(60, ints, 1, 15, Self::MASK16_15, tmp, 0, Self::MASK16_1)?;
        let mut tmp_idx = 0;
        let mut ints_idx = 60;
        for _ in 0..4 {
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
            ints[ints_idx] = l0;
            tmp_idx += 15;
            ints_idx += 1;
        }
        Ok(())
    }

    fn decode16<I: IndexInput>(pdu: &mut PostingDecodingUtil<I>, ints: &mut [i32]) -> Result<()> {
        pdu.input.borrow_mut().read_ints(ints, 0, 64)
    }
}
#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use rand::Rng;

    use crate::codecs::lucene101::for_util::ForUtil;
    use crate::internal::vectorization::posting_decoding_util::PostingDecodingUtil;
    use crate::store::directory::Directory;
    use crate::store::{DataInput, DataOutput, IOContext, IndexInput, IndexOutput};
    use crate::test::util::lucene_test_case::lucene_test_case_util::{new_directory, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::Result;
    use crate::util::packed::PackedInts;
    #[allow(dead_code)] // for quick search
    struct TestForUtil;
    #[test]
    fn test_encode_decode() -> Result<()> {
        let mut random = random();
        let iterations = random.random_range(50..1000);
        let mut values = vec![0i32; iterations * ForUtil::BLOCK_SIZE];

        for i in 0..iterations {
            let bpv = TestUtil::next_int(&mut random, 1, 31);
            for j in 0..ForUtil::BLOCK_SIZE {
                let max_val = PackedInts::max_value(bpv) as i32;
                values[i * ForUtil::BLOCK_SIZE + j] = random.random_range(0..=max_val);
            }
        }

        // TODO:: 这里要换成ByteBuffersDirectory
        let dir = new_directory(&mut random)?;
        let end_pointer;

        {
            // encode
            let mut out = dir.create_output("test.bin", &IOContext::default_io_context()?)?;
            let mut for_util = ForUtil::new();

            for i in 0..iterations {
                let mut source = vec![0i32; ForUtil::BLOCK_SIZE];
                let mut or = 0i64;

                for j in 0..ForUtil::BLOCK_SIZE {
                    let v = values[i * ForUtil::BLOCK_SIZE + j];
                    source[j] = v;
                    or |= v as i64;
                }

                let bpv = PackedInts::bits_required(or)?;
                out.write_byte(bpv as u8)?;
                for_util.encode(&mut source, bpv, &mut out)?;
            }

            end_pointer = out.get_file_pointer();
        }

        {
            // decode
            let input = Rc::new(RefCell::new(
                dir.open_input("test.bin", &IOContext::read_once_io_context()?)?,
            ));
            let mut pdu = PostingDecodingUtil::new(input.clone());
            let mut for_util = ForUtil::new();

            for i in 0..iterations {
                let bits_per_value = input.borrow_mut().read_byte()? as i32;
                let current_fp = input.borrow().get_file_pointer();
                let mut restored = vec![0i32; ForUtil::BLOCK_SIZE];

                for_util.decode(bits_per_value, &mut pdu, &mut restored)?;
                let restored_ints: Vec<i32> = restored.to_vec();

                let expected = &values[i * ForUtil::BLOCK_SIZE..(i + 1) * ForUtil::BLOCK_SIZE];
                assert_eq!(
                    restored_ints,
                    expected.to_vec(),
                    "Mismatch at iteration {}",
                    i
                );

                let expected_bytes = ForUtil::num_bytes(bits_per_value) as i64;
                let actual_bytes = input.borrow().get_file_pointer() - current_fp;
                assert_eq!(
                    expected_bytes, actual_bytes,
                    "Unexpected byte count at iteration {}",
                    i
                );
            }

            assert_eq!(end_pointer, input.borrow().get_file_pointer());
        }

        Ok(())
    }
}

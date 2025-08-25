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
use once_cell::sync::Lazy;

use crate::codecs::lucene101::for_util::ForUtil;
use crate::codecs::lucene101::pfor_util::PForUtil;
use crate::internal::vectorization::posting_decoding_util::PostingDecodingUtil;
use crate::store::{DataOutput, IndexInput};
use crate::util::SliceCopyOps;
use crate::util::error::lucene_error::Result;
use crate::util::packed::PackedInts;

static IDENTITY_PLUS_ONE: Lazy<Vec<i32>> = Lazy::new(|| (1..=ForUtil::BLOCK_SIZE as i32).collect());

/// Inspired from <https://fulmicoton.com/posts/bitpacking/>
/// Encodes multiple integers in a long to get SIMD-like speedups.
/// If `bits_per_value <= 4` then we pack 8 ints per long,
/// else if `bits_per_value <= 11` we pack 4 ints per long,
/// else we pack 2 ints per long.
pub struct ForDeltaUtil {
    tmp: Vec<i32>,
}

impl ForDeltaUtil {
    pub(crate) fn new() -> Self {
        Self {
            tmp: vec![0; ForUtil::BLOCK_SIZE],
        }
    }
}

impl ForDeltaUtil {
    const BLOCK_SIZE: usize = ForUtil::BLOCK_SIZE;
    const HALF_BLOCK_SIZE: usize = ForUtil::BLOCK_SIZE / 2;
    const ONE_BLOCK_SIZE_FOURTH: usize = Self::BLOCK_SIZE / 4;
    const TWO_BLOCK_SIZE_FOURTHS: usize = Self::BLOCK_SIZE / 2;
    const THREE_BLOCK_SIZE_FOURTHS: usize = 3 * Self::BLOCK_SIZE / 4;

    fn prefix_sum_of_ones(arr: &mut [i32], base: i32) {
        arr.copy_from(&IDENTITY_PLUS_ONE, 0);
        for v in arr.iter_mut() {
            *v += base;
        }
    }
    fn prefix_sum8(arr: &mut [i32], base: i32) {
        // When the number of bits per value is 4 or less, we can sum up all
        // values in a block without risking overflowing an 8-bits
        // integer. This allows computing the prefix sum by summing up 4
        // values at once.
        Self::inner_prefix_sum8(arr);
        ForUtil::expand8(arr);

        let l0 = base;
        let l1 = l0 + arr[Self::ONE_BLOCK_SIZE_FOURTH - 1];
        let l2 = l1 + arr[Self::TWO_BLOCK_SIZE_FOURTHS - 1];
        let l3 = l2 + arr[Self::THREE_BLOCK_SIZE_FOURTHS - 1];

        for i in 0..Self::ONE_BLOCK_SIZE_FOURTH {
            arr[i] += l0;
            arr[Self::ONE_BLOCK_SIZE_FOURTH + i] += l1;
            arr[Self::TWO_BLOCK_SIZE_FOURTHS + i] += l2;
            arr[Self::THREE_BLOCK_SIZE_FOURTHS + i] += l3;
        }
    }

    fn prefix_sum16(arr: &mut [i32], base: i32) {
        // When the number of bits per value is 11 or less, we can sum up all
        // values in a block without risking overflowing a 16-bits
        // integer. This allows computing the prefix sum by summing up 4
        // values at once.
        Self::inner_prefix_sum16(arr);
        ForUtil::expand16(arr);

        let l0 = base;
        let l1 = l0 + arr[Self::HALF_BLOCK_SIZE - 1];

        for i in 0..Self::HALF_BLOCK_SIZE {
            arr[i] += l0;
            arr[Self::HALF_BLOCK_SIZE + i] += l1;
        }
    }

    fn prefix_sum32(arr: &mut [i32], base: i32) {
        arr[0] += base;
        for i in 1..Self::BLOCK_SIZE {
            arr[i] += arr[i - 1];
        }
    }
    // For some reason unrolling seems to help
    fn inner_prefix_sum8(arr: &mut [i32]) {
        arr[1] += arr[0];
        arr[2] += arr[1];
        arr[3] += arr[2];
        arr[4] += arr[3];
        arr[5] += arr[4];
        arr[6] += arr[5];
        arr[7] += arr[6];
        arr[8] += arr[7];
        arr[9] += arr[8];
        arr[10] += arr[9];
        arr[11] += arr[10];
        arr[12] += arr[11];
        arr[13] += arr[12];
        arr[14] += arr[13];
        arr[15] += arr[14];
        arr[16] += arr[15];
        arr[17] += arr[16];
        arr[18] += arr[17];
        arr[19] += arr[18];
        arr[20] += arr[19];
        arr[21] += arr[20];
        arr[22] += arr[21];
        arr[23] += arr[22];
        arr[24] += arr[23];
        arr[25] += arr[24];
        arr[26] += arr[25];
        arr[27] += arr[26];
        arr[28] += arr[27];
        arr[29] += arr[28];
        arr[30] += arr[29];
        arr[31] += arr[30];
    }
    // For some reason unrolling seems to help
    fn inner_prefix_sum16(arr: &mut [i32]) {
        arr[1] += arr[0];
        arr[2] += arr[1];
        arr[3] += arr[2];
        arr[4] += arr[3];
        arr[5] += arr[4];
        arr[6] += arr[5];
        arr[7] += arr[6];
        arr[8] += arr[7];
        arr[9] += arr[8];
        arr[10] += arr[9];
        arr[11] += arr[10];
        arr[12] += arr[11];
        arr[13] += arr[12];
        arr[14] += arr[13];
        arr[15] += arr[14];
        arr[16] += arr[15];
        arr[17] += arr[16];
        arr[18] += arr[17];
        arr[19] += arr[18];
        arr[20] += arr[19];
        arr[21] += arr[20];
        arr[22] += arr[21];
        arr[23] += arr[22];
        arr[24] += arr[23];
        arr[25] += arr[24];
        arr[26] += arr[25];
        arr[27] += arr[26];
        arr[28] += arr[27];
        arr[29] += arr[28];
        arr[30] += arr[29];
        arr[31] += arr[30];
        arr[32] += arr[31];
        arr[33] += arr[32];
        arr[34] += arr[33];
        arr[35] += arr[34];
        arr[36] += arr[35];
        arr[37] += arr[36];
        arr[38] += arr[37];
        arr[39] += arr[38];
        arr[40] += arr[39];
        arr[41] += arr[40];
        arr[42] += arr[41];
        arr[43] += arr[42];
        arr[44] += arr[43];
        arr[45] += arr[44];
        arr[46] += arr[45];
        arr[47] += arr[46];
        arr[48] += arr[47];
        arr[49] += arr[48];
        arr[50] += arr[49];
        arr[51] += arr[50];
        arr[52] += arr[51];
        arr[53] += arr[52];
        arr[54] += arr[53];
        arr[55] += arr[54];
        arr[56] += arr[55];
        arr[57] += arr[56];
        arr[58] += arr[57];
        arr[59] += arr[58];
        arr[60] += arr[59];
        arr[61] += arr[60];
        arr[62] += arr[61];
        arr[63] += arr[62];
    }
    /// Encode deltas of a strictly monotonically increasing sequence of
    /// integers. The provided ints are expected to be deltas between
    /// consecutive values.
    pub fn encode_deltas(&mut self, ints: &mut [i32], out: &mut impl DataOutput) -> Result<()> {
        if ints[0] == 1 && PForUtil::all_equal(ints) {
            out.write_byte(0)?;
        } else {
            let mut or = 0;
            for &l in ints.iter() {
                or |= l;
            }
            debug_assert!(or != 0);

            let bits_per_value = PackedInts::bits_required(or as i64)?;
            out.write_byte(bits_per_value as u8)?;

            let primitive_size = if bits_per_value <= 3 {
                ForUtil::collapse8(ints);
                8
            } else if bits_per_value <= 10 {
                ForUtil::collapse16(ints);
                16
            } else {
                32
            };

            ForUtil::encode_with_tmp(ints, bits_per_value, primitive_size, out, &mut self.tmp)?;
        }

        Ok(())
    }
    /// Decode deltas, compute the prefix sum and add `base` to all decoded
    /// ints.
    pub(crate) fn decode_and_prefix_sum<I: IndexInput>(
        &mut self,
        pdu: &mut PostingDecodingUtil<I>,
        base: i32,
        ints: &mut [i32],
    ) -> Result<()> {
        let bits_per_value = pdu.input.borrow_mut().read_byte()? as i32;
        if bits_per_value == 0 {
            Self::prefix_sum_of_ones(ints, base);
        } else {
            self.decode_and_prefix_sum_with_bits(bits_per_value, pdu, base, ints)?;
        }
        Ok(())
    }
    /// Delta-decode 128 integers into `ints`.
    fn decode_and_prefix_sum_with_bits<I: IndexInput>(
        &mut self,
        bits_per_value: i32,
        pdu: &mut PostingDecodingUtil<I>,
        base: i32,
        ints: &mut [i32],
    ) -> Result<()> {
        match bits_per_value {
            1 => {
                ForUtil::decode1(pdu, ints)?;
                ForDeltaUtil::prefix_sum8(ints, base);
            },
            2 => {
                ForUtil::decode2(pdu, ints)?;
                ForDeltaUtil::prefix_sum8(ints, base);
            },
            3 => {
                ForUtil::decode3(pdu, &mut self.tmp, ints)?;
                ForDeltaUtil::prefix_sum8(ints, base);
            },
            4 => {
                ForDeltaUtil::decode_4_to_16(pdu, ints)?;
                ForDeltaUtil::prefix_sum16(ints, base);
            },
            5 => {
                ForDeltaUtil::decode_5_to_16(pdu, &mut self.tmp, ints)?;
                ForDeltaUtil::prefix_sum16(ints, base);
            },
            6 => {
                ForDeltaUtil::decode_6_to_16(pdu, &mut self.tmp, ints)?;
                ForDeltaUtil::prefix_sum16(ints, base);
            },
            7 => {
                ForDeltaUtil::decode_7_to_16(pdu, &mut self.tmp, ints)?;
                ForDeltaUtil::prefix_sum16(ints, base);
            },
            8 => {
                ForDeltaUtil::decode_8_to_16(pdu, ints)?;
                ForDeltaUtil::prefix_sum16(ints, base);
            },
            9 => {
                ForUtil::decode9(pdu, &mut self.tmp, ints)?;
                ForDeltaUtil::prefix_sum16(ints, base);
            },
            10 => {
                ForUtil::decode10(pdu, &mut self.tmp, ints)?;
                ForDeltaUtil::prefix_sum16(ints, base);
            },
            11 => {
                ForDeltaUtil::decode_11_to_32(pdu, &mut self.tmp, ints)?;
                ForDeltaUtil::prefix_sum32(ints, base);
            },
            12 => {
                ForDeltaUtil::decode_12_to_32(pdu, &mut self.tmp, ints)?;
                ForDeltaUtil::prefix_sum32(ints, base);
            },
            13 => {
                ForDeltaUtil::decode_13_to_32(pdu, &mut self.tmp, ints)?;
                ForDeltaUtil::prefix_sum32(ints, base);
            },
            14 => {
                ForDeltaUtil::decode_14_to_32(pdu, &mut self.tmp, ints)?;
                ForDeltaUtil::prefix_sum32(ints, base);
            },
            15 => {
                ForDeltaUtil::decode_15_to_32(pdu, &mut self.tmp, ints)?;
                ForDeltaUtil::prefix_sum32(ints, base);
            },
            16 => {
                ForDeltaUtil::decode_16_to_32(pdu, ints)?;
                ForDeltaUtil::prefix_sum32(ints, base);
            },
            _ => {
                ForUtil::decode_slow(bits_per_value, pdu, &mut self.tmp, ints)?;
                ForDeltaUtil::prefix_sum32(ints, base);
            },
        }
        Ok(())
    }
    fn decode_4_to_16<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_same(16, ints, 12, 4, ForUtil::MASK16_4, 48, ForUtil::MASK16_4)
    }
    fn decode_5_to_16<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(
            20,
            ints,
            11,
            5,
            ForUtil::MASK16_5,
            tmp,
            0,
            ForUtil::MASK16_1,
        )?;
        let mut tmp_idx = 0;
        let mut ints_idx = 60;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 4;
            l0 |= tmp[tmp_idx + 1] << 3;
            l0 |= tmp[tmp_idx + 2] << 2;
            l0 |= tmp[tmp_idx + 3] << 1;
            l0 |= tmp[tmp_idx + 4];
            ints[ints_idx] = l0;
            tmp_idx += 5;
            ints_idx += 1;
        }
        Ok(())
    }

    fn decode_6_to_16<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(
            24,
            ints,
            10,
            6,
            ForUtil::MASK16_6,
            tmp,
            0,
            ForUtil::MASK16_4,
        )?;
        let mut tmp_idx = 0;
        let mut ints_idx = 48;
        for _ in 0..8 {
            let mut l0 = tmp[tmp_idx] << 2;
            l0 |= ((tmp[tmp_idx + 1] as u64 >> 2) as i32) & ForUtil::MASK16_2;
            ints[ints_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & ForUtil::MASK16_2) << 4;
            l1 |= tmp[tmp_idx + 2];
            ints[ints_idx + 1] = l1;

            tmp_idx += 3;
            ints_idx += 2;
        }
        Ok(())
    }
    fn decode_7_to_16<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(28, ints, 9, 7, ForUtil::MASK16_7, tmp, 0, ForUtil::MASK16_2)?;
        let mut tmp_idx = 0;
        let mut ints_idx = 56;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 5;
            l0 |= tmp[tmp_idx + 1] << 3;
            l0 |= tmp[tmp_idx + 2] << 1;
            l0 |= ((tmp[tmp_idx + 3] as u64) >> 1) as i32 & ForUtil::MASK16_1;
            ints[ints_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 3] & ForUtil::MASK16_1) << 6;
            l1 |= tmp[tmp_idx + 4] << 4;
            l1 |= tmp[tmp_idx + 5] << 2;
            l1 |= tmp[tmp_idx + 6];
            ints[ints_idx + 1] = l1;

            tmp_idx += 7;
            ints_idx += 2;
        }
        Ok(())
    }
    fn decode_8_to_16<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_same(32, ints, 8, 8, ForUtil::MASK16_8, 32, ForUtil::MASK16_8)
    }
    fn decode_11_to_32<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(
            44,
            ints,
            21,
            11,
            ForUtil::MASK32_11,
            tmp,
            0,
            ForUtil::MASK32_10,
        )?;

        let mut tmp_idx = 0;
        let mut ints_idx = 88;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 1;
            l0 |= ((tmp[tmp_idx + 1] as u32) >> 9) as i32 & ForUtil::MASK32_1;
            ints[ints_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & ForUtil::MASK32_9) << 2;
            l1 |= ((tmp[tmp_idx + 2] as u32) >> 8) as i32 & ForUtil::MASK32_2;
            ints[ints_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 2] & ForUtil::MASK32_8) << 3;
            l2 |= ((tmp[tmp_idx + 3] as u32) >> 7) as i32 & ForUtil::MASK32_3;
            ints[ints_idx + 2] = l2;

            let mut l3 = (tmp[tmp_idx + 3] & ForUtil::MASK32_7) << 4;
            l3 |= ((tmp[tmp_idx + 4] as u32) >> 6) as i32 & ForUtil::MASK32_4;
            ints[ints_idx + 3] = l3;

            let mut l4 = (tmp[tmp_idx + 4] & ForUtil::MASK32_6) << 5;
            l4 |= ((tmp[tmp_idx + 5] as u32) >> 5) as i32 & ForUtil::MASK32_5;
            ints[ints_idx + 4] = l4;

            let mut l5 = (tmp[tmp_idx + 5] & ForUtil::MASK32_5) << 6;
            l5 |= ((tmp[tmp_idx + 6] as u32) >> 4) as i32 & ForUtil::MASK32_6;
            ints[ints_idx + 5] = l5;

            let mut l6 = (tmp[tmp_idx + 6] & ForUtil::MASK32_4) << 7;
            l6 |= ((tmp[tmp_idx + 7] as u32) >> 3) as i32 & ForUtil::MASK32_7;
            ints[ints_idx + 6] = l6;

            let mut l7 = (tmp[tmp_idx + 7] & ForUtil::MASK32_3) << 8;
            l7 |= ((tmp[tmp_idx + 8] as u32) >> 2) as i32 & ForUtil::MASK32_8;
            ints[ints_idx + 7] = l7;

            let mut l8 = (tmp[tmp_idx + 8] & ForUtil::MASK32_2) << 9;
            l8 |= ((tmp[tmp_idx + 9] as u32) >> 1) as i32 & ForUtil::MASK32_9;
            ints[ints_idx + 8] = l8;

            let mut l9 = (tmp[tmp_idx + 9] & ForUtil::MASK32_1) << 10;
            l9 |= tmp[tmp_idx + 10];
            ints[ints_idx + 9] = l9;

            tmp_idx += 11;
            ints_idx += 10;
        }

        Ok(())
    }
    fn decode_12_to_32<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(
            48,
            ints,
            20,
            12,
            ForUtil::MASK32_12,
            tmp,
            0,
            ForUtil::MASK32_8,
        )?;
        let mut tmp_idx = 0;
        let mut ints_idx = 96;
        for _ in 0..16 {
            let mut l0 = tmp[tmp_idx] << 4;
            l0 |= ((tmp[tmp_idx + 1] as u64) >> 4) as i32 & ForUtil::MASK32_4;
            ints[ints_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & ForUtil::MASK32_4) << 8;
            l1 |= tmp[tmp_idx + 2];
            ints[ints_idx + 1] = l1;

            tmp_idx += 3;
            ints_idx += 2;
        }
        Ok(())
    }
    fn decode_13_to_32<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(
            52,
            ints,
            19,
            13,
            ForUtil::MASK32_13,
            tmp,
            0,
            ForUtil::MASK32_6,
        )?;
        let mut tmp_idx = 0;
        let mut ints_idx = 104;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 7;
            l0 |= tmp[tmp_idx + 1] << 1;
            l0 |= ((tmp[tmp_idx + 2] as u64) >> 5) as i32 & ForUtil::MASK32_1;
            ints[ints_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 2] & ForUtil::MASK32_5) << 8;
            l1 |= tmp[tmp_idx + 3] << 2;
            l1 |= ((tmp[tmp_idx + 4] as u64) >> 4) as i32 & ForUtil::MASK32_2;
            ints[ints_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 4] & ForUtil::MASK32_4) << 9;
            l2 |= tmp[tmp_idx + 5] << 3;
            l2 |= ((tmp[tmp_idx + 6] as u64) >> 3) as i32 & ForUtil::MASK32_3;
            ints[ints_idx + 2] = l2;

            let mut l3 = (tmp[tmp_idx + 6] & ForUtil::MASK32_3) << 10;
            l3 |= tmp[tmp_idx + 7] << 4;
            l3 |= ((tmp[tmp_idx + 8] as u64) >> 2) as i32 & ForUtil::MASK32_4;
            ints[ints_idx + 3] = l3;

            let mut l4 = (tmp[tmp_idx + 8] & ForUtil::MASK32_2) << 11;
            l4 |= tmp[tmp_idx + 9] << 5;
            l4 |= ((tmp[tmp_idx + 10] as u64) >> 1) as i32 & ForUtil::MASK32_5;
            ints[ints_idx + 4] = l4;

            let mut l5 = (tmp[tmp_idx + 10] & ForUtil::MASK32_1) << 12;
            l5 |= tmp[tmp_idx + 11] << 6;
            l5 |= tmp[tmp_idx + 12];
            ints[ints_idx + 5] = l5;

            tmp_idx += 13;
            ints_idx += 6;
        }
        Ok(())
    }
    fn decode_14_to_32<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(
            56,
            ints,
            18,
            14,
            ForUtil::MASK32_14,
            tmp,
            0,
            ForUtil::MASK32_4,
        )?;
        let mut tmp_idx = 0;
        let mut ints_idx = 112;
        for _ in 0..8 {
            let mut l0 = tmp[tmp_idx] << 10;
            l0 |= tmp[tmp_idx + 1] << 6;
            l0 |= tmp[tmp_idx + 2] << 2;
            l0 |= ((tmp[tmp_idx + 3] as u64) >> 2) as i32 & ForUtil::MASK32_2;
            ints[ints_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 3] & ForUtil::MASK32_2) << 12;
            l1 |= tmp[tmp_idx + 4] << 8;
            l1 |= tmp[tmp_idx + 5] << 4;
            l1 |= tmp[tmp_idx + 6];
            ints[ints_idx + 1] = l1;

            tmp_idx += 7;
            ints_idx += 2;
        }
        Ok(())
    }
    fn decode_15_to_32<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i32],
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_diff(
            60,
            ints,
            17,
            15,
            ForUtil::MASK32_15,
            tmp,
            0,
            ForUtil::MASK32_2,
        )?;
        let mut tmp_idx = 0;
        let mut ints_idx = 120;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 13;
            l0 |= tmp[tmp_idx + 1] << 11;
            l0 |= tmp[tmp_idx + 2] << 9;
            l0 |= tmp[tmp_idx + 3] << 7;
            l0 |= tmp[tmp_idx + 4] << 5;
            l0 |= tmp[tmp_idx + 5] << 3;
            l0 |= tmp[tmp_idx + 6] << 1;
            l0 |= ((tmp[tmp_idx + 7] as u64) >> 1) as i32 & ForUtil::MASK32_1;
            ints[ints_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 7] & ForUtil::MASK32_1) << 14;
            l1 |= tmp[tmp_idx + 8] << 12;
            l1 |= tmp[tmp_idx + 9] << 10;
            l1 |= tmp[tmp_idx + 10] << 8;
            l1 |= tmp[tmp_idx + 11] << 6;
            l1 |= tmp[tmp_idx + 12] << 4;
            l1 |= tmp[tmp_idx + 13] << 2;
            l1 |= tmp[tmp_idx + 14];
            ints[ints_idx + 1] = l1;

            tmp_idx += 15;
            ints_idx += 2;
        }
        Ok(())
    }
    fn decode_16_to_32<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        ints: &mut [i32],
    ) -> Result<()> {
        pdu.split_ints_same(64, ints, 16, 16, ForUtil::MASK32_16, 64, ForUtil::MASK32_16)
    }
}
#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use rand::Rng;

    use crate::codecs::lucene101::for_delta_util::ForDeltaUtil;
    use crate::codecs::lucene101::for_util::ForUtil;
    use crate::internal::vectorization::posting_decoding_util::PostingDecodingUtil;
    use crate::store::directory::Directory;
    use crate::store::{IOContext, IndexInput, IndexOutput};
    use crate::test::util::lucene_test_case::lucene_test_case_util::{new_directory, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::Result;
    use crate::util::packed::PackedInts;
    #[allow(dead_code)]
    struct TestForDeltaUtil;
    #[test]
    fn test_encode_decode() -> Result<()> {
        let mut random = random();
        let iterations = random.random_range(50..=1000);
        let mut values = vec![0i32; iterations * ForUtil::BLOCK_SIZE];

        for i in 0..iterations {
            let bpv = TestUtil::next_int(&mut random, 1, 31 - 7);
            for j in 0..ForUtil::BLOCK_SIZE {
                values[i * ForUtil::BLOCK_SIZE + j] =
                    random.random_range(1..=PackedInts::max_value(bpv) as i32);
            }
        }

        // TODO: ByteBuffersDirectory not Implemented
        let mut d = new_directory(&mut random)?;
        let end_pointer;

        // encode
        {
            let mut out = d.create_output("test.bin", &IOContext::default_io_context()?)?;
            let mut for_delta_util = ForDeltaUtil::new();

            for i in 0..iterations {
                let mut source = vec![0i32; ForUtil::BLOCK_SIZE];
                for j in 0..ForUtil::BLOCK_SIZE {
                    source[j] = values[i * ForUtil::BLOCK_SIZE + j] as i32;
                }
                for_delta_util.encode_deltas(&mut source, &mut out)?;
            }
            end_pointer = out.get_file_pointer();
        }

        // decode
        {
            let input = Rc::new(RefCell::new(
                d.open_input("test.bin", &IOContext::read_once_io_context()?)?,
            ));
            // TODO: VECTORIZATION_PROVIDER not Implemented
            let mut pdu = PostingDecodingUtil::new(input.clone());
            let mut for_delta_util = ForDeltaUtil::new();

            for i in 0..iterations {
                let base = 0i32;
                let mut restored = vec![0i32; ForUtil::BLOCK_SIZE];
                for_delta_util.decode_and_prefix_sum(&mut pdu, base, &mut restored)?;

                let mut expected = vec![0i32; ForUtil::BLOCK_SIZE];
                for j in 0..ForUtil::BLOCK_SIZE {
                    expected[j] = values[i * ForUtil::BLOCK_SIZE + j] as i32;
                    if j > 0 {
                        expected[j] += expected[j - 1];
                    } else {
                        expected[j] += base;
                    }
                }

                assert_eq!(
                    restored, expected,
                    "Mismatch at iteration {}: restored = {:?}, expected = {:?}",
                    i, restored, expected
                );
            }

            assert_eq!(end_pointer, input.borrow().get_file_pointer());
        }
        Ok(())
    }
}

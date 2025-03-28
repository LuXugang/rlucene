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
use crate::codecs::lucene912::for_util::ForUtil;
use crate::codecs::lucene912::pfor_util::PForUtil;
use crate::internal::vectorization::posting_decoding_util::PostingDecodingUtil;
use crate::store::{DataOutput, IndexInput};
use crate::util::error::lucene_error::Result;
use crate::util::packed::PackedInts;
use crate::util::SliceCopyOps;
use once_cell::sync::Lazy;

static IDENTITY_PLUS_ONE: Lazy<Vec<i64>> = Lazy::new(|| (1..=ForUtil::BLOCK_SIZE as i64).collect());

/// Inspired from <https://fulmicoton.com/posts/bitpacking/>
/// Encodes multiple integers in a long to get SIMD-like speedups.
/// If `bits_per_value <= 4` then we pack 8 ints per long,
/// else if `bits_per_value <= 11` we pack 4 ints per long,
/// else we pack 2 ints per long.
#[allow(unused)]
pub struct ForDeltaUtil {
    tmp: Vec<i64>,
}
#[allow(unused)]
impl ForDeltaUtil {
    pub(crate) fn new() -> Self {
        Self {
            tmp: vec![0; ForUtil::BLOCK_SIZE / 2],
        }
    }
}
#[allow(unused)]
impl ForDeltaUtil {
    const BLOCK_SIZE: usize = ForUtil::BLOCK_SIZE;
    const ONE_BLOCK_SIZE_FOURTH: usize = Self::BLOCK_SIZE / 4;
    const TWO_BLOCK_SIZE_FOURTHS: usize = Self::BLOCK_SIZE / 2;
    const THREE_BLOCK_SIZE_FOURTHS: usize = 3 * Self::BLOCK_SIZE / 4;

    const ONE_BLOCK_SIZE_EIGHT: usize = Self::BLOCK_SIZE / 8;
    const TWO_BLOCK_SIZE_EIGHTS: usize = Self::BLOCK_SIZE / 4;
    const THREE_BLOCK_SIZE_EIGHTS: usize = 3 * Self::BLOCK_SIZE / 8;
    const FOUR_BLOCK_SIZE_EIGHTS: usize = Self::BLOCK_SIZE / 2;
    const FIVE_BLOCK_SIZE_EIGHTS: usize = 5 * Self::BLOCK_SIZE / 8;
    const SIX_BLOCK_SIZE_EIGHTS: usize = 3 * Self::BLOCK_SIZE / 4;
    const SEVEN_BLOCK_SIZE_EIGHTS: usize = 7 * Self::BLOCK_SIZE / 8;
    fn prefix_sum_of_ones(arr: &mut [i64], base: i64) {
        arr.copy_from(&IDENTITY_PLUS_ONE, 0);
        for v in arr.iter_mut() {
            *v += base;
        }
    }
    fn prefix_sum8(arr: &mut [i64], base: i64) {
        Self::inner_prefix_sum8(arr);
        ForUtil::expand8(arr);

        let l0 = base;
        let l1 = l0 + arr[Self::ONE_BLOCK_SIZE_EIGHT - 1];
        let l2 = l1 + arr[Self::TWO_BLOCK_SIZE_EIGHTS - 1];
        let l3 = l2 + arr[Self::THREE_BLOCK_SIZE_EIGHTS - 1];
        let l4 = l3 + arr[Self::FOUR_BLOCK_SIZE_EIGHTS - 1];
        let l5 = l4 + arr[Self::FIVE_BLOCK_SIZE_EIGHTS - 1];
        let l6 = l5 + arr[Self::SIX_BLOCK_SIZE_EIGHTS - 1];
        let l7 = l6 + arr[Self::SEVEN_BLOCK_SIZE_EIGHTS - 1];

        for i in 0..Self::ONE_BLOCK_SIZE_EIGHT {
            arr[i] += l0;
            arr[Self::ONE_BLOCK_SIZE_EIGHT + i] += l1;
            arr[Self::TWO_BLOCK_SIZE_EIGHTS + i] += l2;
            arr[Self::THREE_BLOCK_SIZE_EIGHTS + i] += l3;
            arr[Self::FOUR_BLOCK_SIZE_EIGHTS + i] += l4;
            arr[Self::FIVE_BLOCK_SIZE_EIGHTS + i] += l5;
            arr[Self::SIX_BLOCK_SIZE_EIGHTS + i] += l6;
            arr[Self::SEVEN_BLOCK_SIZE_EIGHTS + i] += l7;
        }
    }

    fn prefix_sum16(arr: &mut [i64], base: i64) {
        // When the number of bits per value is 11 or less, we can sum up all values in a block without
        // risking overflowing a 16-bits integer. This allows computing the prefix sum by summing up 4
        // values at once.
        Self::inner_prefix_sum16(arr);
        ForUtil::expand16(arr);

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

    fn prefix_sum32(arr: &mut [i64], base: i64) {
        arr[0] += base << 32;
        Self::inner_prefix_sum32(arr);
        ForUtil::expand32(arr);
        let l = arr[Self::BLOCK_SIZE / 2 - 1];
        for elem in &mut arr[Self::BLOCK_SIZE / 2..Self::BLOCK_SIZE] {
            *elem += l;
        }
    }
    // For some reason unrolling seems to help
    fn inner_prefix_sum8(arr: &mut [i64]) {
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
    }
    // For some reason unrolling seems to help
    fn inner_prefix_sum16(arr: &mut [i64]) {
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
    fn inner_prefix_sum32(arr: &mut [i64]) {
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
    /// Encode deltas of a strictly monotonically increasing sequence of integers. The provided {@code
    /// longs} are expected to be deltas between consecutive values.
    pub fn encode_deltas<O: DataOutput>(&mut self, longs: &mut [i64], out: &mut O) -> Result<()> {
        if longs[0] == 1 && PForUtil::all_equal(longs) {
            out.write_byte(0)?;
        } else {
            let mut or = 0;
            for &l in longs.iter() {
                or |= l;
            }
            debug_assert!(or != 0);

            let bits_per_value = PackedInts::bits_required(or)?;
            out.write_byte(bits_per_value as u8)?;

            let primitive_size = if bits_per_value <= 4 {
                ForUtil::collapse8(longs);
                8
            } else if bits_per_value <= 11 {
                ForUtil::collapse16(longs);
                16
            } else {
                ForUtil::collapse32(longs);
                32
            };

            ForUtil::encode_with_tmp(longs, bits_per_value, primitive_size, out, &mut self.tmp)?;
        }

        Ok(())
    }
    /// Decode deltas, compute the prefix sum and add {@code base} to all decoded longs.
    fn decode_and_prefix_sum<I: IndexInput>(
        &mut self,
        pdu: &mut PostingDecodingUtil<I>,
        base: i64,
        longs: &mut [i64],
    ) -> Result<()> {
        let bits_per_value = pdu.input.borrow_mut().read_byte()? as i32;
        if bits_per_value == 0 {
            Self::prefix_sum_of_ones(longs, base);
        } else {
            self.decode_and_prefix_sum_with_bits(bits_per_value, pdu, base, longs)?;
        }
        Ok(())
    }
    /// Delta-decode 128 integers into `longs`.
    fn decode_and_prefix_sum_with_bits<I: IndexInput>(
        &mut self,
        bits_per_value: i32,
        pdu: &mut PostingDecodingUtil<I>,
        base: i64,
        longs: &mut [i64],
    ) -> Result<()> {
        match bits_per_value {
            1 => {
                ForUtil::decode1(pdu, longs)?;
                ForDeltaUtil::prefix_sum8(longs, base);
            }
            2 => {
                ForUtil::decode2(pdu, longs)?;
                ForDeltaUtil::prefix_sum8(longs, base);
            }
            3 => {
                ForUtil::decode3(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum8(longs, base);
            }
            4 => {
                ForUtil::decode4(pdu, longs)?;
                ForDeltaUtil::prefix_sum8(longs, base);
            }
            5 => {
                ForDeltaUtil::decode_5_to_16(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum16(longs, base);
            }
            6 => {
                ForDeltaUtil::decode_6_to_16(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum16(longs, base);
            }
            7 => {
                ForDeltaUtil::decode_7_to_16(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum16(longs, base);
            }
            8 => {
                ForDeltaUtil::decode_8_to_16(pdu, longs)?;
                ForDeltaUtil::prefix_sum16(longs, base);
            }
            9 => {
                ForUtil::decode9(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum16(longs, base);
            }
            10 => {
                ForUtil::decode10(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum16(longs, base);
            }
            11 => {
                ForUtil::decode11(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum16(longs, base);
            }
            12 => {
                ForDeltaUtil::decode_12_to_32(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum32(longs, base);
            }
            13 => {
                ForDeltaUtil::decode_13_to_32(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum32(longs, base);
            }
            14 => {
                ForDeltaUtil::decode_14_to_32(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum32(longs, base);
            }
            15 => {
                ForDeltaUtil::decode_15_to_32(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum32(longs, base);
            }
            16 => {
                ForDeltaUtil::decode_16_to_32(pdu, longs)?;
                ForDeltaUtil::prefix_sum32(longs, base);
            }
            17 => {
                ForUtil::decode17(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum32(longs, base);
            }
            18 => {
                ForUtil::decode18(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum32(longs, base);
            }
            19 => {
                ForUtil::decode19(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum32(longs, base);
            }
            20 => {
                ForUtil::decode20(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum32(longs, base);
            }
            21 => {
                ForUtil::decode21(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum32(longs, base);
            }
            22 => {
                ForUtil::decode22(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum32(longs, base);
            }
            23 => {
                ForUtil::decode23(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum32(longs, base);
            }
            24 => {
                ForUtil::decode24(pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum32(longs, base);
            }
            _ => {
                ForUtil::decode_slow(bits_per_value, pdu, &mut self.tmp, longs)?;
                ForDeltaUtil::prefix_sum32(longs, base);
            }
        }
        Ok(())
    }
    fn decode_5_to_16<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(
            10,
            longs,
            11,
            5,
            ForUtil::MASK16_5,
            tmp,
            0,
            ForUtil::MASK16_1,
        )?;
        let mut tmp_idx = 0;
        let mut longs_idx = 30;
        for _ in 0..2 {
            let mut l0 = tmp[tmp_idx] << 4;
            l0 |= tmp[tmp_idx + 1] << 3;
            l0 |= tmp[tmp_idx + 2] << 2;
            l0 |= tmp[tmp_idx + 3] << 1;
            l0 |= tmp[tmp_idx + 4];
            longs[longs_idx] = l0;
            tmp_idx += 5;
            longs_idx += 1;
        }
        Ok(())
    }

    fn decode_6_to_16<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(
            12,
            longs,
            10,
            6,
            ForUtil::MASK16_6,
            tmp,
            0,
            ForUtil::MASK16_4,
        )?;
        let mut tmp_idx = 0;
        let mut longs_idx = 24;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 2;
            l0 |= ((tmp[tmp_idx + 1] as u64 >> 2) as i64) & ForUtil::MASK16_2;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & ForUtil::MASK16_2) << 4;
            l1 |= tmp[tmp_idx + 2];
            longs[longs_idx + 1] = l1;

            tmp_idx += 3;
            longs_idx += 2;
        }
        Ok(())
    }
    fn decode_7_to_16<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(
            14,
            longs,
            9,
            7,
            ForUtil::MASK16_7,
            tmp,
            0,
            ForUtil::MASK16_2,
        )?;
        let mut tmp_idx = 0;
        let mut longs_idx = 28;
        for _ in 0..2 {
            let mut l0 = tmp[tmp_idx] << 5;
            l0 |= tmp[tmp_idx + 1] << 3;
            l0 |= tmp[tmp_idx + 2] << 1;
            l0 |= ((tmp[tmp_idx + 3] as u64) >> 1) as i64 & ForUtil::MASK16_1;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 3] & ForUtil::MASK16_1) << 6;
            l1 |= tmp[tmp_idx + 4] << 4;
            l1 |= tmp[tmp_idx + 5] << 2;
            l1 |= tmp[tmp_idx + 6];
            longs[longs_idx + 1] = l1;

            tmp_idx += 7;
            longs_idx += 2;
        }
        Ok(())
    }
    fn decode_8_to_16<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_same(16, longs, 8, 8, ForUtil::MASK16_8, 16, ForUtil::MASK16_8)
    }
    fn decode_12_to_32<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(
            24,
            longs,
            20,
            12,
            ForUtil::MASK32_12,
            tmp,
            0,
            ForUtil::MASK32_8,
        )?;
        let mut tmp_idx = 0;
        let mut longs_idx = 48;
        for _ in 0..8 {
            let mut l0 = tmp[tmp_idx] << 4;
            l0 |= ((tmp[tmp_idx + 1] as u64) >> 4) as i64 & ForUtil::MASK32_4;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 1] & ForUtil::MASK32_4) << 8;
            l1 |= tmp[tmp_idx + 2];
            longs[longs_idx + 1] = l1;

            tmp_idx += 3;
            longs_idx += 2;
        }
        Ok(())
    }
    fn decode_13_to_32<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(
            26,
            longs,
            19,
            13,
            ForUtil::MASK32_13,
            tmp,
            0,
            ForUtil::MASK32_6,
        )?;
        let mut tmp_idx = 0;
        let mut longs_idx = 52;
        for _ in 0..2 {
            let mut l0 = tmp[tmp_idx] << 7;
            l0 |= tmp[tmp_idx + 1] << 1;
            l0 |= ((tmp[tmp_idx + 2] as u64) >> 5) as i64 & ForUtil::MASK32_1;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 2] & ForUtil::MASK32_5) << 8;
            l1 |= tmp[tmp_idx + 3] << 2;
            l1 |= ((tmp[tmp_idx + 4] as u64) >> 4) as i64 & ForUtil::MASK32_2;
            longs[longs_idx + 1] = l1;

            let mut l2 = (tmp[tmp_idx + 4] & ForUtil::MASK32_4) << 9;
            l2 |= tmp[tmp_idx + 5] << 3;
            l2 |= ((tmp[tmp_idx + 6] as u64) >> 3) as i64 & ForUtil::MASK32_3;
            longs[longs_idx + 2] = l2;

            let mut l3 = (tmp[tmp_idx + 6] & ForUtil::MASK32_3) << 10;
            l3 |= tmp[tmp_idx + 7] << 4;
            l3 |= ((tmp[tmp_idx + 8] as u64) >> 2) as i64 & ForUtil::MASK32_4;
            longs[longs_idx + 3] = l3;

            let mut l4 = (tmp[tmp_idx + 8] & ForUtil::MASK32_2) << 11;
            l4 |= tmp[tmp_idx + 9] << 5;
            l4 |= ((tmp[tmp_idx + 10] as u64) >> 1) as i64 & ForUtil::MASK32_5;
            longs[longs_idx + 4] = l4;

            let mut l5 = (tmp[tmp_idx + 10] & ForUtil::MASK32_1) << 12;
            l5 |= tmp[tmp_idx + 11] << 6;
            l5 |= tmp[tmp_idx + 12];
            longs[longs_idx + 5] = l5;

            tmp_idx += 13;
            longs_idx += 6;
        }
        Ok(())
    }
    fn decode_14_to_32<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(
            28,
            longs,
            18,
            14,
            ForUtil::MASK32_14,
            tmp,
            0,
            ForUtil::MASK32_4,
        )?;
        let mut tmp_idx = 0;
        let mut longs_idx = 56;
        for _ in 0..4 {
            let mut l0 = tmp[tmp_idx] << 10;
            l0 |= tmp[tmp_idx + 1] << 6;
            l0 |= tmp[tmp_idx + 2] << 2;
            l0 |= ((tmp[tmp_idx + 3] as u64) >> 2) as i64 & ForUtil::MASK32_2;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 3] & ForUtil::MASK32_2) << 12;
            l1 |= tmp[tmp_idx + 4] << 8;
            l1 |= tmp[tmp_idx + 5] << 4;
            l1 |= tmp[tmp_idx + 6];
            longs[longs_idx + 1] = l1;

            tmp_idx += 7;
            longs_idx += 2;
        }
        Ok(())
    }
    fn decode_15_to_32<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        tmp: &mut [i64],
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_diff(
            30,
            longs,
            17,
            15,
            ForUtil::MASK32_15,
            tmp,
            0,
            ForUtil::MASK32_2,
        )?;
        let mut tmp_idx = 0;
        let mut longs_idx = 60;
        for _ in 0..2 {
            let mut l0 = tmp[tmp_idx] << 13;
            l0 |= tmp[tmp_idx + 1] << 11;
            l0 |= tmp[tmp_idx + 2] << 9;
            l0 |= tmp[tmp_idx + 3] << 7;
            l0 |= tmp[tmp_idx + 4] << 5;
            l0 |= tmp[tmp_idx + 5] << 3;
            l0 |= tmp[tmp_idx + 6] << 1;
            l0 |= ((tmp[tmp_idx + 7] as u64) >> 1) as i64 & ForUtil::MASK32_1;
            longs[longs_idx] = l0;

            let mut l1 = (tmp[tmp_idx + 7] & ForUtil::MASK32_1) << 14;
            l1 |= tmp[tmp_idx + 8] << 12;
            l1 |= tmp[tmp_idx + 9] << 10;
            l1 |= tmp[tmp_idx + 10] << 8;
            l1 |= tmp[tmp_idx + 11] << 6;
            l1 |= tmp[tmp_idx + 12] << 4;
            l1 |= tmp[tmp_idx + 13] << 2;
            l1 |= tmp[tmp_idx + 14];
            longs[longs_idx + 1] = l1;

            tmp_idx += 15;
            longs_idx += 2;
        }
        Ok(())
    }
    fn decode_16_to_32<I: IndexInput>(
        pdu: &mut PostingDecodingUtil<I>,
        longs: &mut [i64],
    ) -> Result<()> {
        pdu.split_longs_same(
            32,
            longs,
            16,
            16,
            ForUtil::MASK32_16,
            32,
            ForUtil::MASK32_16,
        )
    }
}
#[cfg(test)]
mod tests {
    use crate::codecs::lucene912::for_delta_util::ForDeltaUtil;
    use crate::codecs::lucene912::for_util::ForUtil;
    use crate::internal::vectorization::posting_decoding_util::PostingDecodingUtil;
    use crate::store::directory::Directory;
    use crate::store::{IOContext, IndexInput, IndexOutput};
    use crate::test::util::lucene_test_case::{new_directory, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::Result;
    use crate::util::packed::PackedInts;
    use rand::Rng;
    use std::cell::RefCell;
    use std::rc::Rc;
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
                let mut source = vec![0i64; ForUtil::BLOCK_SIZE];
                for j in 0..ForUtil::BLOCK_SIZE {
                    source[j] = values[i * ForUtil::BLOCK_SIZE + j] as i64;
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
                let base = 0i64;
                let mut restored = vec![0i64; ForUtil::BLOCK_SIZE];
                for_delta_util.decode_and_prefix_sum(&mut pdu, base, &mut restored)?;

                let mut expected = vec![0i64; ForUtil::BLOCK_SIZE];
                for j in 0..ForUtil::BLOCK_SIZE {
                    expected[j] = values[i * ForUtil::BLOCK_SIZE + j] as i64;
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

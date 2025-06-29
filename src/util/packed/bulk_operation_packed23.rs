/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/
use crate::util::packed::bulk_operation::BulkOperation;
use crate::util::packed::{Decoder, Encoder};

#[derive(Default)]
pub(crate) struct BulkOperationPacked23;
impl Decoder for BulkOperationPacked23 {
    fn decode_u64_to_i64(
        &self,
        blocks: &[u64],
        mut blocks_offset: usize,
        values: &mut [i64],
        mut values_offset: usize,
        iterations: i32,
    ) {
        for _ in 0..iterations {
            let block0 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (block0 >> 41) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 18) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 0x3FFFF) << 5) | (block1 >> 59)) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 36) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 13) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 0x1FFF) << 10) | (block2 >> 54)) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 31) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 8) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 0xFF) << 15) | (block3 >> 49)) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 26) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 3) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 0x7) << 20) | (block4 >> 44)) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 21) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 0x1FFFFF) << 2) | (block5 >> 62)) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 39) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 16) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 0xFFFF) << 7) | (block6 >> 57)) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 34) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 11) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block6 & 0x7FF) << 12) | (block7 >> 52)) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 29) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 6) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block7 & 0x3F) << 17) | (block8 >> 47)) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 24) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 1) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block9 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block8 & 0x1) << 22) | (block9 >> 42)) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 19) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block10 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block9 & 0x7FFFF) << 4) | (block10 >> 60)) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 37) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 14) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block11 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block10 & 0x3FFF) << 9) | (block11 >> 55)) as i64;
            values_offset += 1;
            values[values_offset] = ((block11 >> 32) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block11 >> 9) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block12 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block11 & 0x1FF) << 14) | (block12 >> 50)) as i64;
            values_offset += 1;
            values[values_offset] = ((block12 >> 27) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block12 >> 4) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block13 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block12 & 0xF) << 19) | (block13 >> 45)) as i64;
            values_offset += 1;
            values[values_offset] = ((block13 >> 22) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block14 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block13 & 0x3FFFFF) << 1) | (block14 >> 63)) as i64;
            values_offset += 1;
            values[values_offset] = ((block14 >> 40) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block14 >> 17) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block15 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block14 & 0x1FFFF) << 6) | (block15 >> 58)) as i64;
            values_offset += 1;
            values[values_offset] = ((block15 >> 35) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block15 >> 12) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block16 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block15 & 0xFFF) << 11) | (block16 >> 53)) as i64;
            values_offset += 1;
            values[values_offset] = ((block16 >> 30) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block16 >> 7) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block17 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block16 & 0x7F) << 16) | (block17 >> 48)) as i64;
            values_offset += 1;
            values[values_offset] = ((block17 >> 25) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block17 >> 2) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block18 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block17 & 0x3) << 21) | (block18 >> 43)) as i64;
            values_offset += 1;
            values[values_offset] = ((block18 >> 20) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block19 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block18 & 0xFFFFF) << 3) | (block19 >> 61)) as i64;
            values_offset += 1;
            values[values_offset] = ((block19 >> 38) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block19 >> 15) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block20 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block19 & 0x7FFF) << 8) | (block20 >> 56)) as i64;
            values_offset += 1;
            values[values_offset] = ((block20 >> 33) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block20 >> 10) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block21 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block20 & 0x3FF) << 13) | (block21 >> 51)) as i64;
            values_offset += 1;
            values[values_offset] = ((block21 >> 28) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = ((block21 >> 5) & 0x7FFFFF) as i64;
            values_offset += 1;

            let block22 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block21 & 0x1F) << 18) | (block22 >> 46)) as i64;
            values_offset += 1;
            values[values_offset] = ((block22 >> 23) & 0x7FFFFF) as i64;
            values_offset += 1;
            values[values_offset] = (block22 & 0x7FFFFF) as i64;
            values_offset += 1;
        }
    }
    fn decode_u8_to_i64(
        &self,
        blocks: &[u8],
        mut blocks_offset: usize,
        values: &mut [i64],
        mut values_offset: usize,
        iterations: i32,
    ) {
        for _ in 0..iterations {
            let byte0 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte1 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte2 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = (byte0 << 15) | (byte1 << 7) | (byte2 >> 1);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte5 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] =
                ((byte2 & 1) << 22) | (byte3 << 14) | (byte4 << 6) | (byte5 >> 2);
            values_offset += 1;

            let byte6 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte7 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte8 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] =
                ((byte5 & 3) << 21) | (byte6 << 13) | (byte7 << 5) | (byte8 >> 3);
            values_offset += 1;

            let byte9 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte10 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte11 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] =
                ((byte8 & 7) << 20) | (byte9 << 12) | (byte10 << 4) | (byte11 >> 4);
            values_offset += 1;

            let byte12 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte13 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte14 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] =
                ((byte11 & 15) << 19) | (byte12 << 11) | (byte13 << 3) | (byte14 >> 5);
            values_offset += 1;

            let byte15 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte16 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte17 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] =
                ((byte14 & 31) << 18) | (byte15 << 10) | (byte16 << 2) | (byte17 >> 6);
            values_offset += 1;

            let byte18 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte19 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte20 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] =
                ((byte17 & 63) << 17) | (byte18 << 9) | (byte19 << 1) | (byte20 >> 7);
            values_offset += 1;

            let byte21 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte22 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte20 & 127) << 16) | (byte21 << 8) | byte22;
            values_offset += 1;
        }
    }

    fn decode_u64_to_i32(
        &self,
        blocks: &[u64],
        mut blocks_offset: usize,
        values: &mut [i32],
        mut values_offset: usize,
        iterations: i32,
    ) {
        for _ in 0..iterations {
            let block0 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (block0 >> 41) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 18) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 0x3FFFF) << 5) | (block1 >> 59)) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 36) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 13) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 0x1FFF) << 10) | (block2 >> 54)) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 31) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 8) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 0xFF) << 15) | (block3 >> 49)) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 26) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 3) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 0x7) << 20) | (block4 >> 44)) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 21) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 0x1FFFFF) << 2) | (block5 >> 62)) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 39) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 16) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 0xFFFF) << 7) | (block6 >> 57)) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 34) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 11) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block6 & 0x7FF) << 12) | (block7 >> 52)) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 29) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 6) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block7 & 0x3F) << 17) | (block8 >> 47)) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 24) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 1) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block9 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block8 & 0x1) << 22) | (block9 >> 42)) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 19) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block10 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block9 & 0x7FFFF) << 4) | (block10 >> 60)) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 37) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 14) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block11 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block10 & 0x3FFF) << 9) | (block11 >> 55)) as i32;
            values_offset += 1;
            values[values_offset] = ((block11 >> 32) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block11 >> 9) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block12 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block11 & 0x1FF) << 14) | (block12 >> 50)) as i32;
            values_offset += 1;
            values[values_offset] = ((block12 >> 27) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block12 >> 4) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block13 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block12 & 0xF) << 19) | (block13 >> 45)) as i32;
            values_offset += 1;
            values[values_offset] = ((block13 >> 22) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block14 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block13 & 0x3FFFFF) << 1) | (block14 >> 63)) as i32;
            values_offset += 1;
            values[values_offset] = ((block14 >> 40) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block14 >> 17) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block15 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block14 & 0x1FFFF) << 6) | (block15 >> 58)) as i32;
            values_offset += 1;
            values[values_offset] = ((block15 >> 35) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block15 >> 12) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block16 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block15 & 0xFFF) << 11) | (block16 >> 53)) as i32;
            values_offset += 1;
            values[values_offset] = ((block16 >> 30) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block16 >> 7) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block17 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block16 & 0x7F) << 16) | (block17 >> 48)) as i32;
            values_offset += 1;
            values[values_offset] = ((block17 >> 25) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block17 >> 2) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block18 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block17 & 0x3) << 21) | (block18 >> 43)) as i32;
            values_offset += 1;
            values[values_offset] = ((block18 >> 20) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block19 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block18 & 0xFFFFF) << 3) | (block19 >> 61)) as i32;
            values_offset += 1;
            values[values_offset] = ((block19 >> 38) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block19 >> 15) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block20 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block19 & 0x7FFF) << 8) | (block20 >> 56)) as i32;
            values_offset += 1;
            values[values_offset] = ((block20 >> 33) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block20 >> 10) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block21 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block20 & 0x3FF) << 13) | (block21 >> 51)) as i32;
            values_offset += 1;
            values[values_offset] = ((block21 >> 28) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block21 >> 5) & 0x7FFFFF) as i32;
            values_offset += 1;

            let block22 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block21 & 0x1F) << 18) | (block22 >> 46)) as i32;
            values_offset += 1;
            values[values_offset] = ((block22 >> 23) & 0x7FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = (block22 & 0x7FFFFF) as i32;
            values_offset += 1;
        }
    }

    fn decode_u8_to_i32(
        &self,
        blocks: &[u8],
        mut blocks_offset: usize,
        values: &mut [i32],
        mut values_offset: usize,
        iterations: i32,
    ) {
        for _ in 0..iterations {
            let byte0 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte1 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte2 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = (byte0 << 15) | (byte1 << 7) | (byte2 >> 1);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte5 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] =
                ((byte2 & 1) << 22) | (byte3 << 14) | (byte4 << 6) | (byte5 >> 2);
            values_offset += 1;

            let byte6 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte7 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte8 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] =
                ((byte5 & 3) << 21) | (byte6 << 13) | (byte7 << 5) | (byte8 >> 3);
            values_offset += 1;

            let byte9 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte10 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte11 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] =
                ((byte8 & 7) << 20) | (byte9 << 12) | (byte10 << 4) | (byte11 >> 4);
            values_offset += 1;

            let byte12 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte13 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte14 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] =
                ((byte11 & 15) << 19) | (byte12 << 11) | (byte13 << 3) | (byte14 >> 5);
            values_offset += 1;

            let byte15 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte16 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte17 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] =
                ((byte14 & 31) << 18) | (byte15 << 10) | (byte16 << 2) | (byte17 >> 6);
            values_offset += 1;

            let byte18 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte19 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte20 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] =
                ((byte17 & 63) << 17) | (byte18 << 9) | (byte19 << 1) | (byte20 >> 7);
            values_offset += 1;

            let byte21 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte22 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte20 & 127) << 16) | (byte21 << 8) | byte22;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked23 {}
impl BulkOperation for BulkOperationPacked23 {}

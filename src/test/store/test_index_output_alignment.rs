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
use rand::Rng;

use crate::store::data_output::DataOutput;
use crate::store::{align_offset, IndexOutput, OutputStreamIndexOutput};
use crate::test::util::lucene_test_case::{random, random_multiplier};
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::Result;

#[allow(dead_code)]
struct TestIndexOutputAlignment;

#[test]
fn test_alignment_calculation() -> Result<()> {
    // Test alignment with various sizes
    assert_eq!(align_offset(0, BitUtil::LONG_BYTES as i32)?, 0);
    assert_eq!(align_offset(0, BitUtil::INT_BYTES as i32)?, 0);
    assert_eq!(align_offset(0, BitUtil::SHORT_BYTES as i32)?, 0);
    assert_eq!(align_offset(0, 1)?, 0);

    assert_eq!(align_offset(1, BitUtil::LONG_BYTES as i32)?, 8);
    assert_eq!(align_offset(1, BitUtil::INT_BYTES as i32)?, 4);
    assert_eq!(align_offset(1, BitUtil::SHORT_BYTES as i32)?, 2);
    assert_eq!(align_offset(1, 1)?, 1);

    assert_eq!(align_offset(25, BitUtil::LONG_BYTES as i32)?, 32);
    assert_eq!(align_offset(25, BitUtil::INT_BYTES as i32)?, 28);
    assert_eq!(align_offset(25, BitUtil::SHORT_BYTES as i32)?, 26);
    assert_eq!(align_offset(25, 1)?, 25);

    let val = 1i64 << 48;
    assert_eq!(align_offset(val - 1, BitUtil::LONG_BYTES as i32)?, val);
    assert_eq!(align_offset(val - 1, BitUtil::INT_BYTES as i32)?, val);
    assert_eq!(align_offset(val - 1, BitUtil::SHORT_BYTES as i32)?, val);
    assert_eq!(align_offset(val - 1, 1)?, val - 1);

    assert_eq!(align_offset(i64::MAX, 1)?, i64::MAX);
    Ok(())
}
#[test]
fn test_invalid_alignments() {
    assert_invalid_alignment(0);
    assert_invalid_alignment(6);
    assert_invalid_alignment(43);
}

fn assert_invalid_alignment(size: i32) {
    let result = align_offset(1, size);
    assert!(result.is_err());
}
#[test]
fn test_output_alignment() -> Result<()> {
    let alignments = [
        BitUtil::LONG_BYTES,
        BitUtil::INT_BYTES,
        BitUtil::SHORT_BYTES,
        1usize,
    ];
    for alignment in alignments.iter() {
        run_test_output_alignment(*alignment as i32)?;
    }
    Ok(())
}
pub fn run_test_output_alignment(alignment: i32) -> Result<()> {
    let mut random = random();
    let mut buffer = Vec::new();
    let mut out = OutputStreamIndexOutput::new("test", "test", &mut buffer, 8192)?;

    for _ in 0..(10 * random_multiplier()) {
        let length = random.random_range(0..32);
        let data = vec![0; length];
        out.write_bytes_with_len(&data, length as i32)?;

        let orig_pos = out.get_file_pointer();
        // align to next boundary
        let new_pos = out.align_file_pointer(alignment)?;

        assert_eq!(out.get_file_pointer(), new_pos);
        assert_eq!(new_pos % alignment as i64, 0, "not aligned");
        assert!(new_pos >= orig_pos, "newPos >= origPos");
        assert!(
            new_pos - orig_pos < alignment as i64,
            "Too much added: newPos - origPos = {}",
            new_pos - orig_pos
        );
    }
    Ok(())
}

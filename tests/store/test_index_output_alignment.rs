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
use crate::common::get_random_multiplier;
use crate::util::lucene_test_case::random;
use crate::util::test_error::TestError;
use rand::Rng;
use rlucene::store::data_output::DataOutput;
use rlucene::store::{align_offset, IndexOutput, OutputStreamIndexOutput};
use rlucene::util::bit_util::BitUtil;
use rlucene::util::error::lucene_error::LuceneError;

#[allow(dead_code)]
struct TestIndexOutputAlignment;

#[test]
fn test_alignment_calculation() -> Result<(), TestError> {
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
fn test_output_alignment() -> Result<(), LuceneError> {
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
pub fn run_test_output_alignment(alignment: i32) -> Result<(), LuceneError> {
    let mut random = random();
    let mut buffer = Vec::new();
    let mut out = OutputStreamIndexOutput::new("test", "test", &mut buffer, 8192)?;

    for _ in 0..(10 * get_random_multiplier()) {
        let length = random.gen_range(0..32);
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

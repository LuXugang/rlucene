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
use crate::common::{get_random_multiplier, my_random};
use crate::util::test_error::TestError;
use rand::Rng;
use rlucene::store::data_output::DataOutput;
use rlucene::store::{align_offset, IndexOutput, OutputStreamIndexOutput};
use rlucene::util::bit_util::{INT_BYTES, LONG_BYTES, SHORT_BYTES};
use rlucene::util::error::data_io_error_enum::DataIOError;

#[allow(dead_code)]
struct TestIndexOutputAlignment;

#[test]
fn test_alignment_calculation() -> Result<(), TestError> {
    // Test alignment with various sizes
    assert_eq!(align_offset(0, LONG_BYTES as u32)?, 0);
    assert_eq!(align_offset(0, INT_BYTES as u32)?, 0);
    assert_eq!(align_offset(0, SHORT_BYTES as u32)?, 0);
    assert_eq!(align_offset(0, 1)?, 0);

    assert_eq!(align_offset(1, LONG_BYTES as u32)?, 8);
    assert_eq!(align_offset(1, INT_BYTES as u32)?, 4);
    assert_eq!(align_offset(1, SHORT_BYTES as u32)?, 2);
    assert_eq!(align_offset(1, 1)?, 1);

    assert_eq!(align_offset(25, LONG_BYTES as u32)?, 32);
    assert_eq!(align_offset(25, INT_BYTES as u32)?, 28);
    assert_eq!(align_offset(25, SHORT_BYTES as u32)?, 26);
    assert_eq!(align_offset(25, 1)?, 25);

    let val = 1u64 << 48;
    assert_eq!(align_offset(val - 1, LONG_BYTES as u32)?, val);
    assert_eq!(align_offset(val - 1, INT_BYTES as u32)?, val);
    assert_eq!(align_offset(val - 1, SHORT_BYTES as u32)?, val);
    assert_eq!(align_offset(val - 1, 1)?, val - 1);

    assert_eq!(align_offset(u64::MAX, 1)?, u64::MAX);
    Ok(())
}
#[test]
fn test_invalid_alignments() {
    assert_invalid_alignment(0);
    assert_invalid_alignment(6);
    assert_invalid_alignment(43);
}

fn assert_invalid_alignment(size: u32) {
    let result = align_offset(1, size);
    assert!(result.is_err());
}
#[test]
fn test_output_alignment() -> Result<(), DataIOError> {
    let alignments = [LONG_BYTES, INT_BYTES, SHORT_BYTES, 1usize];
    for alignment in alignments.iter() {
        run_test_output_alignment(*alignment as u32)?;
    }
    Ok(())
}
pub fn run_test_output_alignment(alignment: u32) -> Result<(), DataIOError> {
    let mut random = my_random("test_output_alignment".to_string());
    let mut buffer = Vec::new();
    let mut out = OutputStreamIndexOutput::new("test", "test", &mut buffer, 8192)?;

    for _ in 0..(10 * get_random_multiplier()) {
        let length: usize = random.gen_range(0..32);
        let data = vec![0; length];
        out.write_bytes_with_len(&data, length)?;

        let orig_pos = out.get_file_pointer();
        // align to next boundary
        let new_pos = out.align_file_pointer(alignment)?;

        assert_eq!(out.get_file_pointer(), new_pos);
        assert_eq!(new_pos % alignment as u64, 0, "not aligned");
        assert!(new_pos >= orig_pos, "newPos >= origPos");
        assert!(
            new_pos - orig_pos < alignment as u64,
            "Too much added: newPos - origPos = {}",
            new_pos - orig_pos
        );
    }
    Ok(())
}

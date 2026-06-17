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
use rand::RngExt;

use crate::core::codecs::lucene101::for_delta_util::ForDeltaUtil;
use crate::core::codecs::lucene101::for_util::ForUtil;
use crate::core::internal::vectorization::posting_decoding_util::PostingDecodingUtil;
use crate::core::store::directory::Directory;
use crate::core::store::{ByteBuffersDirectory, IOContext, IndexInput, IndexOutput};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::packed::PackedInts;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use crate::test::core::util::test_util::TestUtil;
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

  let d = ByteBuffersDirectory::new();
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
    let input = d.open_input("test.bin", &IOContext::read_once_io_context()?)?;
    // TODO: VECTORIZATION_PROVIDER not implement
    let mut pdu = PostingDecodingUtil::new(input);
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

    assert_eq!(end_pointer, pdu.input.get_file_pointer()?);
  }
  Ok(())
}

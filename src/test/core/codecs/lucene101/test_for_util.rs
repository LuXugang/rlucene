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
use crate::core::codecs::lucene101::for_util::ForUtil;
use crate::core::internal::vectorization::posting_decoding_util::PostingDecodingUtil;
use crate::core::store::directory::Directory;
use crate::core::store::{
  ByteBuffersDirectory, DataInput, DataOutput, IOContext, IndexInput, IndexOutput,
};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::packed::PackedInts;
use crate::test_framework::core::util::lucene_test_case::random;
use crate::test_framework::core::util::test_util::TestUtil;
use rand::RngExt;

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

  let dir = ByteBuffersDirectory::new();
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

    end_pointer = out.get_file_pointer()?;
  }

  {
    // decode
    let input = dir.open_input("test.bin", &IOContext::read_once_io_context()?)?;
    let mut pdu = PostingDecodingUtil::new(input);
    let mut for_util = ForUtil::new();

    for i in 0..iterations {
      let bits_per_value = pdu.input.read_byte()? as i32;
      let current_fp = pdu.input.get_file_pointer()?;
      let mut restored = vec![0i32; ForUtil::BLOCK_SIZE];

      for_util.decode(bits_per_value, &mut pdu, &mut restored)?;

      let expected = &values[i * ForUtil::BLOCK_SIZE..(i + 1) * ForUtil::BLOCK_SIZE];
      assert_eq!(restored, expected, "Mismatch at iteration {}", i);

      let expected_bytes = ForUtil::num_bytes(bits_per_value);
      let actual_bytes = pdu.input.get_file_pointer()? - current_fp;
      assert_eq!(
        expected_bytes as usize, actual_bytes,
        "Unexpected byte count at iteration {}",
        i
      );
    }

    assert_eq!(end_pointer, pdu.input.get_file_pointer()?);
  }

  Ok(())
}

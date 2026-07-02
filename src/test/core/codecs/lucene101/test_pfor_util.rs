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

use crate::test_framework::core::util::lucene_test_case::random;
use rand::Rng;
use rand::RngExt;

use crate::core::codecs::lucene101::for_util::ForUtil;
use crate::core::codecs::lucene101::pfor_util::PForUtil;
use crate::core::internal::vectorization::posting_decoding_util::PostingDecodingUtil;
use crate::core::store::directory::Directory;
use crate::core::store::{ByteBuffersDirectory, IOContext, IndexInput, IndexOutput};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::packed::PackedInts;
use crate::test_framework::core::util::test_util::TestUtil;
#[allow(dead_code)] // for quick search
struct TestPForUtil;

#[test]
fn test_encode_decode() -> Result<()> {
  let mut random = random();
  let iterations = random.random_range(50..1000);
  let values = create_test_data(iterations, 31, &mut random);

  let dir = ByteBuffersDirectory::new();
  let end_pointer = encode_test_data(iterations, &values, &dir)?;

  let input = dir.open_input("test.bin", &IOContext::read_once_io_context()?)?;
  let mut pdu = PostingDecodingUtil::new(input);
  let mut pfor_util = PForUtil::new();

  for i in 0..iterations {
    {
      if random.random_range(0..5) == 0 {
        PForUtil::skip(&mut pdu.input)?;
        continue;
      }
    }
    let mut restored = vec![0i32; ForUtil::BLOCK_SIZE];
    pfor_util.decode(&mut pdu, &mut restored)?;

    let expected = &values[i * ForUtil::BLOCK_SIZE..(i + 1) * ForUtil::BLOCK_SIZE];
    assert_eq!(restored, expected, "Mismatch at iteration {}", i);
  }

  assert_eq!(end_pointer, pdu.input.get_file_pointer()?);
  Ok(())
}
fn create_test_data<R>(iterations: usize, max_bpv: i32, random: &mut R) -> Vec<i32>
where
  R: Rng + ?Sized,
{
  assert!(max_bpv > 0 && max_bpv <= 31);
  let mut values = vec![0i32; iterations * ForUtil::BLOCK_SIZE];
  for i in 0..iterations {
    let bpv = TestUtil::next_int(random, 0, max_bpv);
    for j in 0..ForUtil::BLOCK_SIZE {
      let idx = i * ForUtil::BLOCK_SIZE + j;
      values[idx] = random.random_range(0..=PackedInts::max_value(bpv) as i32);
      if random.random_range(0..100) == 0 {
        let extra = if random.random_range(0..10) == 0 {
          TestUtil::next_int(random, 9, 16)
        } else {
          TestUtil::next_int(random, 1, 8)
        };
        let exception_bpv = (bpv + extra).min(max_bpv);
        values[idx] |= random.random_range(0..(1 << (exception_bpv - bpv))) << bpv;
      }
    }
  }
  values
}
fn encode_test_data(iterations: usize, values: &[i32], dir: &impl Directory) -> Result<usize> {
  let mut out = dir.create_output("test.bin", &IOContext::default_io_context()?)?;
  let mut pfor_util = PForUtil::new();

  for i in 0..iterations {
    let mut source = [0i32; ForUtil::BLOCK_SIZE];
    for j in 0..ForUtil::BLOCK_SIZE {
      source[j] = values[i * ForUtil::BLOCK_SIZE + j];
    }
    pfor_util.encode(&mut source, &mut out)?;
  }

  let end_pointer = out.get_file_pointer()?;
  Ok(end_pointer)
}

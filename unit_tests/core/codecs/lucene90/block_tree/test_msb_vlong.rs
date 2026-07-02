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
use crate::core::codecs::block_tree::field_reader::read_msb_vlong;
use crate::core::codecs::block_tree::lucene90_block_tree_terms_writer::write_msb_vlong;
use crate::core::store::{ByteArrayDataInput, ByteArrayDataOutput};
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::lucene_test_case::{at_least, random};
#[allow(dead_code)] // for quick search
struct TestMSBVLong;

#[test]
fn test_msb_vlong() -> Result<()> {
  assert_msb_vlong(i64::MAX)?;
  let mut random = random();
  let iter = at_least(&mut random, 10000) as i64;
  for i in 0..iter {
    assert_msb_vlong(i)?;
  }
  Ok(())
}

fn assert_msb_vlong(l: i64) -> Result<()> {
  let buffer = vec![0u8; 10];
  let mut output = ByteArrayDataOutput::with_bytes(buffer);
  write_msb_vlong(&mut output, l)?;
  let len = output.get_position();
  let mut input = ByteArrayDataInput::with_range(output.bytes.as_slice(), 0, len);
  let recovered = read_msb_vlong(&mut input)?;
  assert_eq!(
    recovered, l,
    "Mismatch in MSB VLong roundtrip: {} != {}",
    l, recovered
  );

  Ok(())
}

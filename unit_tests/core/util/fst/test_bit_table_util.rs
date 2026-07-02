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
use crate::test_framework::core::util::lucene_test_case::{at_least, random};
use std::fmt::{Display, Formatter};

use rand::Rng;
use rand::RngExt;

use crate::core::store::DataInput;

use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fst_impl::bit_table_util::BitTableUtil;
use crate::core::util::fst_impl::fst::BytesReader;
use crate::core::util::group_vint_util::GroupVIntUtil;
#[allow(dead_code)] // for quick search
struct TestBitTableUtil;
#[test]
fn test_next_bit_set() -> Result<()> {
  let mut random = random();
  let num_iterations = at_least(&mut random, 1000);

  for i in 0..num_iterations {
    let bits = build_random_bits(&mut random);
    assert!(bits.len() <= i32::MAX as usize);
    let num_bytes = (bits.len() - 1) as i32;
    let num_bits = num_bytes * i8::BITS as i32;

    // Verify next_bit_set with count_bits_upto for all bit indexes.
    for bit_index in -1..num_bits {
      let next_index = BitTableUtil::next_bit_set(bit_index, num_bytes, &mut reader(&bits))?;

      if next_index == -1 {
        assert_eq!(
          BitTableUtil::count_bits_upto(bit_index + 1, &mut reader(&bits))?,
          BitTableUtil::count_bits(num_bytes, &mut reader(&bits))?,
          "No next bit set, so expected no bit count diff (i={} bitIndex={})",
          i,
          bit_index
        );
      } else {
        assert!(
          BitTableUtil::is_bit_set(next_index, &mut reader(&bits))?,
          "Expected next bit set at next_index={} (i={} bitIndex={})",
          next_index,
          i,
          bit_index
        );

        assert_eq!(
          BitTableUtil::count_bits_upto(bit_index + 1, &mut reader(&bits))? + 1,
          BitTableUtil::count_bits_upto(next_index + 1, &mut reader(&bits))?,
          "Next bit set at next_index={} so expected bit count diff of 1 (i={} bitIndex={})",
          next_index,
          i,
          bit_index
        );
      }
    }
  }

  Ok(())
}
#[test]
fn test_previous_bit_set() -> Result<()> {
  let mut random = random();
  let num_iterations = at_least(&mut random, 1000);

  for i in 0..num_iterations {
    let bits = build_random_bits(&mut random);
    assert!(bits.len() <= i32::MAX as usize);
    let num_bytes = (bits.len() - 1) as i32;
    let num_bits = num_bytes * i8::BITS as i32;

    // Verify previous_bit_set with count_bits_upto for all bit
    // indexes.
    for bit_index in 0..=num_bits {
      let previous_index = BitTableUtil::previous_bit_set(bit_index, &mut reader(&bits))?;

      if previous_index == -1 {
        assert_eq!(
          0,
          BitTableUtil::count_bits_upto(bit_index, &mut reader(&bits))?,
          "No previous bit set, so expected bit count 0 (i={} bitIndex={})",
          i,
          bit_index
        );
      } else {
        assert!(
          BitTableUtil::is_bit_set(previous_index, &mut reader(&bits))?,
          "Expected previous bit set at previous_index={} (i={} bitIndex={})",
          previous_index,
          i,
          bit_index
        );

        let bit_count = BitTableUtil::count_bits_upto(
          bit_index.saturating_add(1).min(num_bits),
          &mut reader(&bits),
        )?;
        let expected_previous_bit_count =
          if bit_index < num_bits && BitTableUtil::is_bit_set(bit_index, &mut reader(&bits))? {
            bit_count - 1
          } else {
            bit_count
          };

        assert_eq!(
          expected_previous_bit_count,
          BitTableUtil::count_bits_upto(previous_index + 1, &mut reader(&bits))?,
          "Previous bit set at previous_index={} with current bitCount={} so expected previousBitCount={} (i={} bitIndex={})",
          previous_index,
          bit_count,
          expected_previous_bit_count,
          i,
          bit_index
        );
      }
    }
  }

  Ok(())
}

fn build_random_bits<R>(random: &mut R) -> Vec<u8>
where
  R: Rng + ?Sized,
{
  let len = random.random_range(2..26);
  let mut bits = vec![0; len];

  for byte in &mut bits {
    // Bias towards zeros which require special logic.
    *byte = if random.random_range(0..4) == 0 {
      0
    } else {
      random.random()
    };
  }

  bits
}

/// Creates a `BytesReader` for the given byte slice.
fn reader(bits: &[u8]) -> BytesReaderImpl<'_> {
  BytesReaderImpl::new(bits)
}

struct BytesReaderImpl<'a> {
  bits: &'a [u8],
  position: i64,
}
impl<'a> BytesReaderImpl<'a> {
  fn new(bits: &'a [u8]) -> Self {
    Self {
      bits,
      position: 0i64,
    }
  }
}

impl DataInput for BytesReaderImpl<'_> {
  fn read_byte(&mut self) -> Result<u8> {
    let v = self.bits[self.position as usize];
    self.position += 1;
    Ok(v)
  }

  fn read_bytes(&mut self, _b: &mut [u8], _offset: usize, _len: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation("not implement"))
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    GroupVIntUtil::read_group_vint_i32(self, dst, offset)
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    if num_bytes >= 0 {
      self.position += num_bytes;
    } else {
      self.position -= -num_bytes;
    }
    Ok(())
  }
}

impl crate::core::util::close::Closeable for BytesReaderImpl<'_> {}

impl Display for BytesReaderImpl<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl BytesReader for BytesReaderImpl<'_> {
  fn get_position(&self) -> i64 {
    self.position
  }
  fn set_position(&mut self, pos: i64) {
    self.position = pos;
  }
}

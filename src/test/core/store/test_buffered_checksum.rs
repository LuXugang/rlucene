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
use crc32fast::Hasher;
use rand::{Rng, RngExt};

use crate::core::store::{BufferedChecksum, Checksum, HasherChecksum};
use crate::core::util::bit_util::BitUtil;
use crate::test_framework::core::util::lucene_test_case::{at_least_usize, random};

#[allow(dead_code)] // for quick search
struct TestBufferedChecksum;
#[test]
fn test_simple() {
  let mut buffered = BufferedChecksum::new(HasherChecksum::new(Hasher::new()));
  buffered.update(1);
  buffered.update(2);
  buffered.update(3);

  assert_eq!(1438416925, buffered.get_value());
}

#[test]
fn test_random() {
  let mut raw_crc = Hasher::new();
  let mut buffered = BufferedChecksum::new(HasherChecksum::new(Hasher::new()));

  let mut rng = random();
  let iterations = at_least_usize(&mut rng, 10000);

  for _ in 0..iterations {
    match rng.random_range(0..4) {
      0 => {
        let length = rng.random_range(0..1024);
        let mut bytes = vec![0; length];
        rng.fill(bytes.as_mut_slice());
        raw_crc.update(&bytes);
        buffered.update_bytes(&bytes, 0, length);
      },
      1 => {
        let b = rng.random_range(0..=255) as u8;
        raw_crc.update(&[b]);
        buffered.update(b);
      },
      2 => {
        raw_crc = Hasher::new();
        buffered.reset();
      },
      3 => {
        assert_eq!(buffered.get_value(), raw_crc.clone().finalize() as i64);
      },
      _ => unreachable!(),
    }
  }

  assert_eq!(buffered.get_value(), raw_crc.finalize() as i64);
}

#[test]
fn test_different_input_types() {
  let mut rng = random();
  let iterations = at_least_usize(&mut rng, 1000);

  for _ in 0..iterations {
    let mut input = [0_u8; 4096];
    rng.fill(&mut input);

    let mut crc = Hasher::new();
    crc.update(&input);
    let checksum = crc.finalize() as i64;

    let mut buffered = BufferedChecksum::new(HasherChecksum::new(Hasher::new()));
    update_by_shorts(checksum, &mut buffered, &input, &mut rng);
    update_by_ints(checksum, &mut buffered, &input, &mut rng);
    update_by_longs(checksum, &mut buffered, &input, &mut rng);
    update_by_chunk_of_bytes(checksum, &mut buffered, &input, &mut rng);
    update_by_chunk_of_longs(checksum, &mut buffered, &input, &mut rng);
  }
}

fn update_by_chunk_of_bytes<R: Rng + ?Sized>(
  expected: i64,
  checksum: &mut BufferedChecksum<HasherChecksum>,
  input: &[u8],
  rng: &mut R,
) {
  for &b in input {
    checksum.update(b);
  }
  check_checksum_value_and_reset(expected, checksum);

  checksum.update_bytes(input, 0, input.len());
  check_checksum_value_and_reset(expected, checksum);

  let iterations = at_least_usize(rng, 10);
  for _ in 0..iterations {
    let len0 = rng.random_range(0..input.len() / 2);
    checksum.update_bytes(input, 0, len0);
    checksum.update_bytes(input, len0, input.len() - len0);
    check_checksum_value_and_reset(expected, checksum);

    checksum.update_bytes(input, 0, len0);
    let len1 = rng.random_range(0..input.len() / 4);
    for &b in &input[len0..len0 + len1] {
      checksum.update(b);
    }
    checksum.update_bytes(input, len0 + len1, input.len() - len0 - len1);
    check_checksum_value_and_reset(expected, checksum);
  }
}

fn update_by_shorts<R: Rng + ?Sized>(
  expected: i64,
  checksum: &mut BufferedChecksum<HasherChecksum>,
  input: &[u8],
  rng: &mut R,
) {
  let mut ix = shift_array(checksum, input, rng);
  while ix + BitUtil::SHORT_BYTES <= input.len() {
    let value = BitUtil::get_i16_le(input, ix);
    checksum.update_short(value);
    ix += BitUtil::SHORT_BYTES;
  }
  checksum.update_bytes(input, ix, input.len() - ix);
  check_checksum_value_and_reset(expected, checksum);
}

fn update_by_ints<R: Rng + ?Sized>(
  expected: i64,
  checksum: &mut BufferedChecksum<HasherChecksum>,
  input: &[u8],
  rng: &mut R,
) {
  let mut ix = shift_array(checksum, input, rng);
  while ix + BitUtil::INT_BYTES <= input.len() {
    let value = BitUtil::get_i32_le(input, ix);
    checksum.update_int(value);
    ix += BitUtil::INT_BYTES;
  }
  checksum.update_bytes(input, ix, input.len() - ix);
  check_checksum_value_and_reset(expected, checksum);
}

fn update_by_longs<R: Rng + ?Sized>(
  expected: i64,
  checksum: &mut BufferedChecksum<HasherChecksum>,
  input: &[u8],
  rng: &mut R,
) {
  let mut ix = shift_array(checksum, input, rng);
  while ix + BitUtil::LONG_BYTES <= input.len() {
    let value = BitUtil::get_i64_le(input, ix);
    checksum.update_long(value);
    ix += BitUtil::LONG_BYTES;
  }
  checksum.update_bytes(input, ix, input.len() - ix);
  check_checksum_value_and_reset(expected, checksum);
}

fn shift_array<R: Rng + ?Sized>(
  checksum: &mut BufferedChecksum<HasherChecksum>,
  input: &[u8],
  rng: &mut R,
) -> usize {
  let ix = rng.random_range(0..input.len() / 4);
  checksum.update_bytes(input, 0, ix);
  ix
}

fn update_by_chunk_of_longs<R: Rng + ?Sized>(
  expected: i64,
  checksum: &mut BufferedChecksum<HasherChecksum>,
  input: &[u8],
  rng: &mut R,
) {
  let ix = rng.random_range(0..input.len() / 4);
  let remaining = (input.len() - ix) % BitUtil::LONG_BYTES;
  let long_end = input.len() - remaining;
  let long_input: Vec<i64> = input[ix..long_end]
    .as_chunks::<{ BitUtil::LONG_BYTES }>()
    .0
    .iter()
    .map(|chunk| i64::from_le_bytes(*chunk))
    .collect();

  checksum.update_bytes(input, 0, ix);
  for value in &long_input {
    checksum.update_long(*value);
  }
  checksum.update_bytes(input, long_end, remaining);
  check_checksum_value_and_reset(expected, checksum);

  checksum.update_bytes(input, 0, ix);
  checksum.update_longs(&long_input, 0, long_input.len());
  checksum.update_bytes(input, long_end, remaining);
  check_checksum_value_and_reset(expected, checksum);

  let iterations = at_least_usize(rng, 10);
  for _ in 0..iterations {
    let len0 = rng.random_range(0..long_input.len() / 2);
    checksum.update_bytes(input, 0, ix);
    checksum.update_longs(&long_input, 0, len0);
    checksum.update_longs(&long_input, len0, long_input.len() - len0);
    checksum.update_bytes(input, long_end, remaining);
    check_checksum_value_and_reset(expected, checksum);

    checksum.update_bytes(input, 0, ix);
    checksum.update_longs(&long_input, 0, len0);
    let len1 = rng.random_range(0..long_input.len() / 4);
    for value in &long_input[len0..len0 + len1] {
      checksum.update_long(*value);
    }
    checksum.update_longs(&long_input, len0 + len1, long_input.len() - len0 - len1);
    checksum.update_bytes(input, long_end, remaining);
    check_checksum_value_and_reset(expected, checksum);

    checksum.update_bytes(input, 0, ix);
    checksum.update_longs(&long_input, 0, len0);
    checksum.update_bytes(
      input,
      ix + len0 * BitUtil::LONG_BYTES,
      input.len() - ix - len0 * BitUtil::LONG_BYTES,
    );
    check_checksum_value_and_reset(expected, checksum);
  }
}

fn check_checksum_value_and_reset(expected: i64, checksum: &mut impl Checksum) {
  assert_eq!(expected, checksum.get_value());
  checksum.reset();
}

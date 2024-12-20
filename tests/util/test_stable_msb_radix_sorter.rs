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
use crate::common::{assert_vecs_equal, my_random};
use crate::util::test_error::TestError;
use crate::util::TestUtil;
use rand::rngs::StdRng;
use rand::{Rng, RngCore};
use rlucene::index::{BytesRef, BytesRefBuilder};
use rlucene::util::error::runtime_error::RuntimeError;
use rlucene::util::stable_msb_radix_sorter::{StableMSBRadixSorter, StableMSBRadixSorterBase};
use rlucene::util::{default_build_histogram, default_get_fallback_sorter_stable, default_get_get_bucket, default_reorder, default_should_fallback, MSBRadixSorter, MSBRadixSorterBase, Sorter};
use std::collections::HashSet;

#[allow(dead_code)] // for quick search
struct TestStableMSBRadixSorter;

fn test(refs: &[BytesRef], len: usize, random: &mut StdRng) -> Result<(), TestError> {
    let mut expected: Vec<BytesRef> = refs[..len].to_vec();
    expected.sort();

    let mut max_length = 0;
    for ref_item in &refs[..len] {
        max_length = max_length.max(ref_item.length as i32);
    }

    match random.gen_range(0..3) {
        0 => max_length += random.gen_range(1..=5),
        1 => max_length = i32::MAX,
        _ => {}
    }

    let final_max_length = max_length;
    let mut actual = refs[..len].to_vec();
    let delegate_sorter = StableMSBRadixSorterImpl::new(final_max_length, &mut actual);
    let stable_msb_radix_sorter = StableMSBRadixSorter::new(delegate_sorter);
    let mut msb_radix_sorter = MSBRadixSorter::new(max_length, stable_msb_radix_sorter);
    msb_radix_sorter.sort(0, len as i32)?;

    assert_vecs_equal(&expected, &actual);
    Ok(())
}
#[test]
fn test_empty() -> Result<(), TestError> {
    let mut random = my_random("test_empty".to_string());
    let refs: Vec<BytesRef> = vec![BytesRef::default(); random.gen_range(0..5)];
    test(&refs, 0, &mut random)
}
#[test]
fn test_one_value() -> Result<(), TestError> {
    let mut random = my_random("test_one_value".to_string());
    let bytes = BytesRef::new_from_string(&TestUtil::random_simple_string(&mut random));
    let refs = vec![bytes];
    test(&refs, 1, &mut random)
}

#[test]
fn test_two_values() -> Result<(), TestError> {
    let mut random = my_random("test_two_values".to_string());
    let bytes1 = BytesRef::new_from_string(&TestUtil::random_simple_string(&mut random));
    let bytes2 = BytesRef::new_from_string(&TestUtil::random_simple_string(&mut random));
    let refs = vec![bytes1, bytes2];
    test(&refs, 2, &mut random)
}

fn test_random_impl(
    common_prefix_len: usize,
    max_len: usize,
    random: &mut StdRng,
) -> Result<(), TestError> {
    let mut common_prefix = vec![0u8; common_prefix_len];
    random.fill_bytes(&mut common_prefix);
    let len = random.gen_range(0..100_000);
    let mut bytes: Vec<BytesRef> = Vec::with_capacity(len + random.gen_range(0..50));
    for _ in 0..len {
        let mut b = vec![0u8; common_prefix_len + random.gen_range(0..max_len)];
        random.fill_bytes(&mut b[common_prefix_len..]);
        b[..common_prefix_len].copy_from_slice(&common_prefix);
        bytes.push(BytesRef::new_from_bytes(b));
    }
    test(&bytes, len, random)
}

#[test]
fn test_random() -> Result<(), TestError> {
    let mut random = my_random("test_random_iterations".to_string());
    for _ in 0..10 {
        test_random_impl(0, 10, &mut random)?;
    }
    Ok(())
}

#[test]
fn test_random_with_lots_of_duplicates() -> Result<(), TestError> {
    let mut random = my_random("test_random_with_lots_of_duplicates".to_string());
    for _ in 0..10 {
        test_random_impl(0, 2, &mut random)?;
    }
    Ok(())
}

#[test]
fn test_random_with_shared_prefix() -> Result<(), TestError> {
    let mut random = my_random("test_random_with_shared_prefix".to_string());
    for _ in 0..10 {
        let common_prefix_len = random.gen_range(1..30);
        test_random_impl(common_prefix_len, 10, &mut random)?;
    }
    Ok(())
}

#[test]
fn test_random_with_shared_prefix_and_lots_of_duplicates() -> Result<(), TestError> {
    let mut random = my_random("test_random_with_shared_prefix_and_lots_of_duplicates".to_string());
    for _ in 0..10 {
        let common_prefix_len = random.gen_range(1..30);
        test_random_impl(common_prefix_len, 2, &mut random)?;
    }
    Ok(())
}

#[test]
fn test_random2() -> Result<(), TestError> {
    let mut random = my_random("test_random2".to_string());
    // how large our alphabet is
    let letter_count = random.gen_range(2..=10);

    // how many substring fragments to use
    let substring_count = random.gen_range(2..=10);
    let mut substrings_set = HashSet::new();

    // how many strings to make
    let string_count = random.gen_range(10000..100000);

    // Generate substring fragments
    while substrings_set.len() < substring_count {
        let length = random.gen_range(2..=10);
        let mut bytes = vec![0u8; length];
        for byte in &mut bytes {
            *byte = random.gen_range(0..letter_count) as u8;
        }
        substrings_set.insert(BytesRef::new_from_bytes(bytes));
    }

    let substrings: Vec<BytesRef> = substrings_set.into_iter().collect();
    let mut chance: Vec<f64> = Vec::with_capacity(substrings.len());
    let mut sum = 0.0;

    // Generate random chances
    for _ in &substrings {
        let value = random.gen::<f64>();
        chance.push(value);
        sum += value;
    }

    // give each substring a random chance of occurring:
    let mut accum = 0.0;
    for value in &mut chance {
        accum += *value / sum;
        *value = accum;
    }

    let mut strings_set = HashSet::new();
    let mut iters = 0;

    // Generate strings
    while strings_set.len() < string_count && iters < string_count * 5 {
        let count = random.gen_range(1..=5);
        let mut builder = BytesRefBuilder::new();

        for _ in 0..count {
            let v = random.gen::<f64>();
            let mut accum = 0.0;
            for (j, substring) in substrings.iter().enumerate() {
                accum += chance[j];
                if accum >= v {
                    builder.append_ref(substring);
                    break;
                }
            }
        }

        let br = builder.to_bytes_ref();
        strings_set.insert(br);
        iters += 1;
    }

    let strings_vec: Vec<BytesRef> = strings_set.into_iter().collect();
    test(&strings_vec, strings_vec.len(), &mut random)
}

struct StableMSBRadixSorterImpl<'a> {
    temp: Vec<BytesRef>,
    final_max_length: i32,
    refs: &'a mut [BytesRef],
}
impl<'a> StableMSBRadixSorterImpl<'a> {
    fn new(final_max_length: i32, refs: &'a mut Vec<BytesRef>) -> Self {
        StableMSBRadixSorterImpl {
            temp: vec![BytesRef::default(); refs.len()],
            final_max_length,
            refs,
        }
    }
}

impl Sorter for StableMSBRadixSorterImpl<'_> {
    fn compare(&mut self, i: i32, j: i32) -> i32 {
        unreachable!()
    }

    fn swap(&mut self, i: i32, j: i32) {
        self.refs.swap(i as usize, j as usize);
    }

    fn set_pivot(&mut self, i: i32) {
        unreachable!()
    }

    fn compare_pivot(&mut self, i: i32) -> i32 {
        unreachable!()
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<(), RuntimeError> {
        unreachable!()
    }
}

impl<'a> MSBRadixSorterBase for StableMSBRadixSorterImpl<'a> {
    fn byte_at(&mut self, i: i32, k: i32) -> i32 {
        assert!(k < self.final_max_length, "k is out of bounds");
        let ref_item = &self.refs[i as usize];

        if ref_item.length <= k as u32 {
            return -1;
        }

        ref_item.bytes[ref_item.offset as usize + k as usize] as i32
    }

    fn get_fallback_sorter(&mut self, k: i32) -> impl Sorter {
        default_get_fallback_sorter_stable(self.final_max_length, self, k)
    }

    fn reorder(
        &mut self,
        from: i32,
        to: i32,
        start_offsets: &mut [i32],
        end_offsets: &mut [i32],
        k: i32,
    ) {
        default_reorder(self, from, to, start_offsets, end_offsets, k)
    }

    fn get_bucket(&mut self, i: i32, k: i32) -> i32 {
        default_get_get_bucket(self, i, k)
    }

    fn build_histogram(
        &mut self,
        prefix_common_bucket: i32,
        prefix_common_len: i32,
        from: i32,
        to: i32,
        k: i32,
        histogram: &mut [i32],
    ) {
        default_build_histogram(
            self,
            prefix_common_bucket,
            prefix_common_len,
            from,
            to,
            k,
            histogram,
        )
    }

    fn should_fallback(&self, from: i32, to: i32, l: i32) -> bool {
        default_should_fallback(from, to, l)
    }
}
impl StableMSBRadixSorterBase for StableMSBRadixSorterImpl<'_> {
    fn save(&mut self, i: i32, j: i32) {
        self.temp[j as usize] = self.refs[i as usize].clone();
    }

    fn restore(&mut self, i: i32, j: i32) {
        for idx in i..j {
            self.refs[idx as usize] = self.temp[idx as usize].clone();
        }
    }
}

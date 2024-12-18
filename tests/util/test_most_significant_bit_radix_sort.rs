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
use std::collections::{BTreeSet, HashSet};
use rand::{Rng, RngCore};
use rand::rngs::StdRng;
use rlucene::index::{BytesRef, BytesRefBuilder};
use rlucene::util::{MSBRadixSorter, MSBRadixSorterBase, Sorter};
use rlucene::util::error::runtime_error::RuntimeError;
use crate::common::my_random;
use crate::util::test_error::TestError;
use crate::util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestMSBRadixSorter;


fn test(refs: &mut [BytesRef], len: usize, random: &mut StdRng) -> Result<(), TestError>{
    let mut expected: Vec<BytesRef> = refs[..len].to_vec();
    expected.sort();

    let mut max_length:i32 = 0;
    for ref_item in &refs[..len] {
        max_length = max_length.max(ref_item.length as i32);
    }

    match random.gen_range(0..3) {
        0 => max_length += random.gen_range(1..=5),
        1 => max_length = i32::MAX,
        _ => {}
    }

    let final_max_length = max_length;
    let sub_sorter = MSBRadixSorterImpl::new(final_max_length, refs[..len].to_vec());
    let mut msb_radix_sorter = MSBRadixSorter::new(max_length, sub_sorter);
    msb_radix_sorter.sort(0, len as i32)?;

    assert_eq!(expected, msb_radix_sorter.get_sub_sorter().refs);
    Ok(())
}
#[test]
fn test_empty() -> Result<(), TestError>{
   let mut random = my_random("test_empty".to_string());
    let mut refs: Vec<BytesRef> = vec![BytesRef::default(); random.gen_range(0..5)];
    assert!(test(&mut refs, 0, &mut random).is_ok());
   test(&mut refs, 0, &mut random) 
}
#[test]
fn test_one_value() -> Result<(), TestError> {
    let mut random = my_random("test_one_value".to_string());

    let bytes = BytesRef::new_from_string(&TestUtil::random_simple_string(&mut random));
    let mut refs = vec![bytes];
    test(&mut refs, 1, &mut random)
}
#[test]
fn test_two_values() -> Result<(), TestError> {
    let mut random = my_random("test_two_values".to_string());

    let bytes1 = BytesRef::new_from_string(&TestUtil::random_simple_string(&mut random));
    let bytes2 = BytesRef::new_from_string(&TestUtil::random_simple_string(&mut random));
    let mut refs = vec![bytes1, bytes2];

    test(&mut refs, 2, &mut random)
}

fn test_random_impl(common_prefix_len: usize, max_len: i32, random: &mut StdRng) -> Result<(), TestError> {
    let mut common_prefix = vec![0u8; common_prefix_len];
    random.fill_bytes(&mut common_prefix);
    let len = random.gen_range(0..10000);
    let mut bytes: Vec<BytesRef> = Vec::with_capacity(len + random.gen_range(0..50));
    for _ in 0..len {
        let mut b = vec![0u8; common_prefix_len + random.gen_range(0..max_len) as usize];
        random.fill_bytes(&mut b[common_prefix_len..]);

        b[..common_prefix_len].copy_from_slice(&common_prefix);

        bytes.push(BytesRef::new_from_bytes(b));
    }
    test(&mut bytes, len, random)
}
#[test]
fn test_random() -> Result<(), TestError> {
    let mut random = my_random("test_random".to_string());
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
        let shared_prefix = random.gen_range(1..30);
        test_random_impl(shared_prefix, 10, &mut random)?;
    }
    Ok(())
}

#[test]
fn test_random_with_shared_prefix_and_lots_of_duplicates() -> Result<(), TestError> {
    let mut random = my_random("test_random_with_shared_prefix_and_lots_of_duplicates".to_string());
    for _ in 0..10 {
        let shared_prefix = random.gen_range(1..30);
        test_random_impl(shared_prefix, 2, &mut random)?;
    }
    Ok(())
}

#[test]
fn test_random2() -> Result<(), TestError> {
    let mut random = my_random("test_random2".to_string());
    // How large our alphabet is
    let letter_count = random.gen_range(2..=10);

    // How many substring fragments to use
    let substring_count = random.gen_range(2..10);
    let mut substrings_set = HashSet::new();

    // How many strings to make
    let string_count = random.gen_range(10000..1000000);
    // let string_count = ;

    // Generate unique substrings
    while substrings_set.len() < substring_count {
        let length = random.gen_range(2..10);
        let bytes: Vec<u8> = (0..length)
            .map(|_| random.gen_range(0..letter_count) as u8)
            .collect();
        let br = BytesRef::new_from_bytes(bytes);
        substrings_set.insert(br);
    }

    let substrings: Vec<BytesRef> = Vec::from_iter(substrings_set);
    let mut chance = vec![0.0; substrings.len()];
    let mut sum = 0.0;

    for chance_value in &mut chance {
        *chance_value = random.gen::<f64>();
        sum += *chance_value;
    }

    // give each substring a random chance of occurring:
    let mut accum = 0.0;
    for chance_value in chance.iter_mut() {
        accum += *chance_value / sum;
        *chance_value = accum;
    }

    // Generate unique strings
    let mut strings_set = BTreeSet::new();
    let mut iters = 0;
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

    // Run test with generated strings
    let strings: Vec<BytesRef> = strings_set.into_iter().collect();
    test(&mut strings.clone(), strings.len(), &mut random)
}

struct MSBRadixSorterImpl{
    final_max_length: i32,
    refs: Vec<BytesRef>
}

impl MSBRadixSorterImpl{
    fn new(final_max_length: i32, refs: Vec<BytesRef>) -> Self {
        Self {
            final_max_length,
            refs
        }
    }
}

impl MSBRadixSorterBase for MSBRadixSorterImpl{
    fn byte_at(&self, i: i32, k: i32) -> i32 {
        assert!(
            k < self.final_max_length,
            "Index out of bounds: k={} exceeds final_max_length={}",
            k,
            self.final_max_length
        );

        let ref_item = &self.refs[i as usize];
        if ref_item.length as i32 <= k {
            -1
        } else {
            ref_item.bytes[ref_item.offset as usize + k as usize] as i32 
        }
    }
}
impl Sorter for MSBRadixSorterImpl{
    fn compare(&self, _i: i32, _j: i32) -> i32 {
        unreachable!()
    }

    fn swap(&mut self, i: i32, j: i32) {
        self.refs.swap(i as usize, j as usize);
    }

    fn set_pivot(&mut self, _i: i32) {
        unreachable!()
    }

    fn compare_pivot(&self, _i: i32) -> i32 {
        unreachable!()
    }

    fn sort(&mut self, _from: i32, _to: i32) -> Result<(), RuntimeError> {
        unreachable!()
    }
}

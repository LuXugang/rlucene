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
use rlucene::util::bytes_ref_comparator::{BytesRefComparator, Natural};
use rlucene::util::error::runtime_error::RuntimeError;
use rlucene::util::{
    default_fall_back_sorter, default_radix_sorter, Comparator, NaturalOrder, Sorter, StringSorter,
    StringSorterBase,
};

#[allow(dead_code)] // for quick search
struct TestStringSorter;

fn test(refs: Vec<BytesRef>, len: usize) -> Result<(), TestError> {
    test_impl(refs.clone(), len, Natural::default())?;
    test_impl(refs.clone(), len, NaturalOrder::default())?;
    Ok(())
}

fn test_impl(
    refs: Vec<BytesRef>,
    len: usize,
    comparator: impl BytesRefComparator + Comparator<BytesRef>,
) -> Result<(), TestError> {
    let mut expected: Vec<BytesRef> = refs.clone();
    expected.sort();
    let sub_sorter = StringSorterImpl::new(refs.clone());
    let mut string_sorter = StringSorter::new(sub_sorter, comparator);
    string_sorter.sort(0, len as i32)?;

    assert_vecs_equal(&expected, &string_sorter.get_sub_sorter().refs);
    Ok(())
}
#[test]
fn test_empty() -> Result<(), TestError> {
    let mut random = my_random("test_empty".to_string());
    let len = random.gen_range(0..5);
    let refs: Vec<BytesRef> = (0..len).map(|_| BytesRef::default()).collect();
    test(refs, 0)
}

#[test]
fn test_one_value() -> Result<(), TestError> {
    let mut random = my_random("test_one_value".to_string());
    let bytes = BytesRef::new_from_string(&TestUtil::random_simple_string(&mut random));
    test(vec![bytes], 1)
}

#[test]
fn test_two_values() -> Result<(), TestError> {
    let mut random = my_random("test_two_values".to_string());
    let bytes1 = BytesRef::new_from_string(&TestUtil::random_simple_string(&mut random));
    let bytes2 = BytesRef::new_from_string(&TestUtil::random_simple_string(&mut random));
    test(vec![bytes1, bytes2], 2)
}

fn test_random_impl(
    common_prefix_len: usize,
    max_len: usize,
    random: &mut StdRng,
) -> Result<(), TestError> {
    let mut common_prefix = vec![0u8; common_prefix_len];
    random.fill_bytes(&mut common_prefix);
    // let len = random.gen_range(0..100000);
    let len = random.gen_range(0..200);

    let mut bytes: Vec<BytesRef> = Vec::with_capacity(len + random.gen_range(0..50));
    for _ in 0..len {
        let mut b = vec![0u8; common_prefix_len + random.gen_range(0..max_len)];
        random.fill_bytes(&mut b[common_prefix_len..]);
        b[..common_prefix_len].copy_from_slice(&common_prefix);
        bytes.push(BytesRef::new_from_bytes(b));
    }

    test(bytes, len)
}
#[test]
fn test_random() -> Result<(), TestError> {
    let mut random = my_random("test_random".to_string());
    let num_iters = random.gen_range(3..100);
    for _ in 0..num_iters {
        test_random_impl(0, 10, &mut random)?;
    }
    Ok(())
}
#[test]
fn test_random_with_lots_of_duplicates() -> Result<(), TestError> {
    let mut random = my_random("test_random_with_lots_of_duplicates".to_string());
    let num_iters = random.gen_range(3..100);
    for _ in 0..num_iters {
        test_random_impl(0, 2, &mut random)?;
    }
    Ok(())
}
#[test]
fn test_random_with_shared_prefix() -> Result<(), TestError> {
    let mut random = my_random("test_random_with_shared_prefix".to_string());
    let num_iters = random.gen_range(3..100);
    for _ in 0..num_iters {
        let shared_prefix_len = random.gen_range(1..30);
        test_random_impl(shared_prefix_len, 10, &mut random)?;
    }
    Ok(())
}
#[test]
fn test_random_with_shared_prefix_and_lots_of_duplicates() -> Result<(), TestError> {
    let mut random = my_random("test_random_with_shared_prefix_and_lots_of_duplicates".to_string());
    let num_iters = random.gen_range(3..100);
    for _ in 0..num_iters {
        let shared_prefix_len = random.gen_range(1..30);
        test_random_impl(shared_prefix_len, 2, &mut random)?;
    }
    Ok(())
}

struct StringSorterImpl {
    refs: Vec<BytesRef>,
}

impl StringSorterImpl {
    fn new(refs: Vec<BytesRef>) -> Self {
        Self { refs }
    }
}
impl Sorter for StringSorterImpl {
    fn compare(&mut self, _i: i32, _j: i32) -> i32 {
        unreachable!()
    }

    fn swap(&mut self, i: i32, j: i32) {
        self.refs.swap(i as usize, j as usize);
    }

    fn set_pivot(&mut self, _i: i32) {
        unreachable!()
    }

    fn compare_pivot(&mut self, _i: i32) -> i32 {
        unreachable!()
    }

    fn sort(&mut self, _from: i32, _to: i32) -> Result<(), RuntimeError> {
        unreachable!()
    }
}
impl StringSorterBase for StringSorterImpl {
    fn get(&mut self, _builder: &mut BytesRefBuilder, result: &mut BytesRef, i: i32) {
        let ref_item = &self.refs[i as usize];
        result.offset = ref_item.offset;
        result.length = ref_item.length;
        result.bytes = ref_item.bytes.clone();
    }

    fn fall_back_sorter<'a, T, C>(&'a mut self, cmp: &'a mut C, k: Option<i32>) -> impl Sorter + 'a
    where
        T: Sorter + StringSorterBase,
        C: BytesRefComparator + Comparator<BytesRef>,
    {
        default_fall_back_sorter(cmp, self, k)
    }

    fn radix_sorter<'a, C>(&'a mut self, cmp: &'a mut C) -> impl Sorter + 'a
    where
        C: BytesRefComparator + Comparator<BytesRef>,
    {
        default_radix_sorter(cmp, self)
    }
}

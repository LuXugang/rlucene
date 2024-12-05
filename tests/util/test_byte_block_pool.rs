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
use crate::common::my_random;
use rand::distributions::Alphanumeric;
use rand::{Rng, RngCore};
use rlucene::index::{BytesRef, BytesRefBuilder};
use rlucene::util::{
    new_counter, AllocatorEnum, ByteBlockPool, DirectAllocator, DirectTrackingAllocator,
    BYTE_BLOCK_SIZE,
};

#[allow(dead_code)] // for quick search
struct TestByteBlockPool {}
#[test]
fn test_append_from_other_pool() {
    let mut random = my_random("test_append_from_other_pool".to_string());
    let mut pool = ByteBlockPool::new(AllocatorEnum::DA(DirectAllocator::new()));
    let num_bytes = random.gen_range(2 << 16..(2 << 16) + 1000000);
    let bytes = (&mut random)
        .sample_iter(&Alphanumeric)
        .take(num_bytes)
        .map(char::from)
        .collect::<String>()
        .as_bytes()
        .to_vec();
    pool.append(bytes.clone());
    let bytes_length = bytes.len();

    let mut another_pool = ByteBlockPool::new(AllocatorEnum::DA(DirectAllocator::new()));
    let existing_bytes = vec![0; random.gen_range(500..100000)];
    another_pool.append(existing_bytes.clone());

    let offset = random.gen_range(1..=bytes_length);
    let mut length = bytes_length - offset;
    if random.gen_bool(0.5) {
        length = random.gen_range(1..=length);
    }
    another_pool.append_from_byte_block_pool(&pool, offset as i64, length as i32);
    assert_eq!(
        (existing_bytes.len() + length) as i64,
        another_pool.get_position()
    );

    let mut result = vec![0; length];
    let result_length = result.len() as i32;
    another_pool.read_bytes(existing_bytes.len() as i64, &mut result, 0, result_length);
    for i in 0..length {
        assert_eq!(bytes[offset + i], result[i], "byte @ index= {}", i);
    }
}
#[test]
fn test_read_and_write() {
    let mut random = my_random("test_read_and_write".to_string());
    let mut byte_used = new_counter(false);
    let mut pool = ByteBlockPool::new(AllocatorEnum::DTA(DirectTrackingAllocator::new(
        &mut byte_used,
    )));
    pool.next_buffer();
    let reuse_first = random.gen_bool(0.5);
    for _j in 0..2 {
        let mut list: Vec<BytesRef> = Vec::new();
        let max_length = random.gen_range(500..1000);
        let num_values = random.gen_range(100..1000);
        let mut bytes_ref_builder = BytesRefBuilder::new();
        for _i in 0..num_values {
            let value = (&mut random)
                .sample_iter(&Alphanumeric)
                .take(max_length)
                .map(char::from)
                .collect::<String>();
            let value_copy = value.clone();
            list.push(BytesRef::new_from_string(&value));
            bytes_ref_builder.copy_chars_with_string(&value_copy);
            pool.append_bytes_ref(bytes_ref_builder.get().clone());
        }
        let mut position = 0;
        let mut builder = BytesRefBuilder::new();
        for expected in list.iter() {
            bytes_ref_builder.set_length(expected.length);
            assert!(bytes_ref_builder.length() <= i32::MAX as u32);
            let bytes_ref_builder_length = bytes_ref_builder.length();
            let value = random.gen_range(0..2);
            match value {
                0 => {
                    pool.read_bytes(
                        position,
                        &mut bytes_ref_builder.get().bytes,
                        0,
                        bytes_ref_builder_length as i32,
                    );
                }
                1 => {
                    let mut scratch = BytesRef::new();
                    scratch.bytes = vec![0; bytes_ref_builder_length as usize];
                    pool.set_bytes_ref(
                        &mut builder,
                        &mut scratch,
                        position,
                        bytes_ref_builder.length() as i32,
                    );
                    bytes_ref_builder.get().bytes[0..bytes_ref_builder_length as usize]
                        .copy_from_slice(
                            &scratch.bytes[scratch.offset as usize
                                ..(scratch.offset + bytes_ref_builder_length) as usize],
                        );
                }
                _ => {
                    unreachable!()
                }
            }
            assert!(bytes_ref_builder.get().bytes_equals(expected));
            position += bytes_ref_builder.length() as i64;
        }
        pool.reset(random.gen_bool(0.5), reuse_first);
        if reuse_first {
            assert_eq!(BYTE_BLOCK_SIZE as i64, pool.get_bytes_used())
        } else {
            assert_eq!(0, pool.get_bytes_used());
            pool.next_buffer();
        }
    }
}
#[test]
fn test_large_random_block() {
    let mut random = my_random("test_large_random_block".to_string());
    let mut byte_used = new_counter(false);
    let mut pool = ByteBlockPool::new(AllocatorEnum::DTA(DirectTrackingAllocator::new(
        &mut byte_used,
    )));
    pool.next_buffer();

    let mut total_bytes = 0;
    let iter = 100;
    let mut iterms: Vec<Vec<u8>> = vec![vec![]; iter];

    let mut size: i32;
    for _i in 0..iter {
        if random.gen_bool(0.5) {
            size = random.gen_range(100..1000);
        } else {
            size = random.gen_range(50000..100000);
        }
        let mut bytes = vec![0; size as usize];
        random.fill_bytes(&mut bytes);
        let bytes_clone = bytes.clone();
        iterms.push(bytes);
        pool.append_bytes_ref(BytesRef::new_from_bytes(bytes_clone));
        total_bytes += size;

        // make sure we report the correct position
        assert_eq!(total_bytes as i64, pool.get_position());
    }

    let mut position = 0;
    for expected in iterms {
        let mut actual: Vec<u8> = vec![0; expected.len()];
        let actual_len = actual.len();
        pool.read_bytes(position, &mut actual, 0, actual_len as i32);
        assert_eq!(expected, actual);
        position += expected.len() as i64;
    }
}

#[test]
fn test_too_many_allocs() {}

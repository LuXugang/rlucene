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
use crate::util::lucene_test_case::{at_least, random};
use crate::util::test_error::TestError;
use crate::util::TestUtil;
use rand::distributions::Alphanumeric;
use rand::{Rng, RngCore};
use rlucene::index::{BytesRef, BytesRefBuilder};
use rlucene::util::error::lucene_error::LuceneError;
use rlucene::util::{
    AllocatorEnum, ByteBlockPool, CounterEnum, DirectAllocator, DirectTrackingAllocator, VecCopyOps,
};
use std::sync::{Arc, Mutex};

#[allow(dead_code)] // for quick search
struct TestByteBlockPool {}
#[test]
fn test_append_from_other_pool() -> Result<(), LuceneError> {
    let mut random = random();
    let mut pool = ByteBlockPool::new(AllocatorEnum::DA(DirectAllocator::new()));
    let num_bytes = at_least(&mut random, 2 << 16) as usize;
    let bytes = (&mut random)
        .sample_iter(&Alphanumeric)
        .take(num_bytes)
        .map(char::from)
        .collect::<String>()
        .as_bytes()
        .to_vec();
    pool.append(&bytes)?;
    let bytes_length = bytes.len();

    let mut another_pool = ByteBlockPool::new(AllocatorEnum::DA(DirectAllocator::new()));
    let existing_bytes = vec![0; at_least(&mut random, 500) as usize];
    another_pool.append(&existing_bytes)?;

    // now slice and append to another pool
    let offset = TestUtil::next_int(&mut random, 1, 2 << 15) as usize;
    let mut length = bytes_length - offset;
    if random.gen_bool(0.5) {
        length = TestUtil::next_int(&mut random, 1, length as i32) as usize;
    }
    another_pool.append_from_byte_block_pool(&pool, offset as i64, length as i32)?;
    assert_eq!(
        (existing_bytes.len() + length) as i64,
        another_pool.get_position()
    );

    let mut result = vec![0; length];
    let result_length = result.len();
    another_pool.read_bytes(
        existing_bytes.len() as i64,
        &mut result,
        0,
        result_length as i32,
    );
    for i in 0..length {
        assert_eq!(bytes[offset + i], result[i], "byte @ index= {}", i);
    }
    Ok(())
}
#[test]
fn test_read_and_write() -> Result<(), LuceneError> {
    let mut random = random();
    let byte_used = Arc::new(Mutex::new(CounterEnum::new_counter(false)));
    let mut pool = ByteBlockPool::new(AllocatorEnum::DTA(DirectTrackingAllocator::new(byte_used)));
    pool.next_buffer()?;
    let reuse_first = random.gen_bool(0.5);
    for _j in 0..2 {
        let mut list: Vec<BytesRef> = Vec::new();
        let max_length = at_least(&mut random, 500) as usize;
        let num_values = at_least(&mut random, 100) as usize;
        let mut bytes_ref_builder = BytesRefBuilder::new();
        for _i in 0..num_values {
            let value = (&mut random)
                .sample_iter(&Alphanumeric)
                .take(max_length)
                .map(char::from)
                .collect::<String>();
            let value_copy = value.clone();
            list.push(BytesRef::from_string(&value));
            bytes_ref_builder.copy_chars_with_string(&value_copy)?;
            pool.append_bytes_ref(bytes_ref_builder.get())?;
        }
        let mut position = 0;
        let mut builder = BytesRefBuilder::new();
        for expected in list.iter() {
            bytes_ref_builder.set_length(expected.length);
            let bytes_ref_builder_length = bytes_ref_builder.length();
            let value = random.gen_range(0..2);
            match value {
                0 => {
                    pool.read_bytes(
                        position,
                        &mut bytes_ref_builder.get().bytes,
                        0,
                        bytes_ref_builder_length,
                    );
                }
                1 => {
                    let mut scratch = BytesRef::new();
                    scratch.bytes = vec![0; bytes_ref_builder_length as usize];
                    pool.set_bytes_ref(
                        &mut builder,
                        &mut scratch,
                        position,
                        bytes_ref_builder.length(),
                    );
                    bytes_ref_builder.get().bytes.copy_from(
                        &scratch.bytes[scratch.offset as usize
                            ..(scratch.offset + bytes_ref_builder_length) as usize],
                        0,
                    );
                }
                _ => {
                    unreachable!()
                }
            }
            assert!(bytes_ref_builder.get().bytes_equals(expected));
            position += bytes_ref_builder.length() as i64;
        }
        pool.reset(random.gen_bool(0.5), reuse_first)?;
        if reuse_first {
            assert_eq!(
                ByteBlockPool::BYTE_BLOCK_SIZE as i64,
                pool.get_bytes_used()?
            )
        } else {
            assert_eq!(0, pool.get_bytes_used()?);
            pool.next_buffer()?;
        }
    }
    Ok(())
}
#[test]
fn test_large_random_block() -> Result<(), TestError> {
    let mut random = random();
    let byte_used = Arc::new(Mutex::new(CounterEnum::new_counter(false)));
    let mut pool = ByteBlockPool::new(AllocatorEnum::DTA(DirectTrackingAllocator::new(byte_used)));
    let _ = pool.next_buffer();

    let mut total_bytes = 0;
    let iter = 100;
    let mut iterms: Vec<Vec<u8>> = vec![vec![]; iter];

    let mut size: i32;
    for _i in 0..iter {
        if random.gen_bool(0.5) {
            size = TestUtil::next_int(&mut random, 100, 1000);
        } else {
            size = TestUtil::next_int(&mut random, 50000, 100000);
        }
        let mut bytes = vec![0; size as usize];
        random.fill_bytes(&mut bytes);
        let bytes_clone = bytes.clone();
        iterms.push(bytes);
        pool.append_bytes_ref(&BytesRef::from_bytes(bytes_clone))?;
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
    Ok(())
}

#[test]
fn test_too_many_allocs() {}

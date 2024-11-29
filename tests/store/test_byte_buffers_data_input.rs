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
use crate::common::{is_night_mode, my_random};
use crate::store::add_random_data;
use rand::Rng;
use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoroshiro128Plus;
use rlucene::store::byte_buffers_data_input::ByteBuffersDataInput;
use rlucene::store::data_output::DataOutput;
use rlucene::store::random_access_input::RandomAccessInput;
use rlucene::store::{ByteBuffersDataOutput, DataInput};

#[allow(dead_code)]
struct TestByteBuffersDataInput;

#[test]
fn test_sanity() {
    let mut out = ByteBuffersDataOutput::new_resettable_instance().unwrap();
    let mut o1 = out.get_data_input();
    assert_eq!(0, o1.length());
    let mut result = DataInput::read_byte(&mut o1);
    assert!(result.is_err());

    out.write_byte(1).unwrap();
    // TODO: how to assert o1's length not modified?
    // assert_eq!(0, o1.length());
    let mut o2 = out.get_data_input();
    assert_eq!(1, o2.length());
    assert_eq!(0, o2.position());

    //TODO
    // assert!(o2.ram_bytes_used() > 0)
    assert_eq!(1, DataInput::read_byte(&mut o2).unwrap() as i32);
    assert_eq!(1, o2.position());
    assert_eq!(1, RandomAccessInput::read_byte(&mut o2, 0).unwrap() as i32);

    result = DataInput::read_byte(&mut o2);
    assert!(result.is_err());
    assert_eq!(1, o2.position());
}

#[test]
fn test_random_reads() {
    let mut random = my_random("test_random_reads".to_string());
    let mut dst = ByteBuffersDataOutput::new_resettable_instance().unwrap();
    let seed: u64 = random.gen();
    let mut random1 = Xoroshiro128Plus::seed_from_u64(seed);
    let max = if is_night_mode() { 1000000 } else { 100000 };
    let reply = add_random_data::<ByteBuffersDataInput>(&mut dst, &mut random1, max);
    let mut src = dst.get_data_input();
    for mut f in reply {
        f(&mut src);
    }
    let result = DataInput::read_byte(&mut src);
    assert!(result.is_err());
}

#[test]
fn test_random_reads_on_slices() {
    let mut random = my_random("test_random_reads_on_slices".to_string());
    let reps = random.gen_range(1..=20);
    for _i in 0..=reps {
        let mut dst = ByteBuffersDataOutput::new_resettable_instance().unwrap();
        let prefix: Vec<u8> = vec![0; random.gen_range(0..=1024 * 8)];
        let prefix_len = prefix.len() as u64;
        dst.write_bytes(prefix).unwrap();
        let seed: u64 = random.gen();
        let max = 10000;
        let mut random1 = Xoroshiro128Plus::seed_from_u64(seed);
        let reply = add_random_data::<ByteBuffersDataInput>(&mut dst, &mut random1, max);
        let suffix: Vec<u8> = vec![0; random.gen_range(0..=1024 * 8)];
        let suffix_len = suffix.len() as u64;
        dst.write_bytes(suffix).unwrap();
        let size = dst.size();
        let mut src = dst
            .get_data_input()
            .slice(prefix_len, size - suffix_len - prefix_len)
            .unwrap();
        assert_eq!(0, src.position());
        assert_eq!(size - prefix_len - suffix_len, src.length());
        for mut f in reply {
            f(&mut src);
        }
        let result = DataInput::read_byte(&mut src);
        assert!(result.is_err());
    }
}
#[test]
fn test_seek_empty() {
    let mut dst = ByteBuffersDataOutput::new_resettable_instance().unwrap();
    let mut data_input = dst.get_data_input();
    let mut result = data_input.seek(0);
    assert!(result.is_ok());
    result = data_input.seek(1);
    assert!(result.is_err());
    result = data_input.seek(0);
    assert!(result.is_ok());
    let read_result = DataInput::read_byte(&mut data_input);
    assert!(read_result.is_err());
}

#[test]
fn test_seek_and_skip() {
    let mut random = my_random("test_seek_and_skip".to_string());
    let reps = random.gen_range(1..=20);
    for _i in 0..reps {
        let mut dst = ByteBuffersDataOutput::new_resettable_instance().unwrap();
        let mut prefix: Vec<u8> = vec![];
        let mut prefix_len: u64 = 0;
        // if random.gen_bool(0.5){
        let len = random.gen_range(0..=1024 * 8);
        prefix = vec![0; len];
        prefix_len = prefix.len() as u64;
        dst.write_bytes(prefix).unwrap();
        // }
        let seed: u64 = random.gen();
        let max = 1000;
        let mut random1 = Xoroshiro128Plus::seed_from_u64(seed);
        let mut reply = add_random_data::<ByteBuffersDataInput>(&mut dst, &mut random1, max);
        let size = dst.size();
        let mut array = dst.get_array_copy();
        array = Vec::from(&array[prefix_len as usize..array.len()]);
        let mut data_input = dst
            .get_data_input()
            .slice(prefix_len, size - prefix_len)
            .unwrap();
        data_input.seek(0).unwrap();
        for f in reply.iter_mut() {
            f(&mut data_input);
        }
        data_input.seek(0).unwrap();
        for f in reply.iter_mut() {
            f(&mut data_input);
        }
        for _i in 0..1000 {
            let offs = random.gen_range(0..=array.len() - 1);
            data_input.seek(offs as u64).unwrap();
            assert_eq!(offs as u64, data_input.position());
            assert_eq!(array[offs], DataInput::read_byte(&mut data_input).unwrap());
        }
        // test skipping
        let max_skip_to = array.len() - 1;
        data_input.seek(0).unwrap();
        // skip chunks of bytes until exhausted
        let mut curr = 0;
        while curr < max_skip_to {
            let skip_to = random.gen_range(curr..=max_skip_to);
            let step = skip_to - curr;
            data_input.skip_bytes(step as u64).unwrap();
            assert_eq!(
                array[skip_to],
                DataInput::read_byte(&mut data_input).unwrap()
            );
            curr = skip_to + 1;
        }

        data_input.seek(data_input.length()).unwrap();
        assert_eq!(data_input.length(), data_input.position());
        let result = DataInput::read_byte(&mut data_input);
        assert!(result.is_err());
    }
}
#[test]
fn test_slicing_window() {
    let mut random = my_random("test_slicing_window".to_string());
    let mut dst = ByteBuffersDataOutput::new_resettable_instance().unwrap();
    assert_eq!(0, dst.get_data_input().slice(0, 0).unwrap().length());
    let random_bytes: Vec<u8> = vec![0; random.gen_range(0..=1024 * 8)];
    dst.write_bytes(random_bytes);
    let max = dst.size();
    let data_input = dst.get_data_input();
    let mut offset = 0;
    while offset < max {
        assert_eq!(0, data_input.slice(offset, 0).unwrap().length());
        assert_eq!(1, data_input.slice(offset, 1).unwrap().length());
        offset += 1;

        let window = (max - offset).min(1024);
        assert_eq!(window, data_input.slice(offset, window).unwrap().length());
    }
    assert_eq!(0, data_input.slice(max, 0).unwrap().length());
}

#[test]
fn test_eof_on_array_read_past_buffer_size() {
    let mut dst = ByteBuffersDataOutput::new_resettable_instance().unwrap();
    let bytes: Vec<u8> = vec![0; 10];
    dst.write_bytes(bytes).unwrap();
    let mut data_input = dst.get_data_input();
    let mut output: Vec<u8> = vec![0; 100];
    let result = DataInput::read_bytes(&mut data_input, &mut output, 0, 100);
    assert!(result.is_err());
}

#[test]
fn test_slicing_large_buffers() {
    // Simulate a "large" (> 4GB) input by duplicating
    // buffers with the same content.
    let mut random = my_random("test_slicing_large_buffers".to_string());
    let mb = 1024 * 1024;
    let page_bytes: Vec<u8> = vec![0; 4 * mb];
    let simulated_length: i64 = random.gen_range(0..2018) as i64 + 4 * i32::MAX as i64;
    let mut remaining = simulated_length;
    let mut dst = ByteBuffersDataOutput::new_resettable_instance().unwrap();
    while remaining > 0 {
        let mut block = page_bytes.clone();
        if block.len() > remaining as usize {
            block.truncate(remaining as usize);
        }
        let len = block.len();
        dst.write_bytes(block).unwrap();
        remaining -= len as i64;
    }
    let data_input = dst.get_data_input();
    assert_eq!(simulated_length as u64, data_input.length());
    let max = data_input.length();
    let mut offset = 0;
    while offset < max {
        assert_eq!(0, data_input.slice(offset, 0).unwrap().length());
        assert_eq!(1, data_input.slice(offset, 1).unwrap().length());

        let window = (max - offset).min(1024);
        let mut slice = data_input.slice(offset, window).unwrap();
        assert_eq!(window, slice.length());
        // Sanity check of the content against original pages.
        for i in 0..window {
            let index = (offset + i) % page_bytes.len() as u64;
            assert!(index <= u32::MAX as u64);
            let expected = page_bytes[index as usize];
            assert_eq!(
                expected,
                RandomAccessInput::read_byte(&mut slice, i).unwrap()
            );
        }
        offset += random.gen_range(mb..4 * mb) as u64;
    }
}

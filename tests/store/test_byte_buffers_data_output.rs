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
use std::sync::atomic::AtomicI32;
use rand::{Rng};
use rlucene::store::{ByteArrayDataInput, ByteBuffersDataOutput, DEFAULT_MAX_BITS_PER_BLOCK, DEFAULT_MIN_BITS_PER_BLOCK, LIMIT_MIN_BITS_PER_BLOCK, MAX_BLOCKS_BEFORE_BLOCK_EXPANSION};
use rlucene::store::data_output::DataOutput;
use crate::common::{is_night_mode, my_random, my_random_with_seed};
use crate::store::base_data_output_test_case::{add_random_data, BaseDataOutputTestCase};

struct TestByteBuffersDataOutput;
impl BaseDataOutputTestCase for TestByteBuffersDataOutput {
    type DO = ByteBuffersDataOutput;

    fn new_instance(&self) -> Self::DO {
        ByteBuffersDataOutput::new_default().unwrap()
    }

    fn get_bytes(&mut self, mut instance: Self::DO) -> Vec<u8>{
        instance.to_array_copy()
    }
}

#[test]
fn test_reuse(){
    let mut random = my_random("test_reuse".to_string());
    let mut o = ByteBuffersDataOutput::new(DEFAULT_MIN_BITS_PER_BLOCK, DEFAULT_MAX_BITS_PER_BLOCK, true).unwrap();
    // add some random data first
    let gen_seed:u64= random.gen();
    let mut random1= my_random_with_seed(gen_seed);
    let mut random2= my_random_with_seed(gen_seed);
    let add_count = random.gen_range(1000..=5000);
    add_random_data::<ByteArrayDataInput>(&mut o, &mut random1, add_count);
    let dta = o.to_array_copy();
    
    o.reset();
    add_random_data::<ByteArrayDataInput>(&mut o, &mut random2, add_count);
    assert_eq!(dta, o.to_array_copy());
}
#[test]
fn test_constructor_with_expected_size(){
    let mut random = my_random("test_constructor_with_expectedsize".to_string());
    let mut o = ByteBuffersDataOutput::new_with_expected_size(0).unwrap();
    o.write_byte(0).unwrap();
    let mut result = o.to_buffer_list();
    let capacity = result.get_mut(0).unwrap().get_ref().len();
    assert_eq!(1 << DEFAULT_MIN_BITS_PER_BLOCK, capacity);
    
    let mb = 1024 *1024;
    let expected_size:i64 = random.gen_range(mb..mb * 1024);
    let mut o = ByteBuffersDataOutput::new_with_expected_size(expected_size as u64).unwrap();
    let _ = o.write_byte(0);
    let cap = o.to_buffer_list().get_mut(0).unwrap().get_ref().len();
    assert!( (cap >> 1) * MAX_BLOCKS_BEFORE_BLOCK_EXPANSION < expected_size as usize);
    assert!(cap * MAX_BLOCKS_BEFORE_BLOCK_EXPANSION >= expected_size as usize);
}

#[test]
fn test_randomized_writes(){
    let mut test = TestByteBuffersDataOutput;
    let mut random = my_random("test_randomized_writes".to_string());
    test.test_randomized_writes::<ByteArrayDataInput>(&mut random);
}


#[test]
fn test_illegal_min_bits_per_block(){
    let o = ByteBuffersDataOutput::new(LIMIT_MIN_BITS_PER_BLOCK - 1, DEFAULT_MAX_BITS_PER_BLOCK, false);
    assert!(o.is_err());
}
#[test]
fn test_illegal_max_bits_per_block(){
    let o = ByteBuffersDataOutput::new(DEFAULT_MIN_BITS_PER_BLOCK, LIMIT_MIN_BITS_PER_BLOCK + 1, false);
    assert!(o.is_err());
}
#[test]
fn test_illegal_bits_per_block_range(){
    let o = ByteBuffersDataOutput::new(20, 19, false);
    assert!(o.is_err());
}
#[test]
fn test_sanity(){
    let case = TestByteBuffersDataOutput;
    let mut o = case.new_instance();

    assert_eq!(o.size(), 0);
    assert_eq!(o.to_array_copy().len(), 0);
    // TODO
    // assert_eq!(o.ram_bytes_used(), 0);

    o.write_byte(1).unwrap();
    assert_eq!(o.size(), 1);
    // TODO
    // assert!(o.ram_bytes_used() > 0);
    assert_eq!(o.to_array_copy(), vec![1]);

    o.write_bytes_with_len(&[2, 3, 4], 3).unwrap();
    assert_eq!(o.size(), 4);
    assert_eq!(o.to_array_copy(), vec![1, 2, 3, 4]);
}
#[test]
fn test_large_array_add() {
    let mut random = my_random("test_large_array_add".to_string());
    let mut o = ByteBuffersDataOutput::new_default().unwrap();
    let mb = 1024 * 1024;
    let mut bytes = if is_night_mode() {
        let size = random.gen_range(5 * mb..=15 * mb);
        vec![0u8; size]
    } else {
        let size = random.gen_range(mb / 2..=mb);
        vec![0u8; size]
    };

    bytes.iter_mut().for_each(|byte| *byte = random.gen());
    let offset = random.gen_range(0..=100);
    let len = bytes.len() - offset;
    o.write_bytes_range(&bytes,offset, len).unwrap();
    assert_eq!(len as u64, o.size());
    let expected = bytes[offset..offset + len].to_vec();
    assert_eq!(expected, o.to_array_copy());
}
#[test]
fn test_copy_bytes_on_heap() {
    let mut random = my_random("test_copy_bytes_on_heap".to_string());
    let mut bytes = vec![0u8; 1024 * 8 + 10];
    random.fill(&mut bytes[..]);
    let offset = random.gen_range(0..=100);
    let len = bytes.len() - offset;
    let bytes_clone = bytes.clone();
    let mut input = ByteArrayDataInput::new_with_range(bytes, offset, len);

    let mut o = ByteBuffersDataOutput::new(
        DEFAULT_MIN_BITS_PER_BLOCK,
        DEFAULT_MAX_BITS_PER_BLOCK,false).unwrap();
    o.copy_bytes(&mut input, len as i64).unwrap();
    let expected = bytes_clone[offset..offset + len].to_vec();
    assert_eq!(o.to_array_copy(), expected);
}
#[test]
fn test_copy_bytes_on_direct_byte_buffer() {
    let mut random = my_random("test_copy_bytes_on_direct_byte_buffer".to_string());
    let mut bytes = vec![0u8; 1024 * 8 + 10];
    random.fill(&mut bytes[..]);
    let offset = random.gen_range(0..=100);
    let len = bytes.len() - offset;
    let bytes_clone = bytes.clone();
    let mut input = ByteArrayDataInput::new_with_range(bytes, offset, len);
    let mut o = ByteBuffersDataOutput::new(
        DEFAULT_MIN_BITS_PER_BLOCK,
        DEFAULT_MAX_BITS_PER_BLOCK,
        false,
    )
        .unwrap();
    o.copy_bytes(&mut input, len as i64).unwrap();
    let expected = bytes_clone[offset..offset + len].to_vec();
    assert_eq!(o.to_array_copy(), expected);
}

#[test]
#[allow(dead_code)]
fn test_ram_bytes_used(){
    // TODO
}
#[allow(dead_code)]
fn compute_ram_bytes_used(){
    // TODO
}
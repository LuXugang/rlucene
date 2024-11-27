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
use rand::rngs::StdRng;
use rand::{Rng, RngCore};
use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoroshiro128Plus;
use rlucene::store::data_output::DataOutput;
use rlucene::store::output_stream_data_output::OutputStreamDataOutput;
use rlucene::store::DataInput;

pub trait BaseDataOutputTestCase {
    type DO: DataOutput;

    fn new_instance(&self) -> Self::DO;
    fn get_bytes(&mut self, instance: Self::DO) -> Vec<u8>;

    fn test_randomized_writes<DI: DataInput>(&mut self, random: &mut StdRng) {
        let seed: u64 = random.gen();
        let mut instance = self.new_instance();
        let mut buffer = Vec::new();
        let mut os = OutputStreamDataOutput::new(&mut buffer);
        let max = 500000;
        let mut random1 = Xoroshiro128Plus::seed_from_u64(seed);
        let mut random2= Xoroshiro128Plus::seed_from_u64(seed);

        add_random_data::<DI>(&mut instance, &mut random1, max);
        add_random_data::<DI>(&mut os, &mut random2, max);
        assert_eq!(&self.get_bytes(instance), os.os.into_inner().unwrap());
    }
}

pub fn add_random_data<DI: DataInput>(
    dst: &mut impl DataOutput,
    rnd: &mut impl RngCore,
    max_add_calls: i32,
) -> Vec<fn(&mut DI) -> ()> {
    let cg = create_generators();
    let mut vec: Vec<fn(&mut DI) -> ()> = Vec::new();
    for _i in 0..max_add_calls {
        let random_generator = rnd.gen_range(0..cg.len());
        vec.push(cg[random_generator](dst, rnd));
    }
    vec
}

type Generator<DO, DI, R> = fn(&mut DO, &mut R) -> fn(&mut DI) -> ();

fn create_generators<DO: DataOutput, DI: DataInput, R:RngCore>() -> Vec<Generator<DO, DI,R >> {
    vec![
        //0 writeByte / readByte
        |dst, rnd| {
            let value: u8 = rnd.gen();
            let _ =dst.write_byte(value);
            |src| {}
        },
        //1 writeBytes / readBytes (array and buffer version).
        |dst, rnd| {
            let len = rnd.gen_range(0..100);
            let bytes: Vec<u8> = (0..len).map(|_| rnd.gen()).collect();
            let bytes_len = bytes.len();
            let _ = dst.write_bytes_with_len(&bytes, bytes_len);
            |src| {}
        },
        //2 writeBytes / readBytes (array + offset).
        |dst, rnd| {
            let len = rnd.gen_range(0..10000);
            let bytes: Vec<u8> = (0..len).map(|_| rnd.gen()).collect();
            let bytes_len = bytes.len();
            let off = if len == 0 { 0 } else { rnd.gen_range(0..bytes_len) };
            let length = if len == 0 { 0 } else { rnd.gen_range(0..(bytes_len - off)) };
            let _ = dst.write_bytes_range(&bytes, off, length);
            |src| {}
        },
        //3 writeInt / readInt
        |dst, rnd| {
            let value: i32 = rnd.gen();
            let _ =dst.write_int(value);
            |src| {}
        },
        //4 writeLong / readInt
        |dst, rnd| {
            let value: i64 = rnd.gen();
            let _ =dst.write_long(value);
            |src| {}
        },
        //5 writeShort / readShort
        |dst, rnd| {
            let value: i16 = rnd.gen();
            let _ = dst.write_short(value);
            |src| {}
        },
        //6 writeVInt / readVInt
        |dst, rnd| {
            let value: i32 = rnd.gen();
            let _ =dst.write_vint(value);
            |src| {}
        },
        //7 writeZInt / readZInt
        |dst, rnd| {
            let value: i32 = rnd.gen();
            let _ = dst.write_zint(value);
            |src| {}
        },
        //8 writeZLong / readZLong
        |dst, rnd| {
            let value: i64 = rnd.gen();
            let _ =dst.write_zlong(value);
            |src| {}
        },
        //9 writeVLong / readVLong
        |dst, rnd| {
            let value: i64 = rnd.gen();
            let _ = dst.write_vlong(value);
            |src| {}
        },
        //10  writeString / readString
        |dst, rnd| {
            let value = if rnd.gen_range(0..50) == 0 {
                // Occasionally a large blob
                (0..rnd.gen_range(2048..4096))
                    .map(|_| rnd.gen::<char>())
                    .collect::<String>()
            } else {
                (0..rnd.gen_range(0..10))
                    .map(|_| rnd.gen::<char>())
                    .collect::<String>()
            };
            let _ =dst.write_string(&value);
            |src| {}
        },
    ]
}

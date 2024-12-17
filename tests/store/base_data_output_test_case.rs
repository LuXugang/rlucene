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
use crate::util::test_error::TestError;
use rand::rngs::StdRng;
use rand::{Rng, RngCore};
use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoroshiro128Plus;
use rlucene::store::data_output::DataOutput;
use rlucene::store::output_stream_data_output::OutputStreamDataOutput;
use rlucene::store::DataInput;

pub trait BaseDataOutputTestCase {
    type DO: DataOutput;

    fn new_instance(&self) -> Result<Self::DO, TestError>;
    fn get_bytes(&mut self, instance: Self::DO) -> Vec<u8>;

    fn test_randomized_writes<DI: DataInput>(
        &mut self,
        random: &mut StdRng,
    ) -> Result<(), TestError> {
        let seed: u64 = random.gen();
        let mut instance = self.new_instance()?;
        let mut buffer = Vec::new();
        let mut os = OutputStreamDataOutput::new(&mut buffer);
        let max = 500000;
        let mut random1 = Xoroshiro128Plus::seed_from_u64(seed);
        let mut random2 = Xoroshiro128Plus::seed_from_u64(seed);

        add_random_data::<DI>(&mut instance, &mut random1, max);
        add_random_data::<DI>(&mut os, &mut random2, max);
        assert_eq!(&self.get_bytes(instance), os.os.into_inner().unwrap());
        Ok(())
    }
}

type DataInputProcessor<DI> = Box<dyn FnMut(&mut DI)>;

pub fn add_random_data<DI: DataInput>(
    dst: &mut impl DataOutput,
    rnd: &mut impl RngCore,
    max_add_calls: i32,
) -> Vec<DataInputProcessor<DI>> {
    let cg = create_generators();
    let mut vec: Vec<DataInputProcessor<DI>> = Vec::new();
    for _i in 0..max_add_calls {
        let random_generator = rnd.gen_range(0..cg.len());
        vec.push(cg[random_generator](dst, rnd));
    }
    vec
}
type Generator<DO, DI, R> = fn(&mut DO, &mut R) -> Box<dyn FnMut(&mut DI)>;

fn create_generators<DO: DataOutput, DI: DataInput, R: RngCore>() -> Vec<Generator<DO, DI, R>> {
    vec![
        //0 writeByte / readByte
        |dst, rnd| {
            let value: u8 = rnd.gen();
            let _ = dst.write_byte(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_byte().unwrap(), value, "Condition failed for DI")
            })
        },
        //1 writeBytes / readBytes (array and buffer version).
        |dst, rnd| {
            let len = rnd.gen_range(0..100);
            let bytes: Vec<u8> = (0..len).map(|_| rnd.gen()).collect();
            let bytes_len = bytes.len();
            let _ = dst.write_bytes_with_len(&bytes, bytes_len as u32);
            Box::new(move |src: &mut DI| {
                let mut buffer = vec![0u8; bytes_len];
                let _ = src.read_bytes(&mut buffer, 0, bytes_len as u32);
                assert_eq!(buffer, bytes, "Condition failed for DI")
            })
        },
        //2 writeBytes / readBytes (array + offset).
        |dst, rnd| {
            let len = rnd.gen_range(0..10000);
            let bytes: Vec<u8> = (0..len).map(|_| rnd.gen()).collect();
            let bytes_len = bytes.len();
            let off = if len == 0 {
                0
            } else {
                rnd.gen_range(0..bytes_len)
            };
            let length = if len == 0 {
                0
            } else {
                rnd.gen_range(0..(bytes_len - off))
            };
            let _ = dst.write_bytes_range(&bytes, off as u32, length as u32);
            Box::new(move |src: &mut DI| {
                let mut read: Vec<u8> = vec![0u8; bytes.len() + off];
                let _ = src.read_bytes(&mut read, off as u32, length as u32);
                assert_eq!(
                    read[off..off + length],
                    bytes[off..off + length],
                    "readBytes(byte[], off)"
                );
            })
        },
        //3 writeInt / readInt
        |dst, rnd| {
            let value: i32 = rnd.gen();
            let _ = dst.write_int(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_int().unwrap(), value, "readInt()");
            })
        },
        //4 writeLong / readInt
        |dst, rnd| {
            let value: i64 = rnd.gen();
            let _ = dst.write_long(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_long().unwrap(), value, "readLong()");
            })
        },
        //5 writeShort / readShort
        |dst, rnd| {
            let value: i16 = rnd.gen();
            let _ = dst.write_short(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_short().unwrap(), value, "readShort()");
            })
        },
        //6 writeVInt / readVInt
        |dst, rnd| {
            let value: i32 = rnd.gen();
            let _ = dst.write_vint(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_vint().unwrap(), value, "readVInt()");
            })
        },
        //7 writeZInt / readZInt
        |dst, rnd| {
            let value: i32 = rnd.gen();
            let _ = dst.write_zint(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_zint().unwrap(), value, "readZInt()");
            })
        },
        //8 writeZLong / readZLong
        |dst, rnd| {
            let value: i64 = rnd.gen();
            let _ = dst.write_zlong(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_zlong().unwrap(), value, "readZLong()");
            })
        },
        //9 writeVLong / readVLong
        |dst, rnd| {
            let mut value: i64 = rnd.gen();
            value &= (-1i64 as u64 >> 1) as i64;
            let _ = dst.write_vlong(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_vlong().unwrap(), value, "readVLong()");
            })
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
            let _ = dst.write_string(&value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_string().unwrap(), value, "readString()");
            })
        },
    ]
}

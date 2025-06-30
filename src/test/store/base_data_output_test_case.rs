/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use rand::Rng;
use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoroshiro128Plus;

use crate::store::data_output::DataOutput;
use crate::store::output_stream_data_output::OutputStreamDataOutput;
use crate::store::DataInput;
use crate::util::error::lucene_error::Result;

pub trait BaseDataOutputTestCase {
    type DO: DataOutput;

    fn new_instance(&self) -> Result<Self::DO>;
    fn get_bytes(&mut self, instance: Self::DO) -> Vec<u8>;

    fn test_randomized_writes<DI: DataInput, R: Rng + ?Sized>(
        &mut self,
        random: &mut R,
    ) -> Result<()> {
        let seed: u64 = random.random();
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
    rnd: &mut impl Rng,
    max_add_calls: i32,
) -> Vec<DataInputProcessor<DI>> {
    let cg = create_generators();
    let mut vec: Vec<DataInputProcessor<DI>> = Vec::new();
    for _i in 0..max_add_calls {
        let random_generator = rnd.random_range(0..cg.len());
        vec.push(cg[random_generator](dst, rnd));
    }
    vec
}
type Generator<DO, DI, R> = fn(&mut DO, &mut R) -> Box<dyn FnMut(&mut DI)>;

fn create_generators<DO: DataOutput, DI: DataInput, R: Rng>() -> Vec<Generator<DO, DI, R>> {
    vec![
        //0 writeByte / readByte
        |dst, rnd| {
            let value: u8 = rnd.random();
            let _ = dst.write_byte(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_byte().unwrap(), value, "Condition failed for DI")
            })
        },
        //1 writeBytes / readBytes (array and buffer version).
        |dst, rnd| {
            let len = rnd.random_range(0..100);
            let bytes: Vec<u8> = (0..len).map(|_| rnd.random()).collect();
            let bytes_len = bytes.len();
            let _ = dst.write_bytes_with_len(&bytes, bytes_len as i32);
            Box::new(move |src: &mut DI| {
                let mut buffer = vec![0u8; bytes_len];
                let _ = src.read_bytes(&mut buffer, 0, bytes_len as i32);
                assert_eq!(buffer, bytes, "Condition failed for DI")
            })
        },
        //2 writeBytes / readBytes (array + offset).
        |dst, rnd| {
            let len = rnd.random_range(0..10000);
            let bytes: Vec<u8> = (0..len).map(|_| rnd.random()).collect();
            let bytes_len = bytes.len();
            let off = if len == 0 {
                0
            } else {
                rnd.random_range(0..bytes_len)
            };
            let length = if len == 0 {
                0
            } else {
                rnd.random_range(0..(bytes_len - off))
            };
            let _ = dst.write_bytes_range(&bytes, off as i32, length as i32);
            Box::new(move |src: &mut DI| {
                let mut read: Vec<u8> = vec![0u8; bytes.len() + off];
                let _ = src.read_bytes(&mut read, off as i32, length as i32);
                assert_eq!(
                    read[off..off + length],
                    bytes[off..off + length],
                    "readBytes(byte[], off)"
                );
            })
        },
        //3 writeInt / readInt
        |dst, rnd| {
            let value: i32 = rnd.random();
            let _ = dst.write_int(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_int().unwrap(), value, "readInt()");
            })
        },
        //4 writeLong / readInt
        |dst, rnd| {
            let value: i64 = rnd.random();
            let _ = dst.write_long(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_long().unwrap(), value, "readLong()");
            })
        },
        //5 writeShort / readShort
        |dst, rnd| {
            let value: i16 = rnd.random();
            let _ = dst.write_short(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_short().unwrap(), value, "readShort()");
            })
        },
        //6 writeVInt / readVInt
        |dst, rnd| {
            let value: i32 = rnd.random();
            let _ = dst.write_vint(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_vint().unwrap(), value, "readVInt()");
            })
        },
        //7 writeZInt / readZInt
        |dst, rnd| {
            let value: i32 = rnd.random();
            let _ = dst.write_zint(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_zint().unwrap(), value, "readZInt()");
            })
        },
        //8 writeZLong / readZLong
        |dst, rnd| {
            let value: i64 = rnd.random();
            let _ = dst.write_zlong(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_zlong().unwrap(), value, "readZLong()");
            })
        },
        //9 writeVLong / readVLong
        |dst, rnd| {
            let mut value: i64 = rnd.random();
            value &= (-1i64 as u64 >> 1) as i64;
            let _ = dst.write_vlong(value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_vlong().unwrap(), value, "readVLong()");
            })
        },
        //10  writeString / readString
        |dst, rnd| {
            let value = if rnd.random_range(0..50) == 0 {
                // Occasionally a large blob
                (0..rnd.random_range(2048..4096))
                    .map(|_| rnd.random::<char>())
                    .collect::<String>()
            } else {
                (0..rnd.random_range(0..10))
                    .map(|_| rnd.random::<char>())
                    .collect::<String>()
            };
            let _ = dst.write_string(&value);
            Box::new(move |src: &mut DI| {
                assert_eq!(src.read_string().unwrap(), value, "readString()");
            })
        },
    ]
}

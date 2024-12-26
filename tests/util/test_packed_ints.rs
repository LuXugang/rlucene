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
use crate::util::lucene_test_case::{new_directory, new_io_context};
use crate::util::test_error::TestError;
use rand::Rng;
use rlucene::store::directory::Directory;
use rlucene::store::{DataOutput, IndexInput, IndexOutput};
use rlucene::util::packed::Format::{Packed, PackedSingleBlock};
use rlucene::util::packed::{create, is_supported, Format, FormatBehavior, Mutable, MutableImpl, MutablePacked64Enum, Packed64, PackedImpl, PackedInts, PackedSingleBlockImpl, Reader, ReaderIterator, Writer, MAX_SUPPORTED_BITS_PER_VALUE};

#[allow(dead_code)] // for quick search
struct TestPackedInts;
#[test]
fn test_byte_count() {
    let mut random = my_random("test_byte_count".to_string());
    const ITERATIONS: usize = 3;

    for _ in 0..ITERATIONS {
        let value_count = random.gen_range(1..i32::MAX) as u32;

        for format in &[
            Packed(PackedImpl::new(0)),
            PackedSingleBlock(PackedSingleBlockImpl::new(1)),
        ] {
            for bpv in 1..=64 {
                let byte_count = format.byte_count(PackedInts::VERSION_CURRENT, value_count, bpv);
                let msg = format!(
                    "format={:?}, byteCount={}, valueCount={}, bpv={}",
                    format, byte_count, value_count, bpv
                );
                assert!(
                    byte_count * 8 >= (value_count as u64) * (bpv as u64),
                    "{}",
                    msg
                );
                if let Packed(_) = format {
                    assert!(
                        (byte_count - 1) * 8 < (value_count as u64) * (bpv as u64),
                        "{}",
                        msg
                    );
                }
            }
        }
    }
}
#[test]
fn test_bits_required() -> Result<(), TestError> {
    assert_eq!(PackedInts::bits_required((2u64.pow(61) - 1) as i64)?, 61);
    assert_eq!(PackedInts::bits_required(0x1FFFFFFFFFFFFFFF)?, 61);
    assert_eq!(PackedInts::bits_required(0x3FFFFFFFFFFFFFFF)?, 62);
    assert_eq!(PackedInts::bits_required(0x7FFFFFFFFFFFFFFF)?, 63);
    assert_eq!(PackedInts::unsigned_bits_required(-1), 64);
    assert_eq!(PackedInts::unsigned_bits_required(i64::MIN), 64);
    assert_eq!(PackedInts::bits_required(0)?, 1);
    Ok(())
}
#[test]
fn test_max_values() {
    assert_eq!(PackedInts::max_value(1), 1, "1 bit -> max == 1");
    assert_eq!(PackedInts::max_value(2), 3, "2 bit -> max == 3");
    assert_eq!(PackedInts::max_value(8), 255, "8 bit -> max == 255");
    assert_eq!(
        PackedInts::max_value(63),
        i64::MAX as u64,
        "63 bit -> max == i64::MAX"
    );
    assert_eq!(
        PackedInts::max_value(64),
        i64::MAX as u64,
        "64 bit -> max == i64::MAX (same as for 63 bit)"
    );
}
#[test]
fn test_packed_ints() -> Result<(), TestError> {
    let mut random = my_random("test_packed_ints".to_string());
    let num = random.gen_range(3..500);
    for _ in 0..num {
        for nbits in 1..=64 {
            let max_value = PackedInts::max_value(nbits);
            let value_count = random.gen_range(1..=600);
            let buffer_size = if random.gen_bool(0.5) {
                random.gen_range(0..=48)
            } else {
                random.gen_range(0..=4096)
            };
            let mut directory = new_directory(&mut random)?;
            let mut values = vec![0i64; value_count];
            let mut fp = 0;
            {
                let mut out = directory.create_output("out.bin", new_io_context(&mut random)?)?;
                let mem = random.gen_range(0..2 * PackedInts::DEFAULT_BUFFER_SIZE);
                let start_fp = out.get_file_pointer();
                let mut writer = PackedInts::get_writer_no_header(
                    &mut out,
                    Packed(PackedImpl::new(0)),
                    value_count as i32,
                    nbits,
                    mem as u32,
                );

                let actual_value_count = if random.gen_bool(0.5) {
                    value_count
                } else {
                    random.gen_range(0..=value_count)
                };

                for i in 0..actual_value_count {
                    values[i] = if nbits == 64 {
                        random.gen()
                    } else {
                        random.gen_range(0..=max_value as i64)
                    };
                    writer.add(values[i])?;
                }
                writer.finish()?;

                // Ensure that finish() added the missing values
                let bytes = writer.get_format().byte_count(
                    PackedInts::VERSION_CURRENT,
                    value_count as u32,
                    writer.bits_per_value,
                );
                fp = out.get_file_pointer();
                assert_eq!(bytes, fp - start_fp);
            }

            // Test reader iterator `next`
            {
                let mut input = directory.open_input("out.bin", new_io_context(&mut random)?)?;
                {
                    let mut reader = PackedInts::get_reader_iterator_no_header(
                        &mut input,
                        Packed(PackedImpl::new(0)),
                        PackedInts::VERSION_CURRENT,
                        value_count as u32,
                        nbits,
                        buffer_size,
                    )?;
                    for i in 0..value_count {
                        let next_value = reader.next()?;
                        assert_eq!(
                            values[i], next_value,
                            "index={}, value_count={}, nbits={}, for reader {}",
                            i, value_count, nbits, reader
                        );
                        assert_eq!(i, reader.ord() as usize);
                    }
                }
                assert_eq!(fp, input.get_file_pointer());
            }

            // Test reader iterator bulk `next`
            {
                let mut input = directory.open_input("out.bin", new_io_context(&mut random)?)?;
                {
                    let mut reader = PackedInts::get_reader_iterator_no_header(
                        &mut input,
                        Packed(PackedImpl::new(0)),
                        PackedInts::VERSION_CURRENT,
                        value_count as u32,
                        nbits as u32,
                        buffer_size,
                    )?;
                    let mut i = 0;
                    while i < value_count {
                        let count = random.gen_range(1..=95);
                        let next = reader.next_batch(count)?;
                        for k in 0..next.length {
                            assert_eq!(
                                values[i + k],
                                next.longs[next.offset + k],
                                "index={}, value_count={}, nbits={}",
                                i,
                                value_count,
                                nbits
                            );
                        }
                        i += next.length;
                    }
                }
                assert_eq!(fp, input.get_file_pointer());
            }
        }
    }
    Ok(())
}
#[test]
fn test_end_pointer() -> Result<(), TestError> {
    let mut random = my_random("test_end_pointer".to_string());

    let mut directory = new_directory(&mut random)?;
    let value_count = random.gen_range(1..=1000);

    {
        let mut out = directory.create_output("tests.bin", new_io_context(&mut random)?)?;
        for _ in 0..value_count {
            out.write_long(0)?
        }
    }

    let mut input = directory.open_input("tests.bin", new_io_context(&mut random)?)?;

    for version in PackedInts::VERSION_START..=PackedInts::VERSION_CURRENT {
        for bpv in 1..=64 {
            for format in &[
                Packed(PackedImpl::new(0)),
                PackedSingleBlock(PackedSingleBlockImpl::new(1)),
            ] {
                if !format.is_supported(bpv) {
                    continue;
                }

                let byte_count = format.byte_count(version, value_count as u32, bpv);

                let msg = format!(
                    "format={:?}, version={}, value_count={}, bpv={}",
                    format, version, value_count, bpv
                );

                // 测试迭代器
                input.seek(0)?;
                {
                    let mut iterator = PackedInts::get_reader_iterator_no_header(
                        &mut input,
                        format.clone(),
                        version,
                        value_count as u32,
                        bpv,
                        random.gen_range(1..=65536), // 缓冲区大小随机
                    )?;

                    for _ in 0..value_count {
                        iterator.next()?;
                    }
                }

                assert_eq!(
                    byte_count,
                    input.get_file_pointer(),
                    "{}: File pointer mismatch",
                    msg
                );
            }
        }
    }
    Ok(())
}
#[test]
fn test_controlled_equality() -> Result<(), TestError> {
    const VALUE_COUNT: usize = 255;
    const BITS_PER_VALUE: u32 = 8;

    let mut packed_ints = create_packed_ints(VALUE_COUNT as u32, BITS_PER_VALUE)?;

    
    for mut packed_int in packed_ints {
        for i in 0..packed_int.size() {
            packed_int.set(i as usize, (i + 1) as i64);
        }
    }
    let mut packed_ints = create_packed_ints(VALUE_COUNT as u32, BITS_PER_VALUE)?;

    assert_list_equality(&mut packed_ints)?;

    Ok(())
}
#[test]
fn random_test() -> Result<(), TestError> {
    for _i in 0..200{
        test_random_bulk_copy()?
    }
    Ok(())
}

#[test]
fn test_random_bulk_copy() -> Result<(), TestError> {
    let mut random = my_random("test_random_bulk_copy".to_string());
    let num_iters = random.gen_range(3..10);

    for j in 0..num_iters {
        let value_count = if is_night_mode(){
            random.gen_range(100000..200000)
        } else {
            random.gen_range(10000..20000)
        };

        let mut bits1 = random.gen_range(1..=64);
        let mut bits2 = random.gen_range(1..=64);


        if bits1 > bits2 {
            std::mem::swap(&mut bits1, &mut bits2);
        }

        let mut packed1 = PackedInts::get_mutable(value_count as u32, bits1, PackedInts::COMPACT)?;
        let mut packed2 = PackedInts::get_mutable(value_count as u32, bits2, PackedInts::COMPACT)?;

        let max_value = PackedInts::max_value(bits1);
        for i in 0..value_count {
            let val = random.gen_range(0..=max_value as i64);
            packed1.set(i, val );
            packed2.set(i, val );
        }

        let mut buffer = vec![0i64; value_count];

        // Copy random slices over 20 times:
        for _ in 0..20 {
            let start = random.gen_range(0..value_count - 1);
            let len = random.gen_range(1..=(value_count - start));
            let offset = if len == value_count {
                0
            } else {
                random.gen_range(0..(value_count - len))
            };

            if random.gen_bool(0.5) {
                let got = packed1.get_bulk(start, &mut buffer, offset, len)?;
                assert!(got as usize<= len);
                let sot = packed2.set_bulk(start, &buffer, offset, got as usize);
                assert!(sot <= got);
            } else {
                PackedInts::copy(
                    &mut packed1,
                    offset,
                    &mut packed2,
                    offset,
                    len,
                    random.gen_range(1..=(10 * len)) as u32,
                )?;
            }
        }

        for i in 0..value_count {
            assert_eq!(
                packed1.get(i)?,
                packed2.get(i)?,
                "Values at index {} differ, iter:{}",
                i,j
            );
        }
    }
    Ok(())
}








fn create_packed_ints(value_count: u32, bits_per_value: u32) -> Result<Vec<MutablePacked64Enum>, TestError> {
    let mut packed_ints: Vec<MutablePacked64Enum> = Vec::new();
    let packed64 = Packed64::new(value_count, bits_per_value);
    let mutable_impl = MutableImpl::new(packed64,value_count, bits_per_value);
    packed_ints.push(MutablePacked64Enum::P64(mutable_impl));

    for bpv in bits_per_value..=MAX_SUPPORTED_BITS_PER_VALUE {
        if is_supported(bpv) {
            packed_ints.push(create(value_count, bpv)?);
        }
    }

    Ok(packed_ints)
}
fn assert_list_equality(
    packed_ints: &mut [MutablePacked64Enum],
) -> Result<(), TestError> {
   assert_list_equality_impl("", packed_ints) 
}
fn assert_list_equality_impl(
    message: &str,
    packed_ints: &mut [MutablePacked64Enum],
) -> Result<(), TestError> {
    if packed_ints.is_empty() {
        return Ok(());
    }
    let mut length = 0;
    let mut value_count =0;
    {
        length = packed_ints.len();
        let mut base = &mut packed_ints[0];
        value_count = base.size();
        for packed_int in packed_ints.iter() {
            assert_eq!(
                value_count,
                packed_int.size(),
                "{}. The number of values should be the same",
                message
            );
        }
    }

    for i in 0..value_count {
        for j in 1..length {
            assert_eq!(
                packed_ints[0].get(i as usize)?,
                packed_ints[j].get(i as usize)?,
                "{}. The value at index {} should be the same ",
                message,
                i
            );
        }
    }

    Ok(())
}
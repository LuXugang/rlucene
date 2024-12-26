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
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rlucene::store::directory::Directory;
use rlucene::store::{DataOutput, IndexInput, IndexOutput};
use rlucene::util::packed::growable_writer::GrowableWriter;
use rlucene::util::packed::Format::{Packed, PackedSingleBlock};
use rlucene::util::packed::{
    create, is_supported, FormatBehavior, Mutable, MutableImpl, MutablePacked64Enum, NullReader,
    Packed64, PackedImpl, PackedInts, PackedSingleBlockImpl, Reader, ReaderIterator, Writer,
    MAX_SUPPORTED_BITS_PER_VALUE,
};

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
        i64::MAX,
        "63 bit -> max == i64::MAX"
    );
    assert_eq!(
        PackedInts::max_value(64),
        i64::MAX,
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
            let fp: u64;
            {
                let mut out = directory.create_output("out.bin", new_io_context(&mut random)?)?;
                let mem = random.gen_range(0..2 * PackedInts::DEFAULT_BUFFER_SIZE);
                let start_fp = out.get_file_pointer();
                let mut writer = PackedInts::get_writer_no_header(
                    &mut out,
                    Packed(PackedImpl::new(0)),
                    value_count as i32,
                    nbits,
                    mem,
                );

                let actual_value_count = if random.gen_bool(0.5) {
                    value_count
                } else {
                    random.gen_range(0..=value_count)
                };

                values
                    .iter_mut()
                    .take(actual_value_count)
                    .enumerate()
                    .try_for_each(|(_, value)| {
                        let val = if nbits == 64 {
                            random.gen()
                        } else {
                            random.gen_range(0..=max_value)
                        };
                        *value = val;
                        writer.add(val)
                    })?;
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
                    for (i, &expected_value) in values.iter().enumerate().take(value_count) {
                        let next_value = reader.next()?;
                        assert_eq!(
                            expected_value, next_value,
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
                        nbits,
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
                        *format,
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

    let packed_ints = create_packed_ints(VALUE_COUNT as u32, BITS_PER_VALUE)?;

    for mut packed_int in packed_ints {
        for i in 0..packed_int.size() {
            packed_int.set(i as usize, (i + 1) as i64)?;
        }
    }
    let mut packed_ints = create_packed_ints(VALUE_COUNT as u32, BITS_PER_VALUE)?;

    assert_list_equality(&mut packed_ints)?;

    Ok(())
}
#[test]
fn test_random_bulk_copy() -> Result<(), TestError> {
    let mut random = my_random("test_random_bulk_copy".to_string());
    let num_iters = random.gen_range(3..10);

    for j in 0..num_iters {
        let value_count = if is_night_mode() {
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
            let val = random.gen_range(0..=max_value);
            packed1.set(i, val)?;
            packed2.set(i, val)?;
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
                assert!(got as usize <= len);
                let sot = packed2.set_bulk(start, &buffer, offset, got as usize)?;
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
                i,
                j
            );
        }
    }
    Ok(())
}
#[test]
fn test_random_equality() -> Result<(), TestError> {
    let mut random = my_random("test_random_equality".to_string());
    let num_iters = if is_night_mode() {
        random.gen_range(2..=5)
    } else {
        1
    };
    for _ in 0..num_iters {
        let value_count = random.gen_range(1..=300);
        for bits_per_value in 1..=64 {
            assert_random_equality(value_count, bits_per_value, random.gen::<u64>())?;
        }
    }
    Ok(())
}
fn assert_random_equality(
    value_count: u32,
    bits_per_value: u32,
    random: u64,
) -> Result<(), TestError> {
    let mut packed_ints = create_packed_ints(value_count, bits_per_value)?;

    for packed_int in &mut packed_ints {
        fill(packed_int, bits_per_value, random)?;
    }

    assert_list_equality(&mut packed_ints)?;

    Ok(())
}

fn create_packed_ints(
    value_count: u32,
    bits_per_value: u32,
) -> Result<Vec<MutablePacked64Enum>, TestError> {
    let mut packed_ints: Vec<MutablePacked64Enum> = Vec::new();
    let packed64 = Packed64::new(value_count, bits_per_value);
    let mutable_impl = MutableImpl::new(packed64);
    packed_ints.push(MutablePacked64Enum::P64(mutable_impl));

    for bpv in bits_per_value..=MAX_SUPPORTED_BITS_PER_VALUE {
        if is_supported(bpv) {
            packed_ints.push(create(value_count, bpv)?);
        }
    }

    Ok(packed_ints)
}

fn fill(
    packed_int: &mut MutablePacked64Enum,
    bits_per_value: u32,
    seed: u64,
) -> Result<(), TestError> {
    let max_value = if bits_per_value == 64 {
        i64::MAX
    } else {
        (1 << bits_per_value) - 1
    };
    let mut random = StdRng::seed_from_u64(seed);
    for i in 0..packed_int.size() as usize {
        let value: i64 = if bits_per_value == 64 {
            random.gen()
        } else {
            random.gen_range(0..=max_value)
        };

        packed_int.set(i, value)?;
        let retrieved_value = packed_int.get(i)?;

        if value != retrieved_value {
            assert_eq!(
                value, retrieved_value,
                "The set/get of the value at index {} should match for {}",
                i, packed_int
            );
        }
    }

    Ok(())
}

fn assert_list_equality(packed_ints: &mut [MutablePacked64Enum]) -> Result<(), TestError> {
    assert_list_equality_impl("", packed_ints)
}
fn assert_list_equality_impl(
    message: &str,
    packed_ints: &mut [MutablePacked64Enum],
) -> Result<(), TestError> {
    if packed_ints.is_empty() {
        return Ok(());
    }
    let length: usize;
    let value_count: u32;
    {
        length = packed_ints.len();
        let base = &mut packed_ints[0];
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
#[test]
fn test_secondary_block_change() -> Result<(), TestError> {
    let mut mutable = MutablePacked64Enum::P64(MutableImpl::new(Packed64::new(26, 5)));
    mutable.set(24, 31)?;
    assert_eq!(mutable.get(24)?, 31, "The value #24 should be correct");
    mutable.set(4, 16)?;
    assert_eq!(
        mutable.get(24)?,
        31,
        "The value #24 should remain unchanged"
    );

    Ok(())
}
#[test]
fn test_int_overflow() -> Result<(), TestError> {
    Ok(())
}
#[test]
fn test_fill() -> Result<(), TestError> {
    let mut random = my_random("test_fill".to_string());
    let value_count = 1111;
    let from = random.gen_range(0..value_count + 1);
    let to = from + random.gen_range(0..value_count + 1 - from);

    for bpv in 1..=64 {
        let val = random.gen_range(0..=PackedInts::max_value(bpv));
        let mut packed_ints = create_packed_ints(value_count as u32, bpv)?;

        for packed in &mut packed_ints {
            let msg = format!(
                "{} bpv={}, from={}, to={}, val={}",
                packed, bpv, from, to, val
            );

            packed.fill(0, packed.size() as usize, 1)?;
            packed.fill(from, to, val)?;

            for i in 0..packed.size() as usize {
                let expected_value: i64 = if i >= from && i < to { val } else { 1 };

                assert_eq!(packed.get(i)?, expected_value, "{}: i={}", msg, i);
            }
        }
    }

    Ok(())
}
#[test]
fn test_packed_ints_null() -> Result<(), TestError> {
    let mut random = my_random("test_packed_ints_null".to_string());
    // must be > 10 for the bulk reads below
    let size = random.gen_range(11..=256);
    let mut packed_ints = NullReader::for_count(size);
    let random_index = random.gen_range(0..size);
    assert_eq!(
        packed_ints.get(random_index as usize)?,
        0,
        "The value at random index {} should be 0",
        random_index
    );
    let mut arr = vec![1i64; (size + 10) as usize];
    let r = packed_ints.get_bulk(0, &mut arr, 0, (size - 1) as usize)?;
    assert_eq!(
        r,
        (size - 1),
        "The number of values read should match size - 1"
    );
    for (i, &value) in arr.iter().take(r as usize).enumerate() {
        assert_eq!(value, 0, "The value at position {} should be 0", i);
    }
    arr.fill(1);
    let r = packed_ints.get_bulk(10, &mut arr, 0, (size + 10) as usize)?;
    assert_eq!(
        r,
        (size - 10),
        "The number of values read should match size - 10"
    );
    for i in 0..(size - 10) {
        assert_eq!(
            arr[i as usize], 0,
            "The value at position {} should be 0",
            i
        );
    }

    Ok(())
}
#[test]
fn test_bulk_get() -> Result<(), TestError> {
    let mut random = my_random("test_bulk_get".to_string());
    let value_count = 1111;
    let index = random.gen_range(0..value_count);
    let len = random.gen_range(1..=(value_count * 2));
    let off = random.gen_range(0..77);

    for bpv in 1..=64 {
        let mask = PackedInts::max_value(bpv);
        let mut packed_ints = create_packed_ints(value_count as u32, bpv)?;
        for ints in &mut packed_ints {
            for i in 0..ints.size() as usize {
                ints.set(i, (31 * i as i64 - 1099) & mask)?;
            }
            let mut arr = vec![0i64; off + len];
            let msg = format!(
                "{} valueCount={}, index={}, len={}, off={}",
                ints, value_count, index, len, off
            );
            let gets = ints.get_bulk(index, &mut arr, off, len)?;
            assert!(gets > 0, "{}: gets should be greater than 0", msg);
            assert!(
                gets <= len as u32,
                "{}: gets should be less than or equal to len",
                msg
            );
            assert!(
                gets <= (ints.size() - index as u32),
                "{}: gets should be less than or equal to remaining values",
                msg
            );
            for (i, &item) in arr.iter().enumerate() {
                let m = format!("{}, i={}", msg, i);
                if i >= off && i < off + gets as usize {
                    assert_eq!(
                        ints.get(i - off + index)?,
                        item,
                        "{}: value mismatch at index {}",
                        m,
                        i
                    );
                } else {
                    assert_eq!(item, 0, "{}: array values outside range should be 0", m);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn test_bulk_set() -> Result<(), TestError> {
    let mut random = my_random("test_bulk_get".to_string());
    let value_count = 1111;
    let index = random.gen_range(0..value_count);
    let len = random.gen_range(1..=(value_count * 2));
    let off = random.gen_range(0..77);

    for bpv in 1..=64 {
        let mask = PackedInts::max_value(bpv);
        let mut packed_ints = create_packed_ints(value_count as u32, bpv)?;
        let mut arr = vec![0i64; off + len];
        let length = arr.len();
        for (i, item) in arr.iter_mut().enumerate().take(length) {
            *item = (31 * i as i64 + 19) & mask;
        }
        for ints in &mut packed_ints {
            let msg = format!(
                "{} valueCount={}, index={}, len={}, off={}",
                ints, value_count, index, len, off
            );
            let sets = ints.set_bulk(index, &arr, off, len)?;
            assert!(sets > 0, "{}: gets should be greater than 0", msg);
            assert!(
                sets <= len as u32,
                "{}: gets should be less than or equal to len",
                msg
            );
            for i in 0..ints.size() as usize {
                let m = format!("{}, i={}", msg, i);
                if i >= index && i < index + sets as usize {
                    assert_eq!(
                        ints.get(i)?,
                        arr[off - index + i],
                        "{}: value mismatch at index {}",
                        m,
                        i
                    );
                } else {
                    assert_eq!(
                        ints.get(i)?,
                        0,
                        "{}: array values outside range should be 0",
                        m
                    );
                }
            }
        }
    }
    Ok(())
}
#[test]
fn test_copy() -> Result<(), TestError> {
    let mut random = my_random("test_copy".to_string());
    let value_count = random.gen_range(5..=600);
    let off1 = random.gen_range(0..value_count);
    let off2 = random.gen_range(0..value_count);
    let len = random.gen_range(0..(value_count - off1).min(value_count - off2));
    let mem = random.gen_range(0..1024);
    for bpv in 1..=64 {
        let mask = PackedInts::max_value(bpv);
        for mut r1 in create_packed_ints(value_count as u32, bpv)? {
            for i in 0..r1.size() as usize {
                r1.set(i, (31 * i as i64 - 1023) & mask)?;
            }
            for mut r2 in create_packed_ints(value_count as u32, bpv)? {
                let msg = format!(
                    "src={}, dest={}, srcPos={}, destPos={}, len={}, mem={}",
                    r1, r2, off1, off2, len, mem
                );
                PackedInts::copy(&mut r1, off1, &mut r2, off2, len, mem)?;
                for i in 0..r2.size() as usize {
                    let m = format!("{}, i={}", msg, i);
                    if i >= off2 && i < off2 + len {
                        assert_eq!(
                            r1.get(i - off2 + off1)?,
                            r2.get(i)?,
                            "{}: Values mismatch at index {}",
                            m,
                            i
                        );
                    } else {
                        assert_eq!(
                            r2.get(i)?,
                            0,
                            "{}: Unexpected non-zero value at index {}",
                            m,
                            i
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn test_growable_writer() -> Result<(), TestError> {
    let mut random = my_random("test_growable_writer".to_string());
    let value_count = 113 + random.gen_range(0..1112);

    let mut wrt = GrowableWriter::new(1, value_count as u32, PackedInts::DEFAULT)?;

    wrt.set(4, 2)?;
    wrt.set(7, 10)?;
    wrt.set(value_count - 10, 99)?;
    wrt.set(99, 999)?;
    wrt.set(value_count - 1, 1 << 10)?;
    assert_eq!(wrt.get(value_count - 1)?, 1 << 10);

    wrt.set(99, (1 << 23) - 1)?;
    assert_eq!(wrt.get(value_count - 1)?, 1 << 10);

    wrt.set(1, i64::MAX)?;
    wrt.set(2, -3)?;
    assert_eq!(wrt.get_bits_per_value(), 64);
    assert_eq!(wrt.get(value_count - 1)?, 1 << 10);
    assert_eq!(wrt.get(1)?, i64::MAX);
    assert_eq!(wrt.get(2)?, -3);
    assert_eq!(wrt.get(4)?, 2);
    assert_eq!(wrt.get(99)?, (1 << 23) - 1);
    assert_eq!(wrt.get(7)?, 10);
    assert_eq!(wrt.get(value_count - 10)?, 99);
    assert_eq!(wrt.get(value_count - 1)?, 1 << 10);

    // TODO:
    // Check memory usage
    // let ram_used = wrt.ram_bytes_used();
    // assert_eq!(ram_used, ram_usage(&wrt));

    Ok(())
}

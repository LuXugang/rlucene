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

use crate::test_framework::core::util::lucene_test_case::{
  at_least, is_night_mode, new_directory_shared, new_io_context, random, random_from_seed, rarely,
};
use rand::RngExt;

use crate::core::store::directory::Directory;
use crate::core::store::{
  ByteArrayDataInput, DataInput, DataOutput, IO_CONTEXT_DEFAULT, IndexInput, IndexOutput,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::long_values::LongValues;
use crate::core::util::packed::Format::{Packed, PackedSingleBlock};
use crate::core::util::packed::abstract_block_packed_writer::AbstractBlockPackedWriter;
use crate::core::util::packed::abstract_paged_mutable::AbstractPagedMutable;
use crate::core::util::packed::block_packed_reader_iterator::BlockPackedReaderIterator;
use crate::core::util::packed::block_packed_writer::BlockPackedWriter;
use crate::core::util::packed::growable_writer::GrowableWriter;
use crate::core::util::packed::monotonic_block_packed_reader::MonotonicBlockPackedReader;
use crate::core::util::packed::monotonic_block_packed_writer::MonotonicBlockPackedWriter;
use crate::core::util::packed::mutable_packed64_enum::MutablePacked64Enum;
use crate::core::util::packed::packed_long_values::{Builder, PackedLongValues};
use crate::core::util::packed::packed64::Packed64;
use crate::core::util::packed::paged_growable_writer::PagedGrowableWriter;
use crate::core::util::packed::paged_mutable::PagedMutable;
use crate::core::util::packed::{
  Decoder, Encoder, FormatBehavior, MAX_SUPPORTED_BITS_PER_VALUE, Mutable, NullReader, PackedImpl,
  PackedInts, PackedSingleBlockImpl, Reader, ReaderIterator, Writer, create, is_supported,
};
use crate::core::util::{SliceCopyOps, TryIntoInt};
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestPackedInts;
#[test]
fn test_byte_count() {
  let mut random = random();
  const ITERATIONS: usize = 3;

  for _ in 0..ITERATIONS {
    let value_count = random.random_range(1..i32::MAX);

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
          byte_count * 8 >= (value_count as i64) * (bpv as i64),
          "{}",
          msg
        );
        if let Packed(_) = format {
          assert!(
            (byte_count - 1) * 8 < (value_count as i64) * (bpv as i64),
            "{}",
            msg
          );
        }
      }
    }
  }
}
#[test]
fn test_bits_required() -> Result<()> {
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
fn test_packed_ints() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 3);
  let io_context = new_io_context(&mut random)?;
  for _ in 0..num {
    for nbits in 1..=64 {
      let max_value = PackedInts::max_value(nbits);
      let value_count = TestUtil::next_usize(&mut random, 1, 600);
      let buffer_size = if random.random_bool(0.5) {
        TestUtil::next_int(&mut random, 0, 48)
      } else {
        TestUtil::next_int(&mut random, 0, 4096)
      };
      let directory = new_directory_shared(&mut random)?;
      let mut values = vec![0i64; value_count];
      let fp: i64;
      {
        let mut out = directory.create_output("out.bin", &io_context)?;
        let mem = random.random_range(0..2 * PackedInts::DEFAULT_BUFFER_SIZE);
        let start_fp = out.get_file_pointer()? as i64;
        let mut writer = PackedInts::get_writer_no_header(
          &mut out,
          Packed(PackedImpl::new(0)),
          value_count as i32,
          nbits,
          mem,
        );

        let actual_value_count = if random.random_bool(0.5) {
          value_count
        } else {
          TestUtil::next_usize(&mut random, 0, value_count)
        };

        values
          .iter_mut()
          .take(actual_value_count)
          .enumerate()
          .try_for_each(|(_, value)| {
            let val = if nbits == 64 {
              random.random()
            } else {
              TestUtil::next_long(&mut random, 0, max_value)
            };
            *value = val;
            writer.add(val)
          })?;
        writer.finish()?;

        // Ensure that finish() added the missing values
        let bytes = writer.get_format().byte_count(
          PackedInts::VERSION_CURRENT,
          value_count as i32,
          writer.bits_per_value,
        );
        fp = out.get_file_pointer()? as i64;
        assert_eq!(bytes, fp - start_fp);
      }

      // Test reader iterator `next`
      {
        let mut input = directory.open_input("out.bin", &io_context)?;
        {
          let mut reader = PackedInts::get_reader_iterator_no_header(
            &mut input,
            Packed(PackedImpl::new(0)),
            PackedInts::VERSION_CURRENT,
            value_count as i32,
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
        assert_eq!(fp, input.get_file_pointer()? as i64);
      }

      // Test reader iterator bulk `next`
      {
        let mut input = directory.open_input("out.bin", &io_context)?;
        {
          let mut reader = PackedInts::get_reader_iterator_no_header(
            &mut input,
            Packed(PackedImpl::new(0)),
            PackedInts::VERSION_CURRENT,
            value_count as i32,
            nbits,
            buffer_size,
          )?;
          let mut i = 0;
          while i < value_count {
            let count = TestUtil::next_int(&mut random, 1, 95);
            let next = reader.next_batch(count)?;
            for k in 0..next.length {
              assert_eq!(
                values[i + k],
                next.longs[(next.offset + k) as usize],
                "index={}, value_count={}, nbits={}",
                i,
                value_count,
                nbits
              );
            }
            i += next.length as usize;
          }
        }
        assert_eq!(fp, input.get_file_pointer()? as i64);
      }
    }
  }
  Ok(())
}
#[test]
fn test_end_pointer() -> Result<()> {
  let mut random = random();

  let directory = new_directory_shared(&mut random)?;
  let value_count = random.random_range(1..=1000);
  let io_context = new_io_context(&mut random)?;

  {
    let mut out = directory.create_output("tests.bin", &io_context)?;
    for _ in 0..value_count {
      out.write_long(0)?
    }
  }

  let mut input = directory.open_input("tests.bin", &io_context)?;

  for version in PackedInts::VERSION_START..=PackedInts::VERSION_CURRENT {
    for bpv in 1..=64 {
      for format in &[
        Packed(PackedImpl::new(0)),
        PackedSingleBlock(PackedSingleBlockImpl::new(1)),
      ] {
        if !format.is_supported(bpv) {
          continue;
        }

        let byte_count = format.byte_count(version, value_count, bpv);

        let msg = format!(
          "format={:?}, version={}, value_count={}, bpv={}",
          format, version, value_count, bpv
        );

        input.seek(0)?;
        {
          let mut iterator = PackedInts::get_reader_iterator_no_header(
            &mut input,
            *format,
            version,
            value_count,
            bpv,
            random.random_range(1..=65536), /* 缓冲区大小随机  */
          )?;

          for _ in 0..value_count {
            iterator.next()?;
          }
        }

        assert_eq!(
          byte_count,
          input.get_file_pointer()? as i64,
          "{}: File pointer mismatch",
          msg
        );
      }
    }
  }
  Ok(())
}
#[test]
fn test_controlled_equality() -> Result<()> {
  const VALUE_COUNT: i32 = 255;
  const BITS_PER_VALUE: i32 = 8;

  let packed_ints = create_packed_ints(VALUE_COUNT, BITS_PER_VALUE)?;

  for mut packed_int in packed_ints {
    for i in 0..packed_int.size() {
      packed_int.set(i, (i + 1) as i64);
    }
  }
  let mut packed_ints = create_packed_ints(VALUE_COUNT, BITS_PER_VALUE)?;

  assert_list_equality(&mut packed_ints)?;

  Ok(())
}
#[test]
fn test_random_bulk_copy() -> Result<()> {
  let mut random = random();
  let num_iters = at_least(&mut random, 3);

  for j in 0..num_iters {
    let value_count = if is_night_mode() {
      at_least(&mut random, 100000)
    } else {
      at_least(&mut random, 10000)
    };

    let mut bits1 = TestUtil::next_int(&mut random, 1, 64);
    let mut bits2 = TestUtil::next_int(&mut random, 1, 64);

    if bits1 > bits2 {
      std::mem::swap(&mut bits1, &mut bits2);
    }

    let mut packed1 = PackedInts::get_mutable(value_count, bits1, PackedInts::COMPACT);
    let mut packed2 = PackedInts::get_mutable(value_count, bits2, PackedInts::COMPACT);

    let max_value = PackedInts::max_value(bits1);
    for i in 0..value_count {
      let val = TestUtil::next_long(&mut random, 0, max_value);
      packed1.set(i, val);
      packed2.set(i, val);
    }

    let mut buffer = vec![0i64; value_count as usize];

    // Copy random slices over 20 times:
    for _ in 0..20 {
      let start = random.random_range(0..value_count - 1);
      let len = TestUtil::next_int(&mut random, 1, value_count - start);
      let offset = if len == value_count {
        0
      } else {
        random.random_range(0..(value_count - len))
      };

      if random.random_bool(0.5) {
        let got = packed1.get_bulk(start, &mut buffer, offset, len);
        assert!(got <= len);
        let sot = packed2.set_bulk(start, &buffer, offset, got);
        assert!(sot <= got);
      } else {
        PackedInts::copy(
          &mut packed1,
          offset,
          &mut packed2,
          offset,
          len,
          random.random_range(1..=(10 * len)),
        );
      }
    }

    for i in 0..value_count as usize {
      assert_eq!(
        packed1.get(i),
        packed2.get(i),
        "Values at index {} differ, iter:{}",
        i,
        j
      );
    }
  }
  Ok(())
}
#[test]
fn test_random_equality() -> Result<()> {
  let mut random = random();
  let num_iters = if is_night_mode() {
    at_least(&mut random, 10)
  } else {
    1
  };
  for _ in 0..num_iters {
    let value_count = TestUtil::next_int(&mut random, 1, 300);
    for bits_per_value in 1..=64 {
      assert_random_equality(value_count, bits_per_value, random.random::<u64>())?;
    }
  }
  Ok(())
}
fn assert_random_equality(value_count: i32, bits_per_value: i32, random: u64) -> Result<()> {
  let mut packed_ints = create_packed_ints(value_count, bits_per_value)?;

  for packed_int in &mut packed_ints {
    fill(packed_int, bits_per_value, random)?;
  }

  assert_list_equality(&mut packed_ints)?;

  Ok(())
}

fn create_packed_ints(value_count: i32, bits_per_value: i32) -> Result<Vec<MutablePacked64Enum>> {
  let mut packed_ints: Vec<MutablePacked64Enum> = Vec::new();
  let packed64 = Packed64::new(value_count, bits_per_value);
  packed_ints.push(MutablePacked64Enum::P64(packed64));

  for bpv in bits_per_value..=MAX_SUPPORTED_BITS_PER_VALUE {
    if is_supported(bpv) {
      packed_ints.push(create(value_count, bpv));
    }
  }

  Ok(packed_ints)
}

fn fill(packed_int: &mut MutablePacked64Enum, bits_per_value: i32, seed: u64) -> Result<()> {
  let max_value = PackedInts::max_value(bits_per_value);
  let mut random = random_from_seed(seed);
  for i in 0..packed_int.size() {
    let value: i64 = if bits_per_value == 64 {
      random.random()
    } else {
      TestUtil::next_long(&mut random, 0, max_value)
    };

    packed_int.set(i, value);
    let retrieved_value = packed_int.get(i as usize);

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

fn assert_list_equality(packed_ints: &mut [MutablePacked64Enum]) -> Result<()> {
  assert_list_equality_impl("", packed_ints)
}
fn assert_list_equality_impl(message: &str, packed_ints: &mut [MutablePacked64Enum]) -> Result<()> {
  if packed_ints.is_empty() {
    return Ok(());
  }
  let length: usize;
  let value_count: i32;
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

  for i in 0..value_count as usize {
    for j in 1..length {
      assert_eq!(
        packed_ints[0].get(i),
        packed_ints[j].get(i),
        "{}. The value at index {} should be the same ",
        message,
        i
      );
    }
  }

  Ok(())
}
#[test]
fn test_secondary_block_change() -> Result<()> {
  let mut mutable = MutablePacked64Enum::P64(Packed64::new(26, 5));
  mutable.set(24, 31);
  assert_eq!(mutable.get(24), 31, "The value #24 should be correct");
  mutable.set(4, 16);
  assert_eq!(mutable.get(24), 31, "The value #24 should remain unchanged");

  Ok(())
}
#[test]
#[ignore = "See LUCENE-4488"]
fn test_int_overflow() -> Result<()> {
  // TODO:
  Ok(())
}
#[test]
fn test_fill() -> Result<()> {
  let mut random = random();
  let value_count = 1111;
  let from = random.random_range(0..value_count + 1);
  let to = from + random.random_range(0..value_count + 1 - from);

  for bpv in 1..=64 {
    let val = TestUtil::next_long(&mut random, 0, PackedInts::max_value(bpv));
    let mut packed_ints = create_packed_ints(value_count, bpv)?;

    for packed in &mut packed_ints {
      let msg = format!(
        "{} bpv={}, from={}, to={}, val={}",
        packed, bpv, from, to, val
      );

      packed.fill(0, packed.size(), 1);
      packed.fill(from, to, val);

      for i in 0..packed.size() {
        let expected_value: i64 = if i >= from && i < to { val } else { 1 };

        assert_eq!(packed.get(i as usize), expected_value, "{}: i={}", msg, i);
      }
    }
  }

  Ok(())
}
#[test]
fn test_packed_ints_null() -> Result<()> {
  let mut random = random();
  // must be > 10 for the bulk reads below
  let size = TestUtil::next_int(&mut random, 11, 256);
  let packed_ints = NullReader::for_count(size);
  let random_index = TestUtil::next_int(&mut random, 0, size - 1).try_convert()?;
  assert_eq!(
    packed_ints.get(random_index),
    0,
    "The value at random index {} should be 0",
    random_index
  );
  let mut arr = vec![1i64; (size + 10) as usize];
  let r = packed_ints.get_bulk(0, &mut arr, 0, size - 1);
  assert_eq!(
    r,
    size - 1,
    "The number of values read should match size - 1"
  );
  for (i, &value) in arr.iter().take(r as usize).enumerate() {
    assert_eq!(value, 0, "The value at position {} should be 0", i);
  }
  arr.fill(1);
  let r = packed_ints.get_bulk(10, &mut arr, 0, size + 10);
  assert_eq!(
    r,
    size - 10,
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
fn test_bulk_get() -> Result<()> {
  let mut random = random();
  let value_count = 1111;
  let index = random.random_range(0..value_count);
  let len = TestUtil::next_usize(&mut random, 1, value_count * 2);
  let off = random.random_range(0..77);

  for bpv in 1..=64 {
    let mask = PackedInts::max_value(bpv);
    let mut packed_ints = create_packed_ints(value_count as i32, bpv)?;
    for ints in &mut packed_ints {
      for i in 0..ints.size() {
        ints.set(i, (31 * i as i64 - 1099) & mask);
      }
      let mut arr = vec![0i64; off + len];
      let msg = format!(
        "{} valueCount={}, index={}, len={}, off={}",
        ints, value_count, index, len, off
      );
      let gets = ints.get_bulk(index as i32, &mut arr, off as i32, len as i32);
      assert!(gets > 0, "{}: gets should be greater than 0", msg);
      assert!(
        gets <= len as i32,
        "{}: gets should be less than or equal to len",
        msg
      );
      assert!(
        gets <= (ints.size() - index as i32),
        "{}: gets should be less than or equal to remaining values",
        msg
      );
      for (i, &item) in arr.iter().enumerate() {
        let m = format!("{}, i={}", msg, i);
        if i >= off && i < off + gets as usize {
          assert_eq!(
            ints.get(i - off + index),
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
fn test_bulk_set() -> Result<()> {
  let mut random = random();
  let value_count = 1111;
  let index = random.random_range(0..value_count);
  let len = random.random_range(1..=(value_count * 2));
  let off = random.random_range(0..77);

  for bpv in 1..=64 {
    let mask = PackedInts::max_value(bpv);
    let mut packed_ints = create_packed_ints(value_count as i32, bpv)?;
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
      let sets = ints.set_bulk(index as i32, &arr, off as i32, len as i32);
      assert!(sets > 0, "{}: gets should be greater than 0", msg);
      assert!(
        sets <= len as i32,
        "{}: gets should be less than or equal to len",
        msg
      );
      for i in 0..ints.size() as usize {
        let m = format!("{}, i={}", msg, i);
        if i >= index && i < index + sets as usize {
          assert_eq!(
            ints.get(i),
            arr[(off as isize - index as isize + i as isize) as usize],
            "{}: value mismatch at index {}",
            m,
            i
          );
        } else {
          assert_eq!(
            ints.get(i),
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
fn test_copy() -> Result<()> {
  let mut random = random();
  let value_count = TestUtil::next_int(&mut random, 5, 600);
  let off1 = random.random_range(0..value_count);
  let off2 = random.random_range(0..value_count);
  let len = random.random_range(0..(value_count - off1).min(value_count - off2));
  let mem = random.random_range(0..1024);
  for bpv in 1..=64 {
    let mask = PackedInts::max_value(bpv);
    for mut r1 in create_packed_ints(value_count, bpv)? {
      for i in 0..r1.size() {
        r1.set(i, (31 * i as i64 - 1023) & mask);
      }
      for mut r2 in create_packed_ints(value_count, bpv)? {
        let msg = format!(
          "src={}, dest={}, srcPos={}, destPos={}, len={}, mem={}",
          r1, r2, off1, off2, len, mem
        );
        PackedInts::copy(&mut r1, off1, &mut r2, off2, len, mem);
        for i in 0..r2.size() {
          let m = format!("{}, i={}", msg, i);
          if i >= off2 && i < off2 + len {
            assert_eq!(
              r1.get((i - off2 + off1).try_convert()?),
              r2.get(i as usize),
              "{}: Values mismatch at index {}",
              m,
              i
            );
          } else {
            assert_eq!(
              r2.get(i as usize),
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
fn test_growable_writer() -> Result<()> {
  let mut random = random();
  let value_count = 113 + random.random_range(0..1112);

  let mut wrt = GrowableWriter::new(1, value_count, PackedInts::DEFAULT);

  wrt.set(4, 2);
  wrt.set(7, 10);
  wrt.set(value_count - 10, 99);
  wrt.set(99, 999);
  wrt.set(value_count - 1, 1 << 10);
  assert_eq!(wrt.get((value_count - 1).try_convert()?), 1 << 10);

  wrt.set(99, (1 << 23) - 1);
  assert_eq!(wrt.get((value_count - 1).try_convert()?), 1 << 10);

  wrt.set(1, i64::MAX);
  wrt.set(2, -3);
  assert_eq!(wrt.get_bits_per_value(), 64);
  assert_eq!(wrt.get((value_count - 1).try_convert()?), 1 << 10);
  assert_eq!(wrt.get(1), i64::MAX);
  assert_eq!(wrt.get(2), -3);
  assert_eq!(wrt.get(4), 2);
  assert_eq!(wrt.get(99), (1 << 23) - 1);
  assert_eq!(wrt.get(7), 10);
  assert_eq!(wrt.get((value_count - 10).try_convert()?), 99);
  assert_eq!(wrt.get((value_count - 1).try_convert()?), 1 << 10);

  // TODO:
  // Check memory usage
  // let ram_used = wrt.ram_bytes_used();
  // assert_eq!(ram_used, ram_usage(&wrt));

  Ok(())
}
#[test]
fn test_paged_growable_writer() -> Result<()> {
  let mut random = random();

  let page_size = 1 << TestUtil::next_int(&mut random, 6, 30);
  let acceptable_overhead_ratio = random.random::<f32>();
  let initial_bit_width = TestUtil::next_int(&mut random, 1, 64);

  let sub_reader =
    PagedGrowableWriter::with_fill_page(initial_bit_width, acceptable_overhead_ratio);

  let mut writer = AbstractPagedMutable::new(0, page_size, sub_reader)?;
  assert_eq!(writer.size(), 0);

  let mut buf = PackedLongValues::delta_packed_long_values_builder_default(random.random::<f32>())?;
  let size = if is_night_mode() {
    random.random_range(0..1_000_000)
  } else {
    random.random_range(0..100_000)
  };

  let mut max = 5;
  for _ in 0..size {
    buf.add(TestUtil::next_long(&mut random, 0, max))?;
    if rarely(&mut random) {
      max = PackedInts::max_value(if rarely(&mut random) {
        TestUtil::next_int(&mut random, 0, 63)
      } else {
        TestUtil::next_int(&mut random, 0, 31)
      });
    }
  }
  let bits_per_value = random.random_range(1..=64);
  writer = AbstractPagedMutable::new(
    size,
    page_size,
    PagedGrowableWriter::with_fill_page(bits_per_value, random.random::<f32>()),
  )?;
  assert_eq!(writer.size(), size);

  let values = buf.build()?;
  for i in (0..size).rev() {
    writer.set(i, values.get(i)?);
  }
  for i in 0..size {
    assert_eq!(values.get(i)?, writer.get(i)?);
  }

  // TODO
  // assert!(
  //     (RamUsageTester::ram_used(&writer) as f64 -
  // writer.ram_bytes_used() as f64).abs() < 8.0 );

  let new_size = TestUtil::next_usize(&mut random, writer.size() / 2, writer.size() * 3 / 2);
  let copy = writer.resize(new_size)?;
  for i in 0..copy.size() {
    if i < writer.size() {
      assert_eq!(writer.get(i)?, copy.get(i)?);
    } else {
      assert_eq!(copy.get(i)?, 0);
    }
  }

  let grow_size = TestUtil::next_usize(&mut random, writer.size() / 2, writer.size() * 3 / 2);
  let grow = writer.grow_with_size(grow_size)?;
  let grow_len;
  if let Some(new_writer) = grow {
    grow_len = new_writer.size();
    for (i, val) in (0..grow_len).map(|i| (i, new_writer.get(i).expect("should not fail"))) {
      if i < writer.size() {
        assert_eq!(val, writer.get(i)?);
      } else {
        assert_eq!(val, 0);
      }
    }
  }

  Ok(())
}
#[test]
fn test_paged_mutable() -> Result<()> {
  let mut random = random();
  let bits_per_value = TestUtil::next_int(&mut random, 1, 64);
  let max = PackedInts::max_value(bits_per_value);
  let page_size = 1 << TestUtil::next_int(&mut random, 6, 30);
  let acceptable_overhead_ratio = random.random::<f32>() / 2.0;

  let mut sub_mutable =
    PagedMutable::with_overhead_ratio(page_size, bits_per_value, acceptable_overhead_ratio);
  let mut writer = AbstractPagedMutable::new(0, page_size, sub_mutable)?;
  assert_eq!(writer.size(), 0);

  let mut buf = PackedLongValues::delta_packed_long_values_builder_default(random.random::<f32>())?;
  let size = if is_night_mode() {
    random.random_range(0..1_000_000)
  } else {
    random.random_range(0..100_000)
  };

  for _ in 0..size {
    let value = if bits_per_value == 64 {
      random.random_range(i64::MIN..=i64::MAX)
    } else {
      TestUtil::next_long(&mut random, 0, max)
    };
    buf.add(value)?;
  }

  let acceptable_overhead_ratio = random.random::<f32>();
  sub_mutable =
    PagedMutable::with_overhead_ratio(page_size, bits_per_value, acceptable_overhead_ratio);
  writer = AbstractPagedMutable::new(size, page_size, sub_mutable)?;

  assert_eq!(writer.size(), size);

  let values = buf.build()?;
  for i in (0..size).rev() {
    writer.set(i, values.get(i)?);
  }
  for i in 0..size {
    assert_eq!(values.get(i)?, writer.get(i)?);
  }

  // TODO
  // assert!(
  //     (RamUsageTester::ram_used(&writer) as f64 -
  // RamUsageTester::ram_used(&writer.format) as f64 -
  // writer.ram_bytes_used() as f64).abs() < 8.0 );

  let new_size = TestUtil::next_usize(&mut random, writer.size() / 2, writer.size() * 3 / 2);
  let copy = writer.resize(new_size)?;
  for i in 0..copy.size() {
    if i < writer.size() {
      assert_eq!(writer.get(i)?, copy.get(i)?);
    } else {
      assert_eq!(copy.get(i)?, 0);
    }
  }

  let grow_size = TestUtil::next_usize(&mut random, writer.size() / 2, writer.size() * 3 / 2);
  let grow_wrapper = writer.grow_with_size(grow_size)?;
  let grow_len;
  if let Some(g) = grow_wrapper {
    let grow = g;
    grow_len = grow.size();
    for i in 0..grow_len {
      if i < writer.size() {
        assert_eq!(grow.get(i)?, writer.get(i)?);
      } else {
        assert_eq!(grow.get(i)?, 0);
      }
    }
  }

  Ok(())
}

// memory hole
#[test]
#[ignore = "memory hole"]
fn test_paged_growable_writer_overflow() -> Result<()> {
  let mut random = random();
  let size = TestUtil::next_long(&mut random, 2 * i32::MAX as i64, 3 * i32::MAX as i64);
  let page_size = 1 << TestUtil::next_int(&mut random, 16, 30);
  let sub_reader = PagedGrowableWriter::with_fill_page(1, random.random::<f32>());
  let mut writer = AbstractPagedMutable::new(size.try_convert()?, page_size, sub_reader)?;
  let index = TestUtil::next_long(&mut random, i32::MAX as i64, size - 1);
  writer.set(index.try_convert()?, 2);
  assert_eq!(2, writer.get(index.try_convert()?)?);
  for _ in 0..1_000_000 {
    let idx = TestUtil::next_long(&mut random, 0, size);
    if idx == index {
      assert_eq!(2, writer.get(idx.try_convert()?)?);
    } else {
      assert_eq!(0, writer.get(idx.try_convert()?)?);
    }
  }
  Ok(())
}

#[test]
fn test_encode_decode() -> Result<()> {
  let mut random = random();

  for format in &[
    Packed(PackedImpl::new(0)),
    PackedSingleBlock(PackedSingleBlockImpl::new(1)),
  ] {
    for bpv in 1..=64 {
      if !format.is_supported(bpv) {
        continue;
      }

      // let msg = format!("{} {}", format, bpv);
      let msg = format!("{}", bpv);

      let encoder = PackedInts::get_encoder(*format, PackedInts::VERSION_CURRENT, bpv)?;
      let decoder = PackedInts::get_decoder(*format, PackedInts::VERSION_CURRENT, bpv)?;

      let long_block_count = Encoder::long_block_count(encoder);
      let long_value_count = Encoder::long_value_count(encoder);
      let byte_block_count = Encoder::byte_block_count(encoder);
      let byte_value_count = Encoder::byte_value_count(encoder);

      assert_eq!(long_block_count, Encoder::long_block_count(decoder));
      assert_eq!(long_value_count, Encoder::long_value_count(decoder));
      assert_eq!(byte_block_count, Encoder::byte_block_count(decoder));
      assert_eq!(byte_value_count, Encoder::byte_value_count(decoder));

      // let long_iterations = random.random_range(0..100);
      let long_iterations = 3;
      let byte_iterations = long_iterations * long_value_count / byte_value_count;
      assert_eq!(
        long_iterations * long_value_count,
        byte_iterations * byte_value_count
      );

      let blocks_offset = random.random_range(0..100) as usize;
      let values_offset = random.random_range(0..100) as usize;
      let blocks_offset2 = random.random_range(0..100) as usize;
      let blocks_len = (long_iterations * long_block_count) as usize;

      // 1. generate random inputs
      let mut blocks: Vec<u64> = vec![0; blocks_offset + blocks_len];
      for block in blocks.iter_mut() {
        *block = random.random::<u64>();

        if matches!(format, PackedSingleBlock(_)) && 64 % bpv != 0 {
          let to_clear = 64 % bpv;
          *block = (*block << to_clear) >> to_clear;
        }
      }

      // 2. decode
      let mut values = vec![0i64; values_offset + (long_iterations * long_value_count) as usize];
      decoder.decode_u64_to_i64(
        &blocks,
        blocks_offset,
        &mut values,
        values_offset,
        long_iterations,
      );
      for &value in &values {
        assert!(
          value <= PackedInts::max_value(bpv),
          "{}: value exceeds maxValue for bpv={}",
          msg,
          bpv
        );
      }
      let mut int_values = vec![0i32; values.len()];
      if bpv <= 32 {
        decoder.decode_u64_to_i32(
          &blocks,
          blocks_offset,
          &mut int_values,
          values_offset,
          long_iterations,
        );
        assert!(equals(&int_values, &values), "{}", msg);
      }

      // 3. re-encode
      let mut blocks2 = vec![0u64; blocks_offset2 + blocks_len];
      encoder.encode_i64_to_u64(
        &values,
        values_offset,
        &mut blocks2,
        blocks_offset2,
        long_iterations,
      );
      assert_eq!(
        &blocks[blocks_offset..],
        &blocks2[blocks_offset2..],
        "{}: Blocks mismatch after encoding",
        msg
      );
      if bpv <= 32 {
        let mut blocks3 = vec![0u64; blocks2.len()];
        encoder.encode_i32_to_u64(
          &int_values,
          values_offset,
          &mut blocks3,
          blocks_offset2,
          long_iterations,
        );
        assert_eq!(blocks2, blocks3, "{}", msg);
      }

      // 4. byte[] decoding
      let mut byte_blocks = vec![0u8; 8 * blocks.len()];
      let mut values2 = vec![0i64; values_offset + (long_iterations * long_value_count) as usize];
      byte_blocks
        .chunks_exact_mut(8)
        .zip(blocks.iter())
        .for_each(|(chunk, &block)| chunk.copy_from(&block.to_be_bytes(), 0));

      decoder.decode_u8_to_i64(
        &byte_blocks,
        blocks_offset * 8,
        &mut values2,
        values_offset,
        byte_iterations,
      );
      for &value in &values2 {
        assert!(
          value <= PackedInts::max_value(bpv),
          "{}: Byte-decoded value exceeds maxValue for bpv={}",
          msg,
          bpv
        );
      }
      assert_eq!(values, values2, "{}", msg);
      if bpv <= 32 {
        let mut int_values2 = vec![0i32; values2.len()];
        decoder.decode_u8_to_i32(
          &byte_blocks,
          blocks_offset * 8,
          &mut int_values2,
          values_offset,
          byte_iterations,
        );
        assert!(equals(&int_values2, &values2), "{}", msg);
      }
      // 5. byte[] encoding
      let mut blocks3 = vec![0u8; 8 * (blocks_offset2 + blocks_len)];
      encoder.encode_i64_to_u8(
        &values,
        values_offset,
        &mut blocks3,
        8 * blocks_offset2,
        byte_iterations,
      );
      assert_eq!(
        blocks2,
        blocks3
          .chunks_exact(8)
          .map(|chunk| u64::from_be_bytes(chunk.try_into().unwrap()))
          .collect::<Vec<_>>(),
        "{}: Byte-encoded blocks mismatch original blocks",
        msg
      );
      if bpv <= 32 {
        let mut blocks4 = vec![0u8; blocks3.len()];
        encoder.encode_i32_to_u8(
          &int_values,
          values_offset,
          &mut blocks4,
          8 * blocks_offset2,
          byte_iterations,
        );
        assert_eq!(blocks3, blocks4, "{}", msg);
      }
    }
  }

  Ok(())
}
fn equals(ints: &[i32], longs: &[i64]) -> bool {
  if ints.len() != longs.len() {
    return false;
  }
  for i in 0..ints.len() {
    if (ints[i] as i64 & 0xFFFFFFFF) != longs[i] {
      return false;
    }
  }
  true
}
#[test]
fn test_packed_long_values_on_zeros() {
  // TOOD
}
enum DataType {
  Packed,
  DeltaPacked,
  Monotonic,
}
#[test]
fn test_packed_long_values() -> Result<()> {
  let mut random = random();

  let arr_size = if is_night_mode() {
    random.random_range(1..=1_000_000)
  } else {
    random.random_range(1..=10_000)
  };
  let mut arr = vec![0i64; arr_size];

  let ratio_options = [PackedInts::DEFAULT, PackedInts::COMPACT, PackedInts::FAST];

  for bpv in [0, 1, 63, 64, random.random_range(2..=62)].iter() {
    for data_type in [DataType::DeltaPacked, DataType::Monotonic, DataType::Packed].iter() {
      // for data_type in [DataType::Packed].iter() {
      let page_size = 1 << TestUtil::next_int(&mut random, 6, 20);
      let acceptable_overhead_ratio =
        ratio_options[TestUtil::next_int(&mut random, 0, ratio_options.len() as i32 - 1) as usize];

      let mut buf: Builder;
      let inc: i64;

      match data_type {
        DataType::Packed => {
          buf = PackedLongValues::packed_long_values_builder(page_size, acceptable_overhead_ratio)?;
          inc = 0;
        },
        DataType::DeltaPacked => {
          buf = PackedLongValues::delta_packed_long_values_builder(
            page_size,
            acceptable_overhead_ratio,
          )?;
          inc = 0;
        },
        DataType::Monotonic => {
          buf =
            PackedLongValues::monotonic_long_values_builder(page_size, acceptable_overhead_ratio)?;
          inc = TestUtil::next_int(&mut random, -1000, 1000) as i64;
        },
      }

      if *bpv == 0 {
        arr[0] = random.random::<i64>();
        for i in 1..arr.len() {
          arr[i] = arr[i - 1] + inc;
        }
      } else if *bpv == 64 {
        arr.iter_mut().for_each(|item| {
          *item = random.random::<i64>();
        });
      } else {
        let min_value = TestUtil::next_long(
          &mut random,
          i64::MIN,
          i64::MAX - PackedInts::max_value(*bpv),
        );
        arr.iter_mut().enumerate().for_each(|(i, item)| {
          *item =
            min_value + inc * i as i64 + (random.random::<i64>() & PackedInts::max_value(*bpv));
        });
      }

      for &value in &arr {
        buf.add(value)?;
        if rarely(&mut random) && !is_night_mode() {
          // TODO
          // let expected_bytes_used =
          // ram_usage_tester_ram_used(&buf)?;
          // let computed_bytes_used = buf.ram_bytes_used();
          // assert_eq!(expected_bytes_used, computed_bytes_used);
        }
      }

      assert_eq!(arr.len(), buf.size() as usize);

      let values = buf.build()?;
      assert_eq!(arr.len(), values.size() as usize);

      for (i, &value) in arr.iter().enumerate() {
        assert_eq!(value, values.get(i)?);
      }

      let mut it = values.iterator();
      for &value in arr.iter() {
        if random.random_bool(0.5) {
          assert!(it.has_next());
        }
        assert_eq!(value, it.next_value());
      }
      assert!(!it.has_next());

      // TODO
      // let expected_bytes_used =
      // ram_usage_tester_ram_used(&values)?;
      // let computed_bytes_used = values.ram_bytes_used();
      // assert_eq!(expected_bytes_used, computed_bytes_used);
    }
  }

  Ok(())
}
#[test]
fn test_packed_input_output() {
  // PackedDataOutput is only used for tests, so we don't need to test it
}
#[test]
fn test_block_packed_reader_writer() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 2);
  for _ in 0..iters {
    let block_size = 1 << TestUtil::next_int(&mut random, 6, 18);
    let value_count: usize = if is_night_mode() {
      random.random_range(0..(1 << 18))
    } else {
      random.random_range(0..(1 << 15))
    };

    let mut values = vec![0i64; value_count];
    let mut min_value = 0;
    let mut bpv = 0;
    #[allow(clippy::needless_range_loop)]
    for i in 0..value_count {
      if i % block_size == 0 {
        min_value = if rarely(&mut random) {
          random.random_range(0..256) as i64
        } else if rarely(&mut random) {
          -5
        } else {
          random.random()
        };
        bpv = random.random_range(0..=64);
      }
      values[i] = if bpv == 0 {
        min_value
      } else if bpv == 64 {
        random.random()
      } else {
        min_value.wrapping_add(TestUtil::next_long(
          &mut random,
          0,
          PackedInts::max_value(bpv),
        ))
      };
    }
    let fp;
    let dir = new_directory_shared(&mut random)?;
    {
      let mut out = dir.create_output("out.bin", &IO_CONTEXT_DEFAULT)?;
      let mut writer = AbstractBlockPackedWriter::new(block_size, BlockPackedWriter)?;
      for (i, &value) in values.iter().enumerate() {
        assert_eq!(i, writer.ord());
        writer.add(value, &mut out)?;
      }
      assert_eq!(value_count, writer.ord());
      writer.finish(&mut out)?;
      assert_eq!(value_count, writer.ord());
      fp = out.get_file_pointer()?;
    }

    let mut buf = vec![0u8; fp];
    // test in1
    {
      let mut in1 = dir.open_input("out.bin", &IO_CONTEXT_DEFAULT)?;
      DataInput::read_bytes(&mut in1, &mut buf, 0, fp)?;
      in1.seek(0)?;
      let mut in_ref = in1;
      let mut it =
        BlockPackedReaderIterator::new(PackedInts::VERSION_CURRENT, block_size, value_count)?;

      let mut i = 0;
      while i < value_count {
        if random.random_bool(0.5) {
          assert_eq!(values[i], it.next_value(&mut in_ref)?);
          i += 1;
        } else {
          let next_values =
            it.next_batch(TestUtil::next_usize(&mut random, 1, 1024), &mut in_ref)?;
          for j in 0..next_values.length as usize {
            assert_eq!(
              values[i + j],
              next_values.longs[j + next_values.offset as usize]
            );
          }
          i += next_values.length;
        }
        assert_eq!(i, it.ord());
      }
      let result = it.next_value(&mut in_ref);
      assert!(matches!(result, Err(LuceneError::Eof(_))));
      assert_eq!(fp, in_ref.get_file_pointer()?);
      in_ref.seek(0)?;
      let mut it2 =
        BlockPackedReaderIterator::new(PackedInts::VERSION_CURRENT, block_size, value_count)?;
      i = 0;
      loop {
        let skip = TestUtil::next_usize(&mut random, 0, value_count - i);
        it2.skip(skip, &mut in_ref)?;
        i += skip;
        assert_eq!(i, it2.ord());
        if i == value_count {
          break;
        } else {
          assert_eq!(values[i], it2.next_value(&mut in_ref)?);
          i += 1;
        }
      }
      assert!(it2.skip(1, &mut in_ref).is_err());
      assert_eq!(fp, in_ref.get_file_pointer()?);
    }
    // test in2
    {
      let in2 = ByteArrayDataInput::with_bytes(buf.as_slice());
      let mut in_ref = in2;
      let mut it =
        BlockPackedReaderIterator::new(PackedInts::VERSION_CURRENT, block_size, value_count)?;

      let mut i = 0;
      while i < value_count {
        if random.random_bool(0.5) {
          assert_eq!(values[i], it.next_value(&mut in_ref)?);
          i += 1;
        } else {
          let next_values = it.next_batch(random.random_range(1..=1024), &mut in_ref)?;
          for j in 0..next_values.length {
            assert_eq!(
              values[i + j],
              next_values.longs[(j + next_values.offset) as usize]
            );
          }
          i += next_values.length;
        }
        assert_eq!(i, it.ord());
      }
      let result = it.next_value(&mut in_ref);
      assert!(matches!(result, Err(LuceneError::Eof(_))));
      assert_eq!(fp, in_ref.get_position());

      in_ref.set_position(0);
      let mut it2 =
        BlockPackedReaderIterator::new(PackedInts::VERSION_CURRENT, block_size, value_count)?;
      i = 0;
      loop {
        let skip = random.random_range(0..=value_count - i);
        it2.skip(skip, &mut in_ref)?;
        i += skip;
        assert_eq!(i, it2.ord());
        if i == value_count {
          break;
        } else {
          assert_eq!(values[i], it2.next_value(&mut in_ref)?);
          i += 1;
        }
      }
      assert!(it2.skip(1, &mut in_ref).is_err());
      assert_eq!(fp, in_ref.get_position());
    }
  }
  Ok(())
}
#[test]
fn test_monotonic_block_packed_reader_writer() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 2);
  for _ in 0..iters {
    let block_size = 1 << TestUtil::next_int(&mut random, 6, 18);
    let value_count = random.random_range(0..(1 << 18));
    let mut values = vec![0i64; value_count];

    if value_count > 0 {
      values[0] = if random.random_bool(0.5) {
        random.random_range(0..10) as i64
      } else {
        random.random_range(0..i32::MAX) as i64
      };

      let mut max_delta = random.random_range(0..64);
      for i in 1..value_count {
        if random.random_bool(0.1) {
          max_delta = random.random_range(0..64);
        }
        values[i] = std::cmp::max(
          0,
          values[i - 1] + TestUtil::next_int(&mut random, -16, max_delta) as i64,
        );
      }
    }
    let dir = new_directory_shared(&mut random)?;
    let file_pointer;
    {
      let mut out = dir.create_output("out.bin", &IO_CONTEXT_DEFAULT)?;
      let mut writer = AbstractBlockPackedWriter::new(block_size, MonotonicBlockPackedWriter)?;
      for (i, &value) in values.iter().enumerate().take(value_count) {
        assert_eq!(i, writer.ord());
        writer.add(value, &mut out)?;
      }
      assert_eq!(value_count, writer.ord());
      writer.finish(&mut out)?;
      assert_eq!(value_count, writer.ord());
      file_pointer = out.get_file_pointer()?;
    }
    let mut input = dir.open_input("out.bin", &IO_CONTEXT_DEFAULT)?;
    let reader = MonotonicBlockPackedReader::of(
      &mut input,
      PackedInts::VERSION_CURRENT,
      block_size as i32,
      value_count,
    )?;
    assert_eq!(file_pointer, input.get_file_pointer()?);
    for (i, &value) in values.iter().enumerate().take(value_count) {
      assert_eq!(value, reader.get(i)?);
    }
  }

  Ok(())
}
#[cfg(feature = "nightly")]
#[test]
#[ignore = "nightly"]
fn test_block_reader_overflow() -> Result<()> {
  let mut random = random();
  let value_count = TestUtil::next_usize(
    &mut random,
    1 + i64::from(i32::MAX) as usize,
    (i64::from(i32::MAX) * 2) as usize,
  );
  let block_size = 1 << TestUtil::next_long(&mut random, 20, 22);
  let dir = new_directory_shared(&mut random)?;
  let value_offset = TestUtil::next_usize(&mut random, 0, value_count - 1);
  let value = random.random::<i64>() & 0xFFFFFFFF;

  {
    let mut out = dir.create_output("out.bin", &IO_CONTEXT_DEFAULT)?;
    let mut writer = AbstractBlockPackedWriter::new(block_size, BlockPackedWriter)?;

    let mut i = 0;
    while i < value_count {
      assert_eq!(i, writer.ord());
      if (i & (block_size - 1)) == 0
        && (i + block_size < value_offset || (i > value_offset && i + block_size < value_count))
      {
        writer.add_block_of_zeros(&mut out)?;
        i += block_size;
      } else if i == value_offset {
        writer.add(value, &mut out)?;
        i += 1;
      } else {
        writer.add(0, &mut out)?;
        i += 1;
      }
    }
  }

  let mut input = dir.open_input("out.bin", &IO_CONTEXT_DEFAULT)?;
  assert!(block_size <= u32::MAX as usize);
  let mut reader =
    BlockPackedReaderIterator::new(PackedInts::VERSION_CURRENT, block_size, value_count)?;

  reader.skip(value_offset, &mut input)?;
  assert_eq!(value, reader.next_value(&mut input,)?);

  Ok(())
}

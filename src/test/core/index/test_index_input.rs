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
use crate::core::store::directory::Directory;
use crate::core::store::dummy::dummy_index_input::DummyIndexInput;
use crate::core::store::{
  ByteArrayDataInput, ByteArrayDataOutput, DataInput, DataOutput, IndexInput,
};
use crate::core::util::TryIntoInt;
use crate::core::util::access::ByteSourceMut;
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::group_vint_util::GroupVIntUtil;
use crate::test_framework::core::util::lucene_test_case::{
  is_night_mode, new_directory_shared, new_io_context, random, random_multiplier, rarely,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;
use std::fmt::{Display, Formatter};
use std::sync::LazyLock;

#[allow(dead_code)] // for quick search
struct TestIndexInput;

struct TestIndexInputContext {
  ints: Vec<i32>,
  longs: Vec<i64>,
  random_test_bytes: Vec<u8>,
}

static CONTEXT: LazyLock<TestIndexInputContext> = LazyLock::new(|| {
  let mut random = random();
  let (ints, longs, random_test_bytes) =
    before_class(&mut random).expect("failed to initialize TestIndexInput");
  TestIndexInputContext {
    ints,
    longs,
    random_test_bytes,
  }
});

pub static READ_TEST_BYTES: &[u8] = &[
  0x80, 0x01, 0xFF, 0x7F, 0x80, 0x80, 0x01, 0x81, 0x80, 0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0x07, 0xFF,
  0xFF, 0xFF, 0xFF, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0x07, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
  0xFF, 0x7F, 0x06, b'L', b'u', b'c', b'e', b'n', b'e', 0x02, 0xC2, 0xBF, 0x0A, b'L', b'u', 0xC2,
  0xBF, b'c', b'e', 0xC2, 0xBF, b'n', b'e', 0x03, 0xE2, 0x98, 0xA0, 0x0C, b'L', b'u', 0xE2, 0x98,
  0xA0, b'c', b'e', 0xE2, 0x98, 0xA0, b'n', b'e', 0x04, 0xF0, 0x9D, 0x84, 0x9E, 0x08, 0xF0, 0x9D,
  0x84, 0x9E, 0xF0, 0x9D, 0x85, 0xA0, 0x0E, b'L', b'u', 0xF0, 0x9D, 0x84, 0x9E, b'c', b'e', 0xF0,
  0x9D, 0x85, 0xA0, b'n', b'e', 0x01, 0x00, 0x08, b'L', b'u', 0x00, b'c', b'e', 0x00, b'n', b'e',
];
pub fn before_class<R>(random: &mut R) -> Result<(Vec<i32>, Vec<i64>, Vec<u8>)>
where
  R: Rng + ?Sized,
{
  let count = random_multiplier() as usize * 65536;

  let mut ints: Vec<i32> = vec![0; count];
  let mut longs: Vec<i64> = vec![0; count];

  let mut random_test_bytes = vec![0u8; count * (5 + 4 + 9 + 8)];

  let mut bdo = ByteArrayDataOutput::with_bytes(random_test_bytes.as_slice_mut());

  for i in 0..count {
    let i1: i32 = random.random();
    ints[i] = i1;

    bdo.write_vint(i1)?;
    bdo.write_int(i1)?;

    let l1: i64 = if rarely(random) {
      let upper = TestUtil::next_long(random, 0, i32::MAX as i64);
      upper << 32
    } else {
      TestUtil::next_long(random, 0, i64::MAX)
    };

    longs[i] = l1;

    bdo.write_vlong(l1)?;
    bdo.write_long(l1)?;
  }

  Ok((ints, longs, random_test_bytes))
}
fn check_reads<D>(input: &mut D) -> Result<()>
where
  D: DataInput,
{
  assert_eq!(128, input.read_vint()?);
  assert_eq!(16383, input.read_vint()?);
  assert_eq!(16384, input.read_vint()?);
  assert_eq!(16385, input.read_vint()?);
  assert_eq!(i32::MAX, input.read_vint()?);
  assert_eq!(-1, input.read_vint()?);

  assert_eq!(i32::MAX as i64, input.read_vlong()?);
  assert_eq!(i64::MAX, input.read_vlong()?);

  assert_eq!("Lucene", input.read_string()?);

  assert_eq!("\u{00BF}", input.read_string()?);
  assert_eq!("Lu\u{00BF}ce\u{00BF}ne", input.read_string()?);

  assert_eq!("\u{2620}", input.read_string()?);
  assert_eq!("Lu\u{2620}ce\u{2620}ne", input.read_string()?);

  assert_eq!("\u{1D11E}", input.read_string()?);
  assert_eq!("\u{1D11E}\u{1D160}", input.read_string()?);
  assert_eq!("Lu\u{1D11E}ce\u{1D160}ne", input.read_string()?);

  assert_eq!("\u{0000}", input.read_string()?);
  assert_eq!("Lu\u{0000}ce\u{0000}ne", input.read_string()?);

  Ok(())
}
fn check_random_reads<D>(input: &mut D, ints: &[i32], longs: &[i64]) -> Result<()>
where
  D: DataInput,
{
  let count = ints.len();
  debug_assert_eq!(count, longs.len());

  for i in 0..count {
    assert_eq!(ints[i], input.read_vint()?);
    assert_eq!(ints[i], input.read_int()?);
    assert_eq!(longs[i], input.read_vlong()?);
    assert_eq!(longs[i], input.read_long()?);
  }

  Ok(())
}
fn check_seeks_and_skips<I, R>(input: &mut I, random: &mut R) -> Result<()>
where
  I: IndexInput,
  R: Rng + ?Sized,
{
  let len = input.length()?;

  let iterations = if is_night_mode() { 1_000 } else { 10 };

  for _ in 0..iterations {
    input.seek(0)?;

    let mut curr = 0;
    while curr < len {
      let max_skip_to = len - 1;

      let skip_to = if len - curr < 10 {
        max_skip_to
      } else {
        TestUtil::next_usize(random, curr, max_skip_to)
      };

      let skip_delta = skip_to - curr;
      input.seek(curr)?;
      let start_byte_1 = input.read_byte()?;
      input.seek(skip_to)?;
      let end_byte_1 = input.read_byte()?;

      input.seek(curr)?;
      let start_byte_2 = input.read_byte()?;
      input.seek(curr)?;
      IndexInput::skip_bytes(input, skip_delta.try_convert()?)?;
      let end_byte_2 = input.read_byte()?;

      assert_eq!(start_byte_1, start_byte_2);
      assert_eq!(end_byte_1, end_byte_2);

      assert_eq!(curr + skip_delta + 1, input.get_file_pointer()?);

      curr = input.get_file_pointer()?;
    }
  }

  Ok(())
}
#[test]
fn test_raw_index_input_read() -> Result<()> {
  let mut random = random();
  let context = &*CONTEXT;

  let read_test_bytes = READ_TEST_BYTES.to_vec();

  for _ in 0..10 {
    let dir = new_directory_shared(&mut random)?;

    {
      let mut os = dir.create_output("foo", &new_io_context(&mut random)?)?;
      os.write_bytes_with_len(&read_test_bytes, read_test_bytes.len())?;
    }

    {
      let mut is = dir.open_input("foo", &new_io_context(&mut random)?)?;
      check_reads(&mut is)?;
      check_seeks_and_skips(&mut is, &mut random)?;
    }

    {
      let mut os = dir.create_output("bar", &new_io_context(&mut random)?)?;
      os.write_bytes_with_len(&context.random_test_bytes, context.random_test_bytes.len())?;
    }

    {
      let mut is = dir.open_input("bar", &new_io_context(&mut random)?)?;
      check_random_reads(&mut is, &context.ints, &context.longs)?;
      check_seeks_and_skips(&mut is, &mut random)?;
    }
  }

  Ok(())
}
#[test]
fn test_byte_array_data_input() -> Result<()> {
  {
    let mut input = ByteArrayDataInput::with_bytes(READ_TEST_BYTES);
    check_reads(&mut input)?;
  }

  {
    let context = &*CONTEXT;
    let mut input = ByteArrayDataInput::with_bytes(context.random_test_bytes.as_slice());
    check_random_reads(&mut input, &context.ints, &context.longs)?;
  }

  Ok(())
}
#[test]
fn test_no_read_on_skip_bytes() -> Result<()> {
  let mut random = random();

  let len = if is_night_mode() {
    i64::MAX as usize
  } else {
    1_000_000
  };

  let max_seek_pos = len - 1;

  let mut input = get_index_input(len);

  while input.get_file_pointer()? < max_seek_pos {
    let curr = input.get_file_pointer()?;

    let seek_pos = TestUtil::next_usize(&mut random, curr, max_seek_pos);

    let skip_delta = seek_pos - curr;

    IndexInput::skip_bytes(&mut input, skip_delta.try_convert()?)?;
    assert_eq!(seek_pos, input.get_file_pointer()?);
  }

  Ok(())
}

pub(crate) fn get_index_input(len: usize) -> InterceptingIndexInput {
  InterceptingIndexInput::new("foo", len)
}

pub(crate) struct InterceptingIndexInput {
  pos: usize,
  len: usize,
  resource_description: String,
}
impl InterceptingIndexInput {
  pub fn new(resource_description: &str, len: usize) -> Self {
    Self {
      pos: 0,
      len,
      resource_description: resource_description.to_string(),
    }
  }
}

impl crate::core::util::close::CloseableRef for InterceptingIndexInput {}

impl DataInput for InterceptingIndexInput {
  fn read_byte(&mut self) -> Result<u8> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn read_bytes(&mut self, _b: &mut [u8], _offset: usize, _len: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    GroupVIntUtil::read_group_vint_i32(self, dst, offset)
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    IndexInput::skip_bytes(self, num_bytes)
  }
}

impl Display for InterceptingIndexInput {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl TryClone for InterceptingIndexInput {
  fn try_clone(&self) -> Result<Self>
  where
    Self: Sized,
  {
    unreachable!("")
  }
}

impl IndexInput for InterceptingIndexInput {
  type IndexInput = InterceptingIndexInput;

  fn get_file_pointer(&self) -> Result<usize> {
    Ok(self.pos)
  }

  fn seek(&mut self, pos: usize) -> Result<()> {
    self.pos = pos;
    Ok(())
  }

  fn length(&self) -> Result<usize> {
    Ok(self.len)
  }

  type RandomAccessSlice = DummyIndexInput;

  fn random_access_slice(&self, _offset: usize, _length: usize) -> Result<Self::RandomAccessSlice> {
    Err(LuceneError::unsupported_operation(""))
  }
}

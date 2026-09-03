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
use crate::core::store::IO_CONTEXT_DEFAULT;

use crate::test_framework::core::util::lucene_test_case::{at_least, new_directory, random};
use rand::Rng;
use rand::RngExt;

use crate::core::codecs::compressing::stored_fields_ints::StoredFieldsInts;
use crate::core::store::directory::Directory;
use crate::core::store::{DataOutput, IOContext, IndexInput, IndexOutput};
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestStoredFieldsInt;
#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let num_iters = at_least(&mut random, 100);
  let dir = new_directory(&mut random)?;

  for _ in 0..num_iters {
    let len = random.random_range(1..=5000);
    let bpv = TestUtil::next_int(&mut random, 1, 31);
    let mut values = vec![0; len];
    for v in values.iter_mut().take(len) {
      *v = TestUtil::next_int(
        &mut random,
        0,
        1i32.wrapping_shl(bpv as u32).wrapping_sub(1),
      );
    }
    test(&mut random, &dir, &values)?;
  }

  Ok(())
}

#[test]
fn test_all_equals() -> Result<()> {
  let mut random = random();
  let dir = new_directory(&mut random)?;
  let len = random.random_range(1..=5000);
  let bpv = TestUtil::next_int(&mut random, 1, 31);
  let value = TestUtil::next_int(
    &mut random,
    0,
    1i32.wrapping_shl(bpv as u32).wrapping_sub(1),
  );
  let values = vec![value; len];
  test(&mut random, &dir, &values)?;
  Ok(())
}

fn test<R>(random: &mut R, dir: &impl Directory, ints: &[i32]) -> Result<()>
where
  R: Rng + ?Sized,
{
  let len;
  {
    let mut out = dir.create_output("tmp", IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?)?;
    StoredFieldsInts::write_ints(ints, 0, ints.len() as i32, &mut out)?;
    len = out.get_file_pointer()?;
    if random.random_bool(0.5) {
      out.write_long(0)?;
    }
  }

  {
    let mut input = dir.open_input("tmp", &IOContext::read_once_io_context()?)?;
    let offset = random.random_range(0..=4);
    let mut read = vec![0i64; ints.len() + offset];
    StoredFieldsInts::read_ints(&mut input, ints.len() as i32, &mut read, offset as i32)?;

    let read_ints: Vec<i32> = read[offset..offset + ints.len()]
      .iter()
      .map(|&v| v as i32)
      .collect();

    assert_eq!(ints, read_ints.as_slice());
    assert_eq!(len, input.get_file_pointer()?);
  }

  dir.delete_file("tmp")?;
  Ok(())
}

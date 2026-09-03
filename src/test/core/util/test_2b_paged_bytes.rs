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
use crate::core::index::BytesRef;
use crate::core::store::IO_CONTEXT_DEFAULT;
use crate::core::store::directory::Directory;
use crate::core::store::{DataOutput, IndexInput, IndexOutput};
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::paged_bytes::PagedBytes;
use crate::test_framework::core::util::lucene_test_case::{
  create_temp_dir_with_prefix, new_fs_directory, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

#[allow(dead_code)] // for quick search
struct Test2BPagedBytes;

#[cfg(feature = "monster")]
#[test]
#[ignore = "monster"]
fn test() -> Result<()> {
  let mut random = random();
  let dir = new_fs_directory(
    &mut random,
    create_temp_dir_with_prefix("test2BPagedBytes")?,
  )?;
  let mut paged_bytes = PagedBytes::new(15);
  let mut data_output =
    dir.create_output("foo", IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?)?;
  let mut net_bytes = 0usize;
  let seed = random.random::<u64>();
  let mut last_file_pointer = 0usize;
  let mut random_2 = StdRng::seed_from_u64(seed);
  while (net_bytes as f64) < 1.1 * (i32::MAX as f64) {
    let num_bytes = TestUtil::next_usize(&mut random_2, 1, 32768);
    let mut bytes = vec![0u8; num_bytes];
    random_2.fill(&mut bytes[..]);
    data_output.write_bytes_range(&bytes, 0, bytes.len())?;
    let file_pointer = data_output.get_file_pointer()?;
    assert_eq!(last_file_pointer + num_bytes, file_pointer);
    last_file_pointer = file_pointer;
    net_bytes += num_bytes;
  }
  data_output.close()?;

  let mut input = dir.open_input("foo", IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?)?;
  let input_length = input.length()?;
  paged_bytes.copy_with_input(&mut input, input_length)?;
  input.close()?;
  let reader = paged_bytes.freeze(true)?;

  random_2 = StdRng::seed_from_u64(seed);
  net_bytes = 0;
  while (net_bytes as f64) < 1.1 * (i32::MAX as f64) {
    let num_bytes = TestUtil::next_usize(&mut random_2, 1, 32768);
    let mut bytes = vec![0u8; num_bytes];
    random_2.fill(&mut bytes[..]);
    let expected = BytesRef::from_bytes(bytes);

    let mut actual = BytesRef::new();
    reader.fill_slice(&mut actual, net_bytes, num_bytes)?;
    assert_eq!(expected, actual);

    net_bytes += num_bytes;
  }
  dir.close()?;
  Ok(())
}

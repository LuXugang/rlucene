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

use rand::Rng;
use rand::RngExt;

use crate::core::codecs::lucene101::for_util::ForUtil;
use crate::core::codecs::lucene101::postings_util::PostingsUtil;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, random,
};

// checks for bug described in https://github.com/apache/lucene/issues/13373
#[allow(dead_code)] // for quick search
struct TestPostingsUtil;
#[test]
fn test_integer_overflow() -> Result<()> {
  let mut random = random();
  let random_size1: usize = random.random_range(1..3);
  let random_size2: usize = random.random_range(4..=ForUtil::BLOCK_SIZE);
  do_test_integer_overflow(&mut random, random_size1)?;
  do_test_integer_overflow(&mut random, random_size2)?;
  Ok(())
}
fn do_test_integer_overflow<R>(random: &mut R, size: usize) -> Result<()>
where
  R: Rng + ?Sized,
{
  let mut doc_delta_buffer = vec![0i32; size];
  let freq_buffer = vec![0i32; size];

  let delta = 1 << 30;
  doc_delta_buffer[0] = delta;

  // TODO: ByteBuffersDirectory not implement
  let dir = new_directory_shared(random)?;
  {
    let mut out = dir.create_output("test", &IOContext::default_io_context()?)?;
    PostingsUtil::write_vint_block(
      &mut out,
      &mut doc_delta_buffer,
      &freq_buffer,
      size as i32,
      true,
    )?;
  }

  let mut restored_docs = vec![0i32; size];
  let mut restored_freqs = vec![0i32; size];

  {
    let mut input = dir.open_input("test", &IOContext::default_io_context()?)?;
    PostingsUtil::read_vint_block(
      &mut input,
      &mut restored_docs,
      &mut restored_freqs,
      size,
      true,
      true,
    )?;
  }

  assert_eq!(delta, restored_docs[0]);
  Ok(())
}

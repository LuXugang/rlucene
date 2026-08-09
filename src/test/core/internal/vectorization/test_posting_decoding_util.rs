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
use crate::core::codecs::lucene101::for_util::ForUtil;
use crate::core::internal::vectorization::posting_decoding_util::PostingDecodingUtil;
use crate::core::internal::vectorization::vectorization_provider::{
  DEFAULT_VECTORIZATION_PROVIDER, VectorizationProvider,
};
use crate::core::store::directory::Directory;
use crate::core::store::mmap_directory::MMapDirectory;
use crate::core::store::{DataOutput, IOContext, IndexInput};
use crate::core::util::bit_util::BitUtil;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::io_utils::IOUtils;
use crate::test_framework::core::util::lucene_test_case::{at_least, create_temp_dir, random};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::RngExt;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[allow(dead_code)] // for quick search
struct TestPostingDecodingUtil;

#[test]
fn test_duel_split_ints() -> Result<()> {
  let mut random = random();
  let iterations = at_least(&mut random, 100);
  let temp_dir = create_temp_dir()?;
  let dir = MMapDirectory::new(temp_dir.path().to_path_buf())?;

  let body_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
    let io_context = IOContext::default_io_context()?;
    let mut out = dir.create_output("tests.bin", &io_context)?;
    let write_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
      out.write_int(random.random())?;
      for _ in 0..ForUtil::BLOCK_SIZE {
        out.write_long(random.random::<i32>() as i64)?;
      }
      Ok(())
    }));
    let close_result = catch_unwind(AssertUnwindSafe(|| out.close()));
    IOUtils::use_or_suppress_caught_result(write_result, close_result)?;

    let input = dir.open_input("tests.bin", &io_context)?;
    let read_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
      let mut expected_b = vec![0i32; ForUtil::BLOCK_SIZE];
      let mut expected_c = vec![0i32; ForUtil::BLOCK_SIZE];
      let mut actual_b = vec![0i32; ForUtil::BLOCK_SIZE];
      let mut actual_c = vec![0i32; ForUtil::BLOCK_SIZE];
      for _ in 0..iterations {
        // Initialize arrays with random content.
        for i in 0..expected_b.len() {
          expected_b[i] = random.random();
          actual_b[i] = expected_b[i];
          expected_c[i] = random.random();
          actual_c[i] = expected_c[i];
        }
        let b_shift = TestUtil::next_int(&mut random, 1, 31);
        let dec = TestUtil::next_int(&mut random, 1, b_shift);
        let num_iters = (b_shift + dec - 1) / dec;
        let count = TestUtil::next_int(&mut random, 1, 64 / num_iters);
        let b_mask = random.random();
        let c_index = random.random_range(0..64);
        let c_mask = random.random();
        let start_fp = random.random_range(0..4);

        // Work on slices that have just enough bytes so an implementation that reads more than
        // the allowed padding fails with an out-of-bounds error.
        let slice_length = start_fp + count as usize * BitUtil::LONG_BYTES;
        let mut expected_util = PostingDecodingUtil::new(input.slice("test", 0, slice_length)?);
        let mut actual_util = DEFAULT_VECTORIZATION_PROVIDER
          .new_posting_decoding_util(input.slice("test", 0, slice_length)?);

        expected_util.input.seek(start_fp)?;
        expected_util.split_ints_diff(
          count,
          &mut expected_b,
          b_shift,
          dec,
          b_mask,
          &mut expected_c,
          c_index,
          c_mask,
        )?;
        let expected_end_fp = expected_util.input.get_file_pointer()?;
        actual_util.input.seek(start_fp)?;
        actual_util.split_ints_diff(
          count,
          &mut actual_b,
          b_shift,
          dec,
          b_mask,
          &mut actual_c,
          c_index,
          c_mask,
        )?;
        assert_eq!(expected_end_fp, actual_util.input.get_file_pointer()?);
        assert_eq!(expected_b, actual_b);
        assert_eq!(expected_c, actual_c);
      }
      Ok(())
    }));
    let close_result = catch_unwind(AssertUnwindSafe(|| input.close()));
    IOUtils::use_or_suppress_caught_result(read_result, close_result)
  }));
  let close_result = catch_unwind(AssertUnwindSafe(|| dir.close()));
  IOUtils::use_or_suppress_caught_result(body_result, close_result)
}

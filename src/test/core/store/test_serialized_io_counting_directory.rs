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
use crate::test_framework::core::util::lucene_test_case::{new_directory, random};
use std::path::PathBuf;

use rand::Rng;

use crate::core::store::directory::Directory;
use crate::core::store::fs_directory_base::{BuiltInFSIndexInput, FSDirectoryBaseEnum};
use crate::core::store::mmap_directory::MMapDirectory;
use crate::core::store::{
  DataInput, DataOutput, FSDirectories, IOContext, IndexInput, IndexInputEnum2, ReadAdvice,
};
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::io_utils::IOUtils;
use crate::test_framework::core::store::base_directory_test_case::BaseDirectoryTestCase;
use crate::test_framework::core::store::serial_io_counting_directory::{
  SerialIOCountingDirectory, SerializedIOCountingIndexInput,
};

#[allow(dead_code)] // for quick search
pub struct TestSerializedIOCountingDirectory;

impl BaseDirectoryTestCase for TestSerializedIOCountingDirectory {
  type Directory = SerialIOCountingDirectory<FSDirectories>;
  type Output =
    IndexInputEnum2<BuiltInFSIndexInput, SerializedIOCountingIndexInput<BuiltInFSIndexInput>>;

  fn get_directory<R>(&self, path: PathBuf, _random: &mut R) -> Result<Self::Directory>
  where
    R: Rng + ?Sized,
  {
    Ok(SerialIOCountingDirectory::new(FSDirectories::open(path)?))
  }

  fn configure_is_loaded_test(&self, dir: &mut Self::Directory) -> bool {
    match &mut dir.get_delegate_mut().sub_fs_directory {
      FSDirectoryBaseEnum::MMap(dir) => {
        dir.set_preload(MMapDirectory::ALL_FILES);
        true
      },
      FSDirectoryBaseEnum::NIO(_) => false,
    }
  }
}

impl TestSerializedIOCountingDirectory {
  fn test_sequential_reads<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = SerialIOCountingDirectory::new(new_directory(random)?);
    let body_result = (|| -> Result<()> {
      let mut out = dir.create_output("test", &IOContext::default_io_context()?)?;
      let write_result = (|| -> Result<()> {
        for _ in 0..10 {
          out.write_bytes_with_len(&[0u8; 4096], 4096)?;
        }
        Ok(())
      })();
      IOUtils::use_or_suppress_result(write_result, out.close())?;

      let context = IOContext::default_io_context()?.with_read_advice_self(ReadAdvice::Normal)?;
      let mut input = dir.open_input("test", &context)?;
      let read_result = (|| -> Result<()> {
        input.read_byte()?;
        let count = dir.count();
        while input.get_file_pointer()? < input.length()? {
          input.read_byte()?;
        }
        // Sequential reads are free with the normal advice
        assert_eq!(count, dir.count());
        Ok(())
      })();
      IOUtils::use_or_suppress_result(read_result, input.close())?;

      let context = IOContext::default_io_context()?.with_read_advice_self(ReadAdvice::Random)?;
      let mut input = dir.open_input("test", &context)?;
      let read_result = (|| -> Result<()> {
        input.read_byte()?;
        let count = dir.count();
        while input.get_file_pointer()? < input.length()? {
          input.read_byte()?;
        }
        // But not with the random advice
        assert_ne!(count, dir.count());
        Ok(())
      })();
      IOUtils::use_or_suppress_result(read_result, input.close())
    })();
    IOUtils::use_or_suppress_result(body_result, dir.close())
  }

  fn test_parallel_reads<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = SerialIOCountingDirectory::new(new_directory(random)?);
    let body_result = (|| -> Result<()> {
      let mut out = dir.create_output("test", &IOContext::default_io_context()?)?;
      let write_result = (|| -> Result<()> {
        for _ in 0..10 {
          out.write_bytes_with_len(&[0u8; 4096], 4096)?;
        }
        Ok(())
      })();
      IOUtils::use_or_suppress_result(write_result, out.close())?;

      let context = IOContext::default_io_context()?.with_read_advice_self(ReadAdvice::Random)?;
      let mut input = dir.open_input("test", &context)?;
      let read_result = (|| -> Result<()> {
        let mut count = dir.count();

        // count is incremented on the first prefetch
        input.prefetch(5_000, 1)?;
        assert_eq!(count + 1, dir.count());
        count = dir.count();

        // but not on the second one since it can be performed in parallel
        input.prefetch(10_000, 1)?;
        assert_eq!(count, dir.count());

        // and reading from a prefetched page doesn't increment the counter
        input.seek(5_000)?;
        input.read_byte()?;
        assert_eq!(count, dir.count());

        input.seek(10_000)?;
        input.read_byte()?;
        assert_eq!(count, dir.count());

        // reading data on a page that was not prefetched increments the counter
        input.seek(15_000)?;
        input.read_byte()?;
        assert_eq!(count + 1, dir.count());
        Ok(())
      })();
      IOUtils::use_or_suppress_result(read_result, input.close())
    })();
    IOUtils::use_or_suppress_result(body_result, dir.close())
  }
}

#[test]
fn test_sequential_reads() -> Result<()> {
  run_case(|case, random| case.test_sequential_reads(random))
}

#[test]
fn test_parallel_reads() -> Result<()> {
  run_case(|case, random| case.test_parallel_reads(random))
}

mod base_directory_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::store::test_serialized_io_counting_directory::run_case;
  use crate::test_framework::core::store::base_directory_test_case::BaseDirectoryTestCase;

  #[test]
  fn test_copy_from() -> Result<()> {
    run_case(|case, random| case.test_copy_from(random))
  }

  #[test]
  fn test_rename() -> Result<()> {
    run_case(|case, random| case.test_rename(random))
  }

  #[test]
  fn test_delete_file() -> Result<()> {
    run_case(|case, random| case.test_delete_file(random))
  }

  #[test]
  fn test_byte() -> Result<()> {
    run_case(|case, random| case.test_byte(random))
  }

  #[test]
  fn test_short() -> Result<()> {
    run_case(|case, random| case.test_short(random))
  }

  #[test]
  fn test_int() -> Result<()> {
    run_case(|case, random| case.test_int(random))
  }

  #[test]
  fn test_long() -> Result<()> {
    run_case(|case, random| case.test_long(random))
  }

  #[test]
  fn test_aligned_little_endian_longs() -> Result<()> {
    run_case(|case, random| case.test_aligned_little_endian_longs(random))
  }

  #[test]
  fn test_unaligned_little_endian_longs() -> Result<()> {
    run_case(|case, random| case.test_unaligned_little_endian_longs(random))
  }

  #[test]
  fn test_little_endian_longs_underflow() -> Result<()> {
    run_case(|case, random| case.test_little_endian_longs_underflow(random))
  }

  #[test]
  fn test_aligned_ints() -> Result<()> {
    run_case(|case, random| case.test_aligned_ints(random))
  }

  #[test]
  fn test_unaligned_ints() -> Result<()> {
    run_case(|case, random| case.test_unaligned_ints(random))
  }

  #[test]
  fn test_ints_underflow() -> Result<()> {
    run_case(|case, random| case.test_ints_underflow(random))
  }

  #[test]
  fn test_aligned_floats() -> Result<()> {
    run_case(|case, random| case.test_aligned_floats(random))
  }

  #[test]
  fn test_unaligned_floats() -> Result<()> {
    run_case(|case, random| case.test_unaligned_floats(random))
  }

  #[test]
  fn test_floats_underflow() -> Result<()> {
    run_case(|case, random| case.test_floats_underflow(random))
  }

  #[test]
  fn test_string() -> Result<()> {
    run_case(|case, random| case.test_string(random))
  }

  #[test]
  fn test_vint() -> Result<()> {
    run_case(|case, random| case.test_vint(random))
  }

  #[test]
  fn test_vlong() -> Result<()> {
    run_case(|case, random| case.test_vlong(random))
  }

  #[test]
  fn test_zint() -> Result<()> {
    run_case(|case, random| case.test_zint(random))
  }

  #[test]
  fn test_zlong() -> Result<()> {
    run_case(|case, random| case.test_zlong(random))
  }

  #[test]
  fn test_set_of_strings() -> Result<()> {
    run_case(|case, random| case.test_set_of_strings(random))
  }

  #[test]
  fn test_map_of_strings() -> Result<()> {
    run_case(|case, random| case.test_map_of_strings(random))
  }

  #[test]
  fn test_checksum() -> Result<()> {
    run_case(|case, random| case.test_checksum(random))
  }

  #[test]
  fn test_thread_safety_in_list_all() -> Result<()> {
    run_case(|case, random| case.test_thread_safety_in_list_all(random))
  }

  #[test]
  fn test_file_exists_in_list_after_created() -> Result<()> {
    run_case(|case, random| case.test_file_exists_in_list_after_created(random))
  }

  #[test]
  fn test_seek_to_eof_then_back() -> Result<()> {
    run_case(|case, random| case.test_seek_to_eof_then_back(random))
  }

  #[test]
  fn test_illegal_eof() -> Result<()> {
    run_case(|case, random| case.test_illegal_eof(random))
  }

  #[test]
  fn test_seek_past_eof() -> Result<()> {
    run_case(|case, random| case.test_seek_past_eof(random))
  }

  #[test]
  fn test_slice_out_of_bounds() -> Result<()> {
    run_case(|case, random| case.test_slice_out_of_bounds(random))
  }

  #[test]
  fn test_no_dir() -> Result<()> {
    run_case(|case, random| case.test_no_dir(random))
  }

  #[test]
  fn test_copy_bytes() -> Result<()> {
    run_case(|case, random| case.test_copy_bytes(random))
  }

  #[test]
  fn test_copy_bytes_with_threads() -> Result<()> {
    run_case(|case, random| case.test_copy_bytes_with_threads(random))
  }

  #[test]
  fn test_fsync_doesnt_create_new_files() -> Result<()> {
    run_case(|case, random| case.test_fsync_doesnt_create_new_files(random))
  }

  #[test]
  fn test_random_long() -> Result<()> {
    run_case(|case, random| case.test_random_long(random))
  }

  #[test]
  fn test_random_int() -> Result<()> {
    run_case(|case, random| case.test_random_int(random))
  }

  #[test]
  fn test_random_short() -> Result<()> {
    run_case(|case, random| case.test_random_short(random))
  }

  #[test]
  fn test_random_byte() -> Result<()> {
    run_case(|case, random| case.test_random_byte(random))
  }

  #[test]
  fn test_slice_of_slice() -> Result<()> {
    run_case(|case, random| case.test_slice_of_slice(random))
  }

  #[test]
  fn test_large_writes() -> Result<()> {
    run_case(|case, random| case.test_large_writes(random))
  }

  #[test]
  fn test_index_output_to_string() -> Result<()> {
    run_case(|case, random| case.test_index_output_to_string(random))
  }

  #[test]
  fn test_create_temp_output() -> Result<()> {
    run_case(|case, random| case.test_create_temp_output(random))
  }

  #[test]
  fn test_create_output_for_existing_file() -> Result<()> {
    run_case(|case, random| case.test_create_output_for_existing_file(random))
  }

  #[test]
  fn test_seek_to_end_of_file() -> Result<()> {
    run_case(|case, random| case.test_seek_to_end_of_file(random))
  }

  #[test]
  fn test_seek_beyond_end_of_file() -> Result<()> {
    run_case(|case, random| case.test_seek_beyond_end_of_file(random))
  }

  #[test]
  fn test_pending_deletions() -> Result<()> {
    run_case(|case, random| case.test_pending_deletions(random))
  }

  #[test]
  fn test_list_all_is_sorted() -> Result<()> {
    run_case(|case, random| case.test_list_all_is_sorted(random))
  }

  #[test]
  fn test_data_types() -> Result<()> {
    run_case(|case, random| case.test_data_types(random))
  }

  #[test]
  fn test_group_vint_overflow() -> Result<()> {
    run_case(|case, random| case.test_group_vint_overflow(random))
  }

  #[test]
  fn test_group_vint() -> Result<()> {
    run_case(|case, random| case.test_group_vint(random))
  }

  #[test]
  fn test_prefetch() -> Result<()> {
    run_case(|case, random| case.test_prefetch(random))
  }

  #[test]
  fn test_prefetch_on_slice() -> Result<()> {
    run_case(|case, random| case.test_prefetch_on_slice(random))
  }

  #[test]
  fn test_update_read_advice() -> Result<()> {
    run_case(|case, random| case.test_update_read_advice(random))
  }

  #[test]
  fn test_is_loaded() -> Result<()> {
    run_case(|case, random| case.test_is_loaded(random))
  }

  #[test]
  fn test_is_loaded_on_slice() -> Result<()> {
    run_case(|case, random| case.test_is_loaded_on_slice(random))
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestSerializedIOCountingDirectory, &mut rand::prelude::StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestSerializedIOCountingDirectory;
  f(&case, &mut random)
}

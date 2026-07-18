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
  is_night_mode, random, random_multiplier,
};
use std::mem::size_of;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;

use parking_lot::Mutex;
use rand::prelude::StdRng;
use rand::{Rng, RngExt, SeedableRng};
use tempfile::Builder;

use crate::core::store::directory::Directory;
use crate::core::store::memory_segment_index_input::MemorySegmentIndexInput;
use crate::core::store::mmap_directory::{MMapDirectory, MMapPreload};
use crate::core::store::{
  DataInput, DataOutput, FSDirectory, IOContext, IndexInput, NativeFSLockFactory, ReadAdvice,
};
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::store::base_directory_test_case::BaseDirectoryTestCase;

#[allow(dead_code)] // for quick search
struct TestMMapDirectory {
  preload_random: Arc<Mutex<StdRng>>,
}

impl BaseDirectoryTestCase for TestMMapDirectory {
  type Directory = FSDirectory<NativeFSLockFactory, MMapDirectory>;
  type Output = MemorySegmentIndexInput;

  fn get_directory<R>(&self, path: PathBuf, _random: &mut R) -> Result<Self::Directory>
  where
    R: Rng + ?Sized,
  {
    let mut dir = MMapDirectory::new(path)?;
    let preload_random = Arc::clone(&self.preload_random);
    dir.set_preload(MMapPreload::custom(move |_file, _context| {
      let mut random = preload_random.lock();
      random.random_bool(0.5)
    }));
    Ok(dir)
  }
}

impl TestMMapDirectory {
  fn test_ace_with_threads<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let n_ints = 8 * 1024 * 1024;
    let io_context = IOContext::default_io_context()?;
    let temp_dir = Builder::new().prefix("testAceWithThreads").tempdir()?;
    let dir = self.get_directory(temp_dir.path().to_path_buf(), random)?;

    {
      let mut out = dir.create_output("test", &io_context)?;
      for _ in 0..n_ints {
        out.write_int(random.random())?;
      }
    }

    let iters = random_multiplier() * if is_night_mode() { 50 } else { 10 };
    for _ in 0..iters {
      let input = dir.open_input("test", &io_context)?;
      let mut clone = input.try_clone()?;
      let mut accum = vec![0u8; n_ints * size_of::<i32>()];
      let shotgun = Arc::new(Barrier::new(2));
      let shotgun_clone = Arc::clone(&shotgun);
      let t1 = thread::spawn(move || -> Result<()> {
        shotgun_clone.wait();
        for _ in 0..10 {
          let read_result = (|| -> Result<()> {
            clone.seek(0)?;
            let accum_len = accum.len();
            DataInput::read_bytes(&mut clone, &mut accum, 0, accum_len)
          })();
          match read_result {
            Ok(()) => {},
            Err(LuceneError::AlreadyClosed(_)) => return Ok(()),
            Err(err) => return Err(err),
          }
        }
        Ok(())
      });
      shotgun.wait();
      drop(input);
      t1.join().unwrap()?;
    }

    Ok(())
  }

  fn test_with_normal<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let size = 8 * 1024;
    let mut bytes = vec![0u8; size];
    random.fill(&mut bytes[..]);
    let io_context = IOContext::default_io_context()?;
    let normal_context = IOContext::with_read_advice(ReadAdvice::Normal)?;
    let temp_dir = Builder::new().prefix("testWithRandom").tempdir()?;
    let dir = MMapDirectory::new(temp_dir.path().to_path_buf())?;

    {
      let mut out = dir.create_output("test", &io_context)?;
      out.write_bytes_with_len(&bytes, bytes.len())?;
    }

    {
      let mut input = dir.open_input("test", &normal_context)?;
      let mut read_bytes = vec![0u8; size];
      DataInput::read_bytes(&mut input, &mut read_bytes, 0, size)?;
      assert_eq!(bytes, read_bytes);
    }

    Ok(())
  }
}

#[test]
fn test_ace_with_threads() -> Result<()> {
  run_case(|case, random| case.test_ace_with_threads(random))
}

#[test]
fn test_null_params_index_input() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_madvise_avail() -> Result<()> {
  assert_eq!(
    cfg!(any(target_os = "linux", target_os = "macos")),
    MMapDirectory::supports_madvise(),
    "madvise should be supported on Linux and macOS"
  );
  Ok(())
}

#[test]
fn test_with_normal() -> Result<()> {
  run_case(|case, random| case.test_with_normal(random))
}

#[test]
fn test_confined() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_arenas() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_arenas_many_segment_files() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_group_by_segment_func() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_no_grouping_func() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

mod base_directory_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::store::test_mmap_directory::run_case;
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
  F: FnOnce(&TestMMapDirectory, &mut rand::prelude::StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestMMapDirectory {
    preload_random: Arc::new(Mutex::new(StdRng::seed_from_u64(random.random()))),
  };
  f(&case, &mut random)
}

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
  create_temp_dir_with_prefix, new_io_context, random,
};
use std::path::PathBuf;

use rand::prelude::StdRng;
use rand::{Rng, RngExt};

use crate::core::store::directory::Directory;
use crate::core::store::memory_segment_index_input::MemorySegmentIndexInput;
use crate::core::store::mmap_directory::MMapDirectory;
use crate::core::store::{
  DataInput, DataOutput, FSDirectory, IOContext, IndexInput, NativeFSLockFactory,
};
use crate::core::util::clone::TryClone;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::store::base_chunked_directory_test_case::BaseChunkedDirectoryTestCase;
use crate::test_framework::core::store::base_directory_test_case::BaseDirectoryTestCase;
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestMultiMMap;

impl BaseDirectoryTestCase for TestMultiMMap {
  type Directory = FSDirectory<NativeFSLockFactory, MMapDirectory>;
  type Output = MemorySegmentIndexInput;

  fn get_directory<R>(&self, path: PathBuf, random: &mut R) -> Result<Self::Directory>
  where
    R: Rng + ?Sized,
  {
    let max_chunk_size = 1_u64 << TestUtil::next_int(random, 10, 20);
    MMapDirectory::with_max_chunk_size(path, max_chunk_size)
  }

  fn configure_is_loaded_test(&self, dir: &mut Self::Directory) -> bool {
    dir.set_preload(MMapDirectory::ALL_FILES);
    true
  }
}

impl BaseChunkedDirectoryTestCase for TestMultiMMap {
  fn get_directory_with_max_chunk_size(
    &self,
    path: PathBuf,
    max_chunk_size: usize,
  ) -> Result<Self::Directory> {
    MMapDirectory::with_max_chunk_size(path, max_chunk_size as u64)
  }
}

impl TestMultiMMapTests for TestMultiMMap {}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestMultiMMap, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestMultiMMap;
  f(&case, &mut random)
}

mod test_multi_mmap_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::store::test_multi_mmap::{TestMultiMMapTests, run_case};

  #[test]
  fn test_seeking_exceptions() -> Result<()> {
    run_case(|case, random| case.test_seeking_exceptions(random))
  }

  #[test]
  fn test_clone_safety() -> Result<()> {
    run_case(|case, random| case.test_clone_safety(random))
  }

  #[test]
  fn test_clone_slice_safety() -> Result<()> {
    run_case(|case, random| case.test_clone_slice_safety(random))
  }

  #[test]
  fn test_implementations() -> Result<()> {
    run_case(|case, random| case.test_implementations(random))
  }
}

mod base_directory_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::store::test_multi_mmap::run_case;
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
  fn test_detect_close() -> Result<()> {
    run_case(|case, random| case.test_detect_close(random))
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
  fn test_large_writes() -> Result<()> {
    run_case(|case, random| case.test_large_writes(random))
  }

  #[test]
  fn test_index_output_to_string() -> Result<()> {
    run_case(|case, random| case.test_index_output_to_string(random))
  }

  #[test]
  fn test_double_close_output() -> Result<()> {
    run_case(|case, random| case.test_double_close_output(random))
  }

  #[test]
  fn test_double_close_input() -> Result<()> {
    run_case(|case, random| case.test_double_close_input(random))
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

mod base_chunked_directory_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::store::test_multi_mmap::run_case;
  use crate::test_framework::core::store::base_chunked_directory_test_case::BaseChunkedDirectoryTestCase;

  #[test]
  fn test_group_vint_multi_blocks() -> Result<()> {
    run_case(|case, random| {
      BaseChunkedDirectoryTestCase::test_group_vint_multi_blocks(case, random)
    })
  }

  #[test]
  fn test_clone_close() -> Result<()> {
    run_case(BaseChunkedDirectoryTestCase::test_clone_close)
  }

  #[test]
  fn test_clone_slice_close() -> Result<()> {
    run_case(BaseChunkedDirectoryTestCase::test_clone_slice_close)
  }

  #[test]
  fn test_seek_zero() -> Result<()> {
    run_case(BaseChunkedDirectoryTestCase::test_seek_zero)
  }

  #[test]
  fn test_seek_slice_zero() -> Result<()> {
    run_case(BaseChunkedDirectoryTestCase::test_seek_slice_zero)
  }

  #[test]
  fn test_seek_end() -> Result<()> {
    run_case(BaseChunkedDirectoryTestCase::test_seek_end)
  }

  #[test]
  fn test_seek_slice_end() -> Result<()> {
    run_case(BaseChunkedDirectoryTestCase::test_seek_slice_end)
  }

  #[test]
  fn test_seeking() -> Result<()> {
    run_case(BaseChunkedDirectoryTestCase::test_seeking)
  }

  #[test]
  fn test_sliced_seeking() -> Result<()> {
    run_case(BaseChunkedDirectoryTestCase::test_sliced_seeking)
  }

  #[test]
  fn test_slice_of_slice() -> Result<()> {
    run_case(BaseChunkedDirectoryTestCase::test_slice_of_slice)
  }

  #[test]
  fn test_random_chunk_sizes() -> Result<()> {
    run_case(BaseChunkedDirectoryTestCase::test_random_chunk_sizes)
  }

  #[test]
  fn test_bytes_cross_boundary() -> Result<()> {
    run_case(BaseChunkedDirectoryTestCase::test_bytes_cross_boundary)
  }

  #[test]
  fn test_little_endian_longs_cross_boundary() -> Result<()> {
    run_case(|case, random| {
      BaseChunkedDirectoryTestCase::test_little_endian_longs_cross_boundary(case, random)
    })
  }

  #[test]
  fn test_little_endian_floats_cross_boundary() -> Result<()> {
    run_case(|case, random| {
      BaseChunkedDirectoryTestCase::test_little_endian_floats_cross_boundary(case, random)
    })
  }
}

trait TestMultiMMapTests: BaseChunkedDirectoryTestCase<Output = MemorySegmentIndexInput> {
  fn test_seeking_exceptions<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let _ = random;
    let slice_size = 128;
    let temp_dir = create_temp_dir_with_prefix("testSeekingExceptions")?;
    let mmap_dir =
      self.get_directory_with_max_chunk_size(temp_dir.path().to_path_buf(), slice_size)?;
    let size = 128 + 63;
    {
      let mut out = mmap_dir.create_output("a", &IOContext::default_io_context()?)?;
      for _ in 0..size {
        out.write_byte(0)?;
      }
      out.close()?;
    }

    let mut input = mmap_dir.open_input("a", &IOContext::default_io_context()?)?;

    // TODO IMPORTANT: Java verifies the error for seek(-1234). Rust's
    // IndexInput::seek accepts usize, so a negative position cannot be
    // represented at this API boundary.

    let pos_after_eof = size + 123;
    let eof = input.seek(pos_after_eof).unwrap_err();
    assert!(matches!(&eof, LuceneError::Eof(_)));
    assert!(
      eof.to_string().contains(&format!("pos={pos_after_eof}")),
      "wrong position in error message: {eof}"
    );

    // This verifies that an invalid position is reported relative to the
    // slice, rather than being transformed to its position in the parent.
    let mut slice = input.slice("slice", 33, slice_size + 15)?;
    Self::assert_correct_impl(false, &slice)?;
    let eof = slice.seek(pos_after_eof).unwrap_err();
    assert!(matches!(&eof, LuceneError::Eof(_)));
    assert!(
      eof.to_string().contains(&format!("pos={pos_after_eof}")),
      "wrong position in error message: {eof}"
    );

    CloseableRef::close(&slice)?;
    CloseableRef::close(&input)?;
    CloseableRef::close(&mmap_dir)?;
    Ok(())
  }

  fn test_clone_safety<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let temp_dir = create_temp_dir_with_prefix("testCloneSafety")?;
    let mmap_dir = self.get_directory(temp_dir.path().to_path_buf(), random)?;
    {
      let mut io = mmap_dir.create_output("bytes", &new_io_context(random)?)?;
      io.write_vint(5)?;
    }

    let mut one = mmap_dir.open_input("bytes", &IOContext::default_io_context()?)?;
    let mut two = one.try_clone()?;
    let mut three = two.try_clone()?;
    CloseableRef::close(&one)?;

    assert!(matches!(
      one.read_vint(),
      Err(LuceneError::AlreadyClosed(_))
    ));
    assert!(matches!(
      two.read_vint(),
      Err(LuceneError::AlreadyClosed(_))
    ));
    assert!(matches!(
      three.read_vint(),
      Err(LuceneError::AlreadyClosed(_))
    ));

    CloseableRef::close(&two)?;
    CloseableRef::close(&three)?;
    CloseableRef::close(&one)?;
    Ok(())
  }

  fn test_clone_slice_safety<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let temp_dir = create_temp_dir_with_prefix("testCloneSliceSafety")?;
    let mmap_dir = self.get_directory(temp_dir.path().to_path_buf(), random)?;
    {
      let mut io = mmap_dir.create_output("bytes", &new_io_context(random)?)?;
      io.write_int(1)?;
      io.write_int(2)?;
    }

    let slicer = mmap_dir.open_input("bytes", &new_io_context(random)?)?;
    let mut one = slicer.slice("first int", 0, 4)?;
    let mut two = slicer.slice("second int", 4, 4)?;
    let mut three = one.try_clone()?;
    let mut four = two.try_clone()?;
    CloseableRef::close(&slicer)?;

    assert!(matches!(one.read_int(), Err(LuceneError::AlreadyClosed(_))));
    assert!(matches!(two.read_int(), Err(LuceneError::AlreadyClosed(_))));
    assert!(matches!(
      three.read_int(),
      Err(LuceneError::AlreadyClosed(_))
    ));
    assert!(matches!(
      four.read_int(),
      Err(LuceneError::AlreadyClosed(_))
    ));

    CloseableRef::close(&one)?;
    CloseableRef::close(&two)?;
    CloseableRef::close(&three)?;
    CloseableRef::close(&four)?;
    CloseableRef::close(&slicer)?;
    Ok(())
  }

  fn test_implementations<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    for i in 2..12 {
      let chunk_size = 1usize << i;
      let temp_dir = create_temp_dir_with_prefix("testImplementations")?;
      let mmap_dir =
        self.get_directory_with_max_chunk_size(temp_dir.path().to_path_buf(), chunk_size)?;
      let io_context = new_io_context(random)?;
      let size = random.random_range(0..chunk_size * 2) + 3;
      let mut bytes = vec![0_u8; size];
      random.fill(&mut bytes[..]);
      {
        let mut io = mmap_dir.create_output("bytes", &io_context)?;
        io.write_bytes_with_len(&bytes, bytes.len())?;
      }

      let mut ii = mmap_dir.open_input("bytes", &new_io_context(random)?)?;
      let mut actual = vec![0_u8; size];
      let actual_len = actual.len();
      DataInput::read_bytes(&mut ii, &mut actual, 0, actual_len)?;
      assert_eq!(bytes, actual);
      ii.seek(0)?;

      Self::assert_correct_impl(size < chunk_size, &ii)?;

      let slice_size = random.random_range(0..size);
      let slice = ii.slice("slice", 0, slice_size)?;
      Self::assert_correct_impl(slice_size < chunk_size, &slice)?;

      let offset = random.random_range(1..size);
      let slice_size = random.random_range(0..=size - offset);
      let slice = ii.slice("slice", offset, slice_size)?;
      Self::assert_correct_impl(offset % chunk_size + slice_size < chunk_size, &slice)?;
    }
    Ok(())
  }

  fn assert_correct_impl(_is_single: bool, _input: &MemorySegmentIndexInput) -> Result<()> {
    test_not_required_in_rust_lucene!();
  }
}

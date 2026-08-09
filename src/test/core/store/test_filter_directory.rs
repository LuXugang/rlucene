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
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)] // for quick search
struct TestFilterDirectory;

#[test]
#[ignore = "Java-only: overridden Directory methods are inspected through Java reflection"]
fn test_overrides() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = ""]
fn test_unwrap() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

mod base_directory_test_case_tests {
  use crate::core::util::error::lucene_error::Result;

  #[test]
  #[ignore = ""]
  fn test_copy_from() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_rename() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_delete_file() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_byte() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_short() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_int() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_long() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_aligned_little_endian_longs() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_unaligned_little_endian_longs() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_little_endian_longs_underflow() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_aligned_ints() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_unaligned_ints() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_ints_underflow() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_aligned_floats() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_unaligned_floats() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_floats_underflow() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_string() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_vint() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_vlong() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_zint() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_zlong() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_set_of_strings() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_map_of_strings() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_checksum() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_detect_close() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_thread_safety_in_list_all() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_file_exists_in_list_after_created() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_seek_to_eof_then_back() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_illegal_eof() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_seek_past_eof() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_slice_out_of_bounds() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_no_dir() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_copy_bytes() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_copy_bytes_with_threads() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_fsync_doesnt_create_new_files() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_random_long() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_random_int() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_random_short() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_random_byte() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_slice_of_slice() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_large_writes() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_index_output_to_string() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_double_close_output() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_double_close_input() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_create_temp_output() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_create_output_for_existing_file() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_seek_to_end_of_file() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_seek_beyond_end_of_file() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_pending_deletions() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_list_all_is_sorted() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_data_types() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_group_vint_overflow() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_group_vint() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_prefetch() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_prefetch_on_slice() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_update_read_advice() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_is_loaded() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  #[ignore = ""]
  fn test_is_loaded_on_slice() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }
}

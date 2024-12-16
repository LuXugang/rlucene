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
use crate::common::my_random;
use crate::store::base_directory_test_case::BaseDirectoryTestCase;
use crate::util::test_error::TestError;
use rlucene::store::directory::Directory;
use rlucene::store::fs_directory::FSDirectory;
use rlucene::store::nio_fs_directory::NIOFSDirectory;
use std::path::PathBuf;

#[allow(dead_code)] // for quick search
struct TestNIOFSDirectory;

impl BaseDirectoryTestCase for TestNIOFSDirectory {
    fn get_directory(&self, path: PathBuf) -> Result<impl Directory, TestError> {
        let sub_directory = NIOFSDirectory::new();
        Ok(FSDirectory::new(path, sub_directory)?)
    }
}

#[test]
fn test_copy_from() -> Result<(), TestError> {
    let mut random = my_random("test_copy_from".to_string());
    let test = TestNIOFSDirectory;
    test.test_copy_from(&mut random)
}
#[test]
fn test_rename() -> Result<(), TestError> {
    let mut random = my_random("test_test_rename".to_string());
    let test = TestNIOFSDirectory;
    test.test_rename(&mut random)
}
#[test]
fn test_delete_file() -> Result<(), TestError> {
    let test = TestNIOFSDirectory;
    test.test_delete_file()
}
#[test]
fn test_byte() -> Result<(), TestError> {
    let mut random = my_random("test_byte".to_string());
    let test = TestNIOFSDirectory;
    test.test_byte(&mut random)
}
#[test]
fn test_short() -> Result<(), TestError> {
    let mut random = my_random("test_short".to_string());
    let test = TestNIOFSDirectory;
    test.test_short(&mut random)
}
#[test]
fn test_int() -> Result<(), TestError> {
    let mut random = my_random("test_int".to_string());
    let test = TestNIOFSDirectory;
    test.test_int(&mut random)
}
#[test]
fn test_long() -> Result<(), TestError> {
    let mut random = my_random("test_long".to_string());
    let test = TestNIOFSDirectory;
    test.test_long(&mut random)
}
#[test]
fn test_aligned_little_endian_longs() -> Result<(), TestError> {
    let mut random = my_random("test_aligned_little_endian_longs".to_string());
    let test = TestNIOFSDirectory;
    test.test_aligned_little_endian_longs(&mut random)
}
#[test]
fn test_unaligned_little_endian_longs() -> Result<(), TestError> {
    let mut random = my_random("test_unaligned_little_endian_longs".to_string());
    let test = TestNIOFSDirectory;
    test.test_unaligned_little_endian_longs(&mut random)
}
#[test]
fn test_little_endian_longs_underflow() -> Result<(), TestError> {
    let mut random = my_random("test_little_endian_longs_underflow".to_string());
    let test = TestNIOFSDirectory;
    test.test_little_endian_longs_underflow(&mut random)
}
#[test]
fn test_aligned_ints() -> Result<(), TestError> {
    let mut random = my_random("test_aligned_ints".to_string());
    let test = TestNIOFSDirectory;
    test.test_aligned_ints(&mut random)
}
#[test]
fn test_unaligned_ints() -> Result<(), TestError> {
    let mut random = my_random("test_unaligned_ints".to_string());
    let test = TestNIOFSDirectory;
    test.test_unaligned_ints(&mut random)
}
#[test]
fn test_ints_underflow() -> Result<(), TestError> {
    let mut random = my_random("test_ints_underflow".to_string());
    let test = TestNIOFSDirectory;
    test.test_ints_underflow(&mut random)
}
#[test]
fn test_aligned_floats() -> Result<(), TestError> {
    let mut random = my_random("test_aligned_floats".to_string());
    let test = TestNIOFSDirectory;
    test.test_aligned_floats(&mut random)
}
#[test]
fn test_unaligned_floats() -> Result<(), TestError> {
    let mut random = my_random("test_unaligned_floats".to_string());
    let test = TestNIOFSDirectory;
    test.test_unaligned_floats(&mut random)
}
#[test]
fn test_floats_underflow() -> Result<(), TestError> {
    let mut random = my_random("test_floats_underflow".to_string());
    let test = TestNIOFSDirectory;
    test.test_floats_underflow(&mut random)
}
#[test]
fn test_string() -> Result<(), TestError> {
    let mut random = my_random("test_string".to_string());
    let test = TestNIOFSDirectory;
    test.test_string(&mut random)
}
#[test]
fn test_vint() -> Result<(), TestError> {
    let mut random = my_random("test_vint".to_string());
    let test = TestNIOFSDirectory;
    test.test_vint(&mut random)
}
#[test]
fn test_vlong() -> Result<(), TestError> {
    let mut random = my_random("test_vlong".to_string());
    let test = TestNIOFSDirectory;
    test.test_vlong(&mut random)
}
#[test]
fn test_zint() -> Result<(), TestError> {
    let mut random = my_random("test_zint".to_string());
    let test = TestNIOFSDirectory;
    test.test_zint(&mut random)
}
#[test]
fn test_zlong() -> Result<(), TestError> {
    let mut random = my_random("test_zlong".to_string());
    let test = TestNIOFSDirectory;
    test.test_zlong(&mut random)
}
#[test]
fn test_set_of_strings() -> Result<(), TestError> {
    let mut random = my_random("test_set_of_strings".to_string());
    let test = TestNIOFSDirectory;
    test.test_set_of_strings(&mut random)
}
#[test]
fn test_map_of_strings() -> Result<(), TestError> {
    let mut random = my_random("test_map_of_strings".to_string());
    let test = TestNIOFSDirectory;
    test.test_map_of_strings(&mut random)
}
#[test]
fn test_checksum() -> Result<(), TestError> {
    let mut random = my_random("test_checksum".to_string());
    let test = TestNIOFSDirectory;
    test.test_checksum(&mut random)
}
#[test]
fn test_thread_safety_in_list_all() -> Result<(), TestError> {
    let mut random = my_random("test_thread_safety_in_list_all".to_string());
    let test = TestNIOFSDirectory;
    test.test_thread_safety_in_list_all(&mut random)
}
#[test]
fn test_file_exists_in_list_after_created() -> Result<(), TestError> {
    let mut random = my_random("test_file_exists_in_list_after_created".to_string());
    let test = TestNIOFSDirectory;
    test.test_file_exists_in_list_after_created(&mut random)
}
#[test]
fn test_seek_to_eof_then_back() -> Result<(), TestError> {
    let mut random = my_random("test_seek_to_eof_then_back".to_string());
    let test = TestNIOFSDirectory;
    test.test_seek_to_eof_then_back(&mut random)
}
#[test]
fn test_illegal_eof() -> Result<(), TestError> {
    let mut random = my_random("test_illegal_eof".to_string());
    let test = TestNIOFSDirectory;
    test.test_illegal_eof(&mut random)
}
#[test]
fn test_seek_past_eof() -> Result<(), TestError> {
    let mut random = my_random("test_seek_past_eof".to_string());
    let test = TestNIOFSDirectory;
    test.test_seek_past_eof(&mut random)
}
#[test]
fn test_slice_out_of_bounds() -> Result<(), TestError> {
    let mut random = my_random("test_slice_out_of_bounds".to_string());
    let test = TestNIOFSDirectory;
    test.test_slice_out_of_bounds(&mut random)
}
#[test]
fn test_no_dir() -> Result<(), TestError> {
    //TODO
    Ok(())
}

#[test]
fn test_copy_bytes() -> Result<(), TestError> {
    let mut random = my_random("test_copy_bytes".to_string());
    let test = TestNIOFSDirectory;
    test.test_copy_bytes(&mut random)
}
#[test]
fn test_copy_bytes_with_threads() -> Result<(), TestError> {
    //TODO
    Ok(())
}
#[test]
fn test_fsync_doesnt_create_new_files() -> Result<(), TestError> {
    let mut random = my_random("test_fsync_doesnt_create_new_files".to_string());
    let test = TestNIOFSDirectory;
    test.test_fsync_doesnt_create_new_files(&mut random)
}
#[test]
fn test_random_long() -> Result<(), TestError> {
    let mut random = my_random("test_random_long".to_string());
    let test = TestNIOFSDirectory;
    test.test_random_long(&mut random)
}
#[test]
fn test_random_int() -> Result<(), TestError> {
    let mut random = my_random("test_random_int".to_string());
    let test = TestNIOFSDirectory;
    test.test_random_int(&mut random)
}
#[test]
fn test_random_short() -> Result<(), TestError> {
    let mut random = my_random("test_random_short".to_string());
    let test = TestNIOFSDirectory;
    test.test_random_short(&mut random)
}
#[test]
fn test_random_byte() -> Result<(), TestError> {
    let mut random = my_random("test_random_byte".to_string());
    let test = TestNIOFSDirectory;
    test.test_random_byte(&mut random)
}
#[test]
fn test_slice_of_slice() -> Result<(), TestError> {
    let mut random = my_random("test_slice_of_slice".to_string());
    let test = TestNIOFSDirectory;
    test.test_slice_of_slice(&mut random)
}
#[test]
fn test_large_writes() -> Result<(), TestError> {
    let mut random = my_random("test_large_writes".to_string());
    let test = TestNIOFSDirectory;
    test.test_large_writes(&mut random)
}
#[test]
fn test_index_output_to_string() -> Result<(), TestError> {
    let mut random = my_random("test_index_output_to_string".to_string());
    let test = TestNIOFSDirectory;
    test.test_index_output_to_string(&mut random)
}
#[test]
fn test_create_temp_output() -> Result<(), TestError> {
    let mut random = my_random("test_create_temp_output".to_string());
    let test = TestNIOFSDirectory;
    test.test_create_temp_output(&mut random)
}
#[test]
fn test_create_output_for_existing_file() -> Result<(), TestError> {
    let test = TestNIOFSDirectory;
    test.test_create_output_for_existing_file()
}
#[test]
fn test_seek_to_end_of_file() -> Result<(), TestError> {
    let test = TestNIOFSDirectory;
    test.test_seek_to_end_of_file()
}
#[test]
fn test_seek_beyond_end_of_file() -> Result<(), TestError> {
    let test = TestNIOFSDirectory;
    test.test_seek_beyond_end_of_file()
}
#[test]
fn test_pending_deletions() -> Result<(), TestError> {
    let mut random = my_random("test_pending_deletions".to_string());
    let test = TestNIOFSDirectory;
    test.test_pending_deletions(&mut random)
}
#[test]
fn test_list_all_is_sorted() -> Result<(), TestError> {
    let mut random = my_random("test_list_all_is_sorted".to_string());
    let test = TestNIOFSDirectory;
    test.test_list_all_is_sorted(&mut random)
}
#[test]
fn test_data_types() -> Result<(), TestError> {
    let test = TestNIOFSDirectory;
    test.test_data_types()
}

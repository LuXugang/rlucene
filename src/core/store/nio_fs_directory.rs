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
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::path::Path;

use crate::core::store::fs_directory_base::FSDirectoryBase;
use crate::core::store::nio_fs_index_input::NIOFSIndexInput;
use crate::core::store::{BufferedIndexInput, IOContext};
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// An implementation of
/// [`FSDirectory`](crate::core::store::fs_directory::FSDirectory)that uses
/// `std::fs::File` for positional reads, allowing multiple threads to read from
/// the same file without synchronization.
///
/// # Read and Write Modes
///
/// This struct uses `std::fs::File` for reading, enabling thread-safe
/// concurrent reads. Writing is achieved using
/// [`OutputStreamIndexOutput`](crate::core::store::output_stream_index_output).
pub struct NIOFSDirectory;

impl Default for NIOFSDirectory {
    fn default() -> Self {
        Self::new()
    }
}

impl NIOFSDirectory {
    pub fn new() -> Self {
        Self
    }
}

impl Display for NIOFSDirectory {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "NIOFSDirectory")
    }
}

/// this method should only be called in
/// [`FSDirectory::open_input`](crate::core::store::fs_directory::FSDirectory), which
/// will first check whether file could be read
impl FSDirectoryBase for NIOFSDirectory {
    type Output = BufferedIndexInput<NIOFSIndexInput>;
    fn open_input(&self, name: &str, context: &IOContext, path: &Path) -> Result<Self::Output> {
        let file_path = path.join(name);
        let file_name = file_path.to_string_lossy().to_string();
        let file = match File::open(file_path) {
            Ok(file) => file,
            Err(err) => {
                return Err(LuceneError::io_with_path(file_name, err));
            },
        };
        let resource_desc = format!("NIOFSIndexInput(path=\"{}\")", path.display());
        // let resource_desc_string = resource_desc.to_string();
        let index_input = NIOFSIndexInput::new(file, &resource_desc);
        BufferedIndexInput::with_io_context(index_input, &resource_desc, context)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::core::store::nio_fs_directory::NIOFSDirectory;
    use crate::core::store::nio_fs_index_input::NIOFSIndexInput;
    use crate::core::store::{BufferedIndexInput, FSDirectory, NativeFSLockFactory};
    use crate::core::util::error::lucene_error::Result;
    use crate::test::store::base_directory_test_case::BaseDirectoryTestCase;
    use crate::test::util::lucene_test_case::lucene_test_case_util::random;

    #[allow(dead_code)] // for quick search
    struct TestNIOFSDirectory;

    impl BaseDirectoryTestCase for TestNIOFSDirectory {
        type Directory = FSDirectory<NativeFSLockFactory, NIOFSDirectory>;
        type Output = BufferedIndexInput<NIOFSIndexInput>;
        fn get_directory(&self, path: PathBuf) -> Result<Self::Directory> {
            let sub_directory = NIOFSDirectory::new();
            FSDirectory::new(path, sub_directory)
        }
    }

    #[test]
    fn test_copy_from() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_copy_from(&mut random)
    }
    #[test]
    fn test_rename() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_rename(&mut random)
    }
    #[test]
    fn test_delete_file() -> Result<()> {
        let test = TestNIOFSDirectory;
        test.test_delete_file()
    }
    #[test]
    fn test_byte() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_byte(&mut random)
    }
    #[test]
    fn test_short() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_short(&mut random)
    }
    #[test]
    fn test_int() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_int(&mut random)
    }
    #[test]
    fn test_long() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_long(&mut random)
    }
    #[test]
    fn test_aligned_little_endian_longs() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_aligned_little_endian_longs(&mut random)
    }
    #[test]
    fn test_unaligned_little_endian_longs() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_unaligned_little_endian_longs(&mut random)
    }
    #[test]
    fn test_little_endian_longs_underflow() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_little_endian_longs_underflow(&mut random)
    }
    #[test]
    fn test_aligned_ints() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_aligned_ints(&mut random)
    }
    #[test]
    fn test_unaligned_ints() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_unaligned_ints(&mut random)
    }
    #[test]
    fn test_ints_underflow() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_ints_underflow(&mut random)
    }
    #[test]
    fn test_aligned_floats() -> Result<()> {
        let test = TestNIOFSDirectory;
        test.test_aligned_floats()
    }
    #[test]
    fn test_unaligned_floats() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_unaligned_floats(&mut random)
    }
    #[test]
    fn test_floats_underflow() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_floats_underflow(&mut random)
    }
    #[test]
    fn test_string() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_string(&mut random)
    }
    #[test]
    fn test_vint() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_vint(&mut random)
    }
    #[test]
    fn test_vlong() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_vlong(&mut random)
    }
    #[test]
    fn test_zint() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_zint(&mut random)
    }
    #[test]
    fn test_zlong() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_zlong(&mut random)
    }
    #[test]
    fn test_set_of_strings() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_set_of_strings(&mut random)
    }
    #[test]
    fn test_map_of_strings() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_map_of_strings(&mut random)
    }
    #[test]
    fn test_checksum() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_checksum(&mut random)
    }
    #[test]
    fn test_thread_safety_in_list_all() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_thread_safety_in_list_all(&mut random)
    }
    #[test]
    fn test_file_exists_in_list_after_created() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_file_exists_in_list_after_created(&mut random)
    }
    #[test]
    fn test_seek_to_eof_then_back() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_seek_to_eof_then_back(&mut random)
    }
    #[test]
    fn test_illegal_eof() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_illegal_eof(&mut random)
    }
    #[test]
    fn test_seek_past_eof() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_seek_past_eof(&mut random)
    }
    #[test]
    fn test_slice_out_of_bounds() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_slice_out_of_bounds(&mut random)
    }
    #[test]
    fn test_no_dir() -> Result<()> {
        //TODO
        Ok(())
    }

    #[test]
    fn test_copy_bytes() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_copy_bytes(&mut random)
    }
    #[test]
    fn test_copy_bytes_with_threads() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_copy_bytes_with_threads(&mut random)
    }
    #[test]
    fn test_fsync_doesnt_create_new_files() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_fsync_doesnt_create_new_files(&mut random)
    }
    #[test]
    fn test_random_long() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_random_long(&mut random)
    }
    #[test]
    fn test_random_int() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_random_int(&mut random)
    }
    #[test]
    fn test_random_short() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_random_short(&mut random)
    }
    #[test]
    fn test_random_byte() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_random_byte(&mut random)
    }
    #[test]
    fn test_slice_of_slice() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_slice_of_slice(&mut random)
    }
    #[test]
    fn test_large_writes() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_large_writes(&mut random)
    }
    #[test]
    fn test_index_output_to_string() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_index_output_to_string(&mut random)
    }
    #[test]
    fn test_create_temp_output() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_create_temp_output(&mut random)
    }
    #[test]
    fn test_create_output_for_existing_file() -> Result<()> {
        let test = TestNIOFSDirectory;
        test.test_create_output_for_existing_file()
    }
    #[test]
    fn test_seek_to_end_of_file() -> Result<()> {
        let test = TestNIOFSDirectory;
        test.test_seek_to_end_of_file()
    }
    #[test]
    fn test_seek_beyond_end_of_file() -> Result<()> {
        let test = TestNIOFSDirectory;
        test.test_seek_beyond_end_of_file()
    }
    #[test]
    fn test_pending_deletions() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_pending_deletions(&mut random)
    }
    #[test]
    fn test_list_all_is_sorted() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_list_all_is_sorted(&mut random)
    }
    #[test]
    fn test_data_types() -> Result<()> {
        let test = TestNIOFSDirectory;
        test.test_data_types()
    }
    #[test]
    fn test_group_vint_overflow() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_group_vint_overflow(&mut random)
    }
    #[test]
    fn test_group_vint() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_group_vint(&mut random)
    }
    #[test]
    fn test_prefetch() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_prefetch(&mut random)
    }
    #[test]
    fn test_prefetch_on_slice() -> Result<()> {
        let mut random = random();
        let test = TestNIOFSDirectory;
        test.test_prefetch_on_slice(&mut random)
    }
    #[test]
    fn test_is_loaded() -> Result<()> {
        //TODO
        Ok(())
    }
    #[test]
    fn test_is_loaded_on_slice() -> Result<()> {
        //TODO
        Ok(())
    }
}

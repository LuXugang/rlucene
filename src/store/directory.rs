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
use crate::index::IndexFileNames;
use crate::store::buffered_checksum_index_input::BufferedChecksumIndexInput;
use crate::store::data_output::DataOutput;
use crate::store::index_input::IndexInput;
use crate::store::lock::Lock;
use crate::store::{IOContext, IndexOutput};
use crate::util::error::data_io_error_enum::DataIOError;
use std::collections::HashSet;
use std::fmt::Display;

/// A `Directory` provides an abstraction layer for storing a list of files. A directory contains only files (no sub-folder hierarchy).
///
/// # Implementation Notes
/// Implementing types must comply with the following:
/// - A file in a directory can be created (`create_output`), appended to, and then closed.
/// - A file open for writing may not be available for read access until the corresponding [`IndexOutput`] is closed.
/// - Once a file is created, it must only be opened for input (`open_input`) or deleted (`delete_file`).
///   Calling `create_output` on an existing file must return an error similar to `FileAlreadyExistsError`.
///
/// # Note
/// If your application requires external synchronization, you should **not** synchronize on the `Directory` implementation instance
/// as this may cause deadlock. Instead, use your own synchronization primitives.
///
/// # See Also
/// [`FSDirectory`](crate::store::fs_directory::FSDirectory)
/// [`ByteBuffersDirectory`](crate::store::byte_buffers_directory::ByteBuffersDirectory)
/// [`FilterDirectory`](crate::store::filter_directory::FilterDirectory)
#[allow(dead_code)]
pub trait Directory: Display + Sized {
    /// Returns the names of all files stored in this directory. The output must be sorted
    /// in UTF-8 order (using `str::cmp` for comparison).
    ///
    /// # Errors
    /// Returns an `std::io::Error` in case of an I/O error.
    fn list_all(&self) -> Vec<String>;
    /// Removes an existing file in the directory.
    ///
    /// # Errors
    /// This method must return an error if `name` points to a non-existing file:
    /// - [`std::io::ErrorKind::NotFound`] for non-existing files.
    ///
    /// Returns an `std::io::Error` in case of other I/O errors.
    ///
    /// # Arguments
    /// * `name` - The name of an existing file to be removed.
    fn delete_file(&self, name: &str) -> Result<(), DataIOError>;

    /// Returns the byte length of a file in the directory.
    ///
    /// # Errors
    /// This method must return an error if `name` points to a non-existing file:
    /// - [`std::io::ErrorKind::NotFound`] for non-existing files.
    ///
    /// Returns an `std::io::Error` in case of other I/O errors.
    ///
    /// # Arguments
    /// * `name` - The name of an existing file.
    fn file_length(&self, name: &str) -> u64;
    /// Creates a new, empty file in the directory and returns an `IndexOutput` instance for
    /// appending data to this file.
    ///
    /// # Errors
    /// This method must return an error if the file already exists:
    /// - [`std::io::ErrorKind::AlreadyExists`] for existing files.
    ///
    /// Returns an `std::io::Error` in case of other I/O errors.
    ///
    /// # Arguments
    /// * `name` - The name of the file to create.
    fn create_output(&self, name: &str, context: IOContext) -> impl IndexOutput;

    /// Creates a new, empty, temporary file in the directory and returns an `IndexOutput` instance
    /// for appending data to this file.
    ///
    /// The temporary file name (accessible via `IndexOutput::get_name`) will start with `prefix`,
    /// end with `suffix`, and have a reserved file extension `.tmp`.
    ///
    /// # Arguments
    /// * `prefix` - The prefix for the temporary file name.
    /// * `suffix` - The suffix for the temporary file name.
    fn create_temp_output(
        &self,
        prefix: &str,
        suffix: &str,
        context: IOContext,
    ) -> Result<impl IndexOutput, DataIOError>;
    /// Ensures that any writes to these files are moved to stable storage (made durable).
    ///
    /// Lucene uses this to properly commit changes to the index, preventing corruption in case of a
    /// machine or OS crash.
    ///
    /// # See Also
    /// [`sync_metadata`](Directory::sync_metadata)
    fn sync(&self, names: Vec<&str>);
    /// Ensures that directory metadata, such as recent file renames, are moved to stable storage.
    ///
    /// # See Also
    /// [`sync`](Directory::sync)
    fn sync_metadata(&self);
    /// Renames `source` file to `dest` file where `dest` must not already exist in the directory.
    ///
    /// It is permitted for this operation to not be truly atomic, meaning both `source` and `dest`
    /// could temporarily be visible in the list of files. However, the implementation must ensure that
    /// the content of `dest` appears as the entire `source` atomically. Once `dest` is visible for
    /// readers, the entire content of the previous `source` must be visible.
    ///
    /// This method is used by `IndexWriter` to publish commits.
    ///
    /// # Arguments
    /// * `source` - The file to rename.
    /// * `dest` - The new name for the file.
    fn rename(&self, source: &str, dest: &str) -> Result<(), DataIOError>;

    /// Opens a stream for reading an existing file.
    ///
    /// # Errors
    /// This method must return an error if `name` points to a non-existing file:
    /// - [`std::io::ErrorKind::NotFound`] for non-existing files.
    ///
    /// Returns an `std::io::Error` in case of other I/O errors.
    ///
    /// # Arguments
    /// * `name` - The name of an existing file.
    fn open_input(&self, name: &str, context: IOContext) -> Result<impl IndexInput, DataIOError>;

    /// Opens a checksum-computing stream for reading an existing file.
    ///
    /// # Errors
    /// This method must return an error if `name` points to a non-existing file:
    /// - [`std::io::ErrorKind::NotFound`] for non-existing files.
    ///
    /// Returns an `std::io::Error` in case of other I/O errors.
    ///
    /// # Arguments
    /// * `name` - The name of an existing file.
    fn open_checksum_input<T: IndexInput>(
        &self,
        name: &str,
    ) -> Result<BufferedChecksumIndexInput<impl IndexInput + Sized>, DataIOError> {
        Ok(BufferedChecksumIndexInput::new(
            self.open_input(name, IOContext::read_once_io_context()?)?,
        ))
    }

    /// Acquires and returns a `Lock` for a file with the given name.
    ///
    /// # Errors
    /// - Returns a `LockObtainFailedException` (optional specific exception) if the lock could not be
    ///   obtained because it is currently held elsewhere.
    /// - Returns an `std::io::Error` if any I/O error occurs attempting to gain the lock.
    ///
    /// # Arguments
    /// * `name` - The name of the lock file.
    fn obtain_lock(&self, name: &str) -> Result<impl Lock, DataIOError>;
    /// Copies an existing `src` file from directory `from` to a non-existent file `dest` in this directory.
    /// The given `IOContext` is only used for opening the destination file.
    ///
    /// # Arguments
    /// * `src` - The source file to copy.
    /// * `from` - The directory containing the source file.
    /// * `dest` - The destination file in this directory.
    /// * `io_context` - The I/O context used for opening the destination file.
    fn copy_from(
        &self,
        from: &impl Directory,
        src: &str,
        dest: &str,
        context: IOContext,
    ) -> Result<(), DataIOError> {
        let mut success = false;

        let result = (|| -> Result<(), DataIOError> {
            let mut is = from.open_input(src, IOContext::read_once_io_context()?)?;
            let mut os = self.create_output(dest, context);
            let length = is.length();
            os.copy_bytes(&mut is, length)?;
            success = true;
            Ok(())
        })();

        if !success {
            self.delete_files_ignoring_exceptions(&[dest.to_string()]);
        }

        result
    }
    fn delete_files_ignoring_exceptions(&self, files: &[String]) {
        for name in files {
            if self.delete_file(name).is_err() {
                // ignore
            }
        }
    }
    /// Returns a set of files currently pending deletion in this directory.
    ///
    /// # Note
    /// This is an internal API.
    fn get_pending_deletions(&self) -> HashSet<String>;

    /// Creates a file name for a temporary file. The name will start with `prefix`, end with
    /// `suffix`, and have a reserved file extension `.tmp`.
    ///
    /// # See Also
    /// [`create_temp_output`](Directory::create_temp_output)
    fn get_temp_file_name(prefix: &str, suffix: &str, counter: u64) -> String {
        //base-36
        let counter_str = format!("{:x}", counter);
        let full_suffix = format!("{}_{}", suffix, counter_str);
        IndexFileNames::segment_file_name(prefix, &full_suffix, "tmp")
    }
}

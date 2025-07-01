/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use std::fs::File;
use std::io;
use std::path::PathBuf;

use crate::store::directory::Directory;
use crate::util::error::lucene_error::{LuceneError, Result};

pub struct IOUtils;
impl IOUtils {
    /// Deletes all given files, suppressing all thrown errors.
    ///
    /// Note: The `files` collection should not be empty or contain `None`.
    pub fn delete_files_ignoring_exceptions(dir: &mut impl Directory, files: &[&String]) {
        for name in files {
            if dir.delete_file(name).is_err() {
                // Ignore the error and continue with the next file.
            }
        }
    }
    pub fn delete_files(dir: &mut impl Directory, names: &[&str]) -> Result<()> {
        for name in names {
            dir.delete_file(name)?;
        }
        Ok(())
    }

    /// Ensure that any writes to the given file are written to the storage
    /// device.
    ///
    /// # Arguments
    ///
    /// * `file_to_sync` - The path to the file or directory to sync.
    /// * `is_dir` - If `true`, the given path is a directory. On platforms
    ///   where directory syncing is unsupported (like Windows), this will be
    ///   ignored for directories.
    pub fn fsync(file_to_sync: &PathBuf, is_dir: bool) -> Result<()> {
        if is_dir {
            if cfg!(windows) {
                if !file_to_sync.exists() {
                    return Err(LuceneError::not_found(format!(
                        "Directory not found: {}",
                        file_to_sync.display()
                    )));
                }
                return Ok(());
            }

            let dir_file =
                File::options()
                    .read(true)
                    .open(file_to_sync)
                    .map_err(|e| match e.kind() {
                        io::ErrorKind::NotFound => LuceneError::not_found(format!(
                            "Directory not found: {}",
                            file_to_sync.display()
                        )),
                        _ => {
                            LuceneError::io_with_path(file_to_sync.to_string_lossy().to_string(), e)
                        },
                    })?;

            if let Err(_e) = dir_file.sync_all() {
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                debug_assert!(
                    false,
                    "On Linux and macOS, syncing a directory should not throw an error. Got: {_e}"
                );
                return Ok(());
            }
        } else {
            let file = File::options()
                .write(true)
                .open(file_to_sync)
                .map_err(|e| {
                    LuceneError::io_with_path(file_to_sync.to_string_lossy().to_string(), e)
                })?;

            file.sync_all().map_err(|e| {
                LuceneError::io_with_path(
                    file_to_sync.to_string_lossy().to_string(),
                    io::Error::new(e.kind(), format!("Failed to sync file: {e}")),
                )
            })?;
        }

        Ok(())
    }
}

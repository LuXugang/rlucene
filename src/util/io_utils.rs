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

use crate::util::error::data_io_error_enum::RuntimeError;
use std::fs::File;
use std::io;
use std::path::PathBuf;

pub struct IOUtils;
impl IOUtils {
    /// Ensure that any writes to the given file are written to the storage device.
    ///
    /// # Arguments
    ///
    /// * `file_to_sync` - The path to the file or directory to sync.
    /// * `is_dir` - If `true`, the given path is a directory. On platforms where directory syncing
    ///   is unsupported (like Windows), this will be ignored for directories.
    pub fn fsync(file_to_sync: &PathBuf, is_dir: bool) -> Result<(), RuntimeError> {
        if is_dir {
            if cfg!(windows) {
                if !file_to_sync.exists() {
                    return Err(RuntimeError::not_found(format!(
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
                        io::ErrorKind::NotFound => RuntimeError::not_found(format!(
                            "Directory not found: {}",
                            file_to_sync.display()
                        )),
                        _ => RuntimeError::io_with_path(
                            file_to_sync.to_string_lossy().to_string(),
                            e,
                        ),
                    })?;

            if let Err(_e) = dir_file.sync_all() {
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                debug_assert!(
                    false,
                    "On Linux and macOS, syncing a directory should not throw an error. Got: {}",
                    _e
                );
                return Ok(());
            }
        } else {
            let file = File::options()
                .write(true)
                .open(file_to_sync)
                .map_err(|e| {
                    RuntimeError::io_with_path(file_to_sync.to_string_lossy().to_string(), e)
                })?;

            file.sync_all().map_err(|e| {
                RuntimeError::io_with_path(
                    file_to_sync.to_string_lossy().to_string(),
                    io::Error::new(e.kind(), format!("Failed to sync file: {}", e)),
                )
            })?;
        }

        Ok(())
    }
}

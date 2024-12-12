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
use crate::store::fs_lock_factory::FSLockFactory;
use crate::store::lock::{FSLockEnum, Lock};
use crate::store::lock_factory::LockFactory;
use crate::util::error::data_io_error_enum::DataIOError;
use chrono::{DateTime, Utc};
use fs2::FileExt;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::{File, Metadata, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

/// Implements [`lock_factory`](crate::store::lock_factory) using native OS file locks.
///
/// # Notes
/// - This `lock_factory` relies on `std::fs` and native OS file locking APIs. Any issues with these
///   APIs may cause locking to fail. For example, in certain NFS environments, native file locks
///   might fail (allowing locks to be acquired twice incorrectly), whereas
///   [`simple_fs_lock_factory`](crate::store::simple_fs_lock_factory) works correctly
///   in those environments.
/// - For NFS-based access to an index, it is recommended to try
///   [`simple_fs_lock_factory`](crate::store::simple_fs_lock_factory) first and handle its
///   limitation: a lock file may remain if the process exits abnormally.
///
/// # Advantages
/// The primary advantage of `native_fs_lock_factory` is that locks (but not the lock files themselves)
/// will be properly released by the operating system if the process exits abnormally.
///
/// # Lock File Behavior
/// Unlike [`simple_fs_lock_factory`](crate::store::simple_fs_lock_factory), leftover lock
/// files in the filesystem are acceptable because the OS will release the locks even if the files
/// remain. This implementation will not actively remove these lock files, so they might be visible,
/// but this does not mean the index is locked.
///
/// # Implementation Change Warning
/// Special care is required when changing the locking implementation:
/// - Ensure no writer is currently writing to the index before making changes, as this could corrupt the index.
/// - Apply the `lock_factory` change across all instances using the index.
/// - Clean up leftover lock files before starting the new configuration.
///
/// Different locking implementations are not compatible and cannot work together.
///
/// # See Also
/// - [`lock_factory`](crate::store::lock_factory)

pub struct NativeFSLockFactory {
    lock_held: Arc<Mutex<HashSet<String>>>,
}

impl Default for NativeFSLockFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeFSLockFactory {
    /// Creates a new instance.
    pub fn new() -> Self {
        Self {
            lock_held: get_lock_held(),
        }
    }
}
impl LockFactory for NativeFSLockFactory {
    fn obtain_lock(&self, dir: &PathBuf, lock_name: &str) -> Result<FSLockEnum, DataIOError> {
        FSLockFactory::obtain_lock(self, dir, lock_name)
    }
}

impl Display for NativeFSLockFactory {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "NativeFSLockFactory")
    }
}

impl FSLockFactory for NativeFSLockFactory {
    fn obtain_fs_lock(&self, dir: &PathBuf, lock_name: &str) -> Result<FSLockEnum, DataIOError> {
        let dir_name = dir.to_string_lossy().to_string();
        fs::create_dir_all(dir).map_err(|e|DataIOError::io_with_path(dir_name,e))?;

        let lock_file = dir.join(lock_name);

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&lock_file)
            .map_err(|e|DataIOError::io_with_path(lock_file.to_string_lossy().to_string(),e))?;

        let real_path = lock_file.canonicalize().map_err(|e|DataIOError::io_with_path(lock_file.to_string_lossy().to_string(),e))?;
        let real_path_str = real_path.to_string_lossy().to_string();

        let mut lock_held = self.lock_held.lock().unwrap();
        if !lock_held.insert(real_path_str.clone()) {
            return Err(DataIOError::lock_already_held(format!(
                "Lock held by another program: {}",
                real_path_str
            )));
        }

        match file.try_lock_exclusive() {
            Ok(_) => {
                let metadata = file.metadata()?;
                let lock = NativeFSLock {
                    file,
                    path: real_path,
                    metadata,
                };
                Ok(FSLockEnum::Native(lock))
            }
            Err(_) => {
                lock_held.remove(&real_path_str);
                Err(DataIOError::lock_held_by_other(format!(
                    "Lock held by this virtual machine: {}",
                    real_path_str
                )))
            }
        }
    }
}

impl Drop for NativeFSLock {
    fn drop(&mut self) {
        let real_path_str = self.path.to_string_lossy().to_string();
        let locks = get_lock_held();
        let mut lock_held = locks.lock().unwrap();
        lock_held.remove(&real_path_str);
    }
}

static LOCK_HELD: OnceLock<Arc<Mutex<HashSet<String>>>> = OnceLock::new();

fn get_lock_held() -> Arc<Mutex<HashSet<String>>> {
    LOCK_HELD
        .get_or_init(|| Arc::new(Mutex::new(HashSet::new())))
        .clone()
}

pub struct NativeFSLock {
    file: File,
    path: PathBuf,
    metadata: Metadata,
}

impl NativeFSLock {
    fn format_metadata(&self) -> String {
        let size = self.metadata.len();
        let permissions = self.metadata.permissions();
        let modified_time = self.metadata.modified().ok().map_or_else(
            || "unknown".to_string(),
            |time| match time.duration_since(SystemTime::UNIX_EPOCH) {
                Ok(duration) => {
                    let datetime = DateTime::<Utc>::from(SystemTime::UNIX_EPOCH + duration);
                    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                }
                Err(_) => "invalid time".to_string(),
            },
        );
        format!(
            "size: {} bytes, permissions: {:?}, modified: {}",
            size, permissions, modified_time
        )
    }
}

impl Display for NativeFSLock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NativeFSLock(path= {}, file_metadata= {})",
            self.path.display(),
            self.format_metadata()
        )
    }
}

impl Lock for NativeFSLock {
    /// Ensures the validity of the current lock.
    ///
    /// # Errors
    /// - Returns `DataIOError::illegal_state` if:
    ///   - The lock file is no longer in the global lock map.
    ///   - The file lock is no longer valid.
    ///   - The lock file size is not 0.
    ///   - The lock file has been deleted or is inaccessible.
    fn ensure_valid(&self) -> Result<(), DataIOError> {
        let lock_held = LOCK_HELD.get_or_init(|| Arc::new(Mutex::new(HashSet::new())));
        let lock_held = lock_held.lock().unwrap();
        if !lock_held.contains(&self.path.to_string_lossy().to_string()) {
            return Err(DataIOError::illegal_state(format!(
                "Lock path unexpectedly cleared from map: {:?}",
                self.path
            )));
        }

        if self.file.try_lock_exclusive().is_ok() {
            return Err(DataIOError::illegal_state(format!(
                "File lock invalidated by an external force: {:?}",
                self.path
            )));
        }

        let metadata = self.file.metadata().map_err(DataIOError::io)?;
        if metadata.len() != 0 {
            return Err(DataIOError::illegal_state(format!(
                "Unexpected lock file size: {}, (lock: {:?})",
                metadata.len(),
                self.path
            )));
        }

        if !self.path.exists() {
            return Err(DataIOError::illegal_state(format!(
                "Lock file deleted or inaccessible: {:?}",
                self.path
            )));
        }

        Ok(())
    }
}

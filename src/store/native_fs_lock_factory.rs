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
use std::fmt::{write, Display, Formatter};
use crate::store::base_directory::BaseDirectory;
use crate::store::directory::Directory;
use crate::store::fs_directory::FSDirectory;
use crate::store::fs_lock_factory::{FSLockFactory};
use crate::store::lock::{FSLockEnum, Lock};
use crate::store::lock_factory::LockFactory;
use crate::util::error::data_io_error_enum::DataIOError;

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
/// # Singleton Instance
/// This implementation is designed as a singleton. Use [`INSTANCE`](Self::INSTANCE).
///
/// # See Also
/// - [`lock_factory`](crate::store::lock_factory)

pub struct NativeFSLockFactory;

impl NativeFSLockFactory {
    /// Creates a new instance.
    pub fn new() -> Self {
        NativeFSLockFactory
    }
}
impl LockFactory for NativeFSLockFactory {

    fn obtain_lock(&mut self, dir: &mut impl Directory, lock_name: &str) -> Result<FSLockEnum, DataIOError> {
        FSLockFactory::obtain_lock(self, dir, lock_name)
    }
}

impl Display for NativeFSLockFactory {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl FSLockFactory for NativeFSLockFactory{
    fn obtain_fs_lock(&self, dir: &mut impl Directory, lock_name: &str) -> Result<FSLockEnum, DataIOError> {
        todo!()
    }
 
}


pub struct NativeFSLock;

impl Lock for NativeFSLock {
    fn ensure_valid() {
        todo!()
    }
}
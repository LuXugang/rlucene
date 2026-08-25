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
use std::path::Path;

use crate::core::store::NativeFSLockFactory;
use crate::core::store::lock_factory::LockFactory;
use crate::core::util::error::lucene_error::Result;

/// Base trait for file-system-based locking implementations.
///
/// The Rust [`LockFactory`] API receives a filesystem path directly, so it does
/// not need Java's runtime [`Directory`](crate::core::store::directory::Directory)-to-[`FSDirectory`](crate::core::store::fs_directory::FSDirectory) type check.
pub trait FSLockFactory: LockFactory {
  /// Returns the default locking implementation for this platform.
  ///
  /// This method always returns
  /// [`native_fs_lock_factory`](NativeFSLockFactory).
  fn obtain_lock(&self, directory: &Path, lock_name: &str) -> Result<Self::Lock> {
    self.obtain_fs_lock(directory, lock_name)
  }

  /// Gets a lock for a `fs_directory` instance.
  ///
  /// # Errors
  /// Returns an `io::Error` if the lock could not be obtained.
  ///
  /// # Note
  /// Implement this method to define how the lock should be acquired.
  fn obtain_fs_lock(&self, directory: &Path, lock_name: &str) -> Result<Self::Lock>;
}

pub(crate) fn get_default() -> NativeFSLockFactory {
  NativeFSLockFactory::new()
}

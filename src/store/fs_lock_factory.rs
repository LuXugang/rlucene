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

use crate::store::lock::FSLockEnum;
use crate::store::lock_factory::LockFactory;
use crate::store::NativeFSLockFactory;
use crate::util::error::lucene_error::Result;

/// Base struct for file-system-based locking implementation.
/// This struct is explicitly checking that
/// the passed [`Directory`](crate::store::directory::Directory)
/// is an [`FSDirectory`](crate::store::fs_directory::FSDirectory).
pub trait FSLockFactory: LockFactory {
    /// Returns the default locking implementation for this platform.
    ///
    /// This method always returns
    /// [`native_fs_lock_factory`](NativeFSLockFactory).
    fn obtain_lock(&self, directory: &Path, lock_name: &str) -> Result<FSLockEnum> {
        self.obtain_fs_lock(directory, lock_name)
    }

    /// Gets a lock for a `fs_directory` instance.
    ///
    /// # Errors
    /// Returns an `io::Error` if the lock could not be obtained.
    ///
    /// # Note
    /// Implement this method to define how the lock should be acquired.
    fn obtain_fs_lock(&self, directory: &Path, lock_name: &str) -> Result<FSLockEnum>;
}
#[allow(unused)]
pub(crate) fn get_default() -> impl FSLockFactory {
    NativeFSLockFactory::new()
}

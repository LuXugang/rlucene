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
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use crate::store::base_directory::BaseDirectory;
use crate::store::directory::Directory;
use crate::store::fs_directory::FSDirectory;
use crate::store::lock::{FSLockEnum, Lock};
use crate::store::lock_factory::LockFactory;
use crate::store::mmap_directory::MMapDirectory;
use crate::store::{ByteBuffersIndexInput, ByteBuffersIndexOutput, IOContext, IndexOutput, NativeFSLock, NativeFSLockFactory};
use crate::store::index_input::IndexInput;
use crate::store::nio_fs_directory::NIOFSDirectory;
use crate::store::raf_directory::RAFDirectory;
use crate::util::error::data_io_error_enum::DataIOError;

/// Base class for file system based locking implementation. This class is explicitly checking that
/// the passed [`Directory`](crate::store::directory::Directory) is an [`FSDirectory`](crate::store::fs_directory::FSDirectory).
pub trait FSLockFactory: LockFactory {
    /// Returns the default locking implementation for this platform.
    ///
    /// This method always returns [`native_fs_lock_factory`](crate::store::native_fs_lock_factory::NativeFSLockFactory).
   
    
    fn obtain_lock(&self, directory: &mut impl Directory, lock_name: &str) -> Result<FSLockEnum, DataIOError> {
        self.obtain_fs_lock(directory, lock_name)
    }
    
    /// Obtains a lock for a `fs_directory` instance.
    ///
    /// # Errors
    /// Returns an `io::Error` if the lock could not be obtained.
    ///
    /// # Note
    /// Implement this method to define how the lock should be acquired.
    fn obtain_fs_lock(&self, directory: &mut impl Directory, lock_name: &str) -> Result<FSLockEnum, DataIOError>;
}
pub(crate) fn get_default() -> impl FSLockFactory {
    NativeFSLockFactory::new()
}
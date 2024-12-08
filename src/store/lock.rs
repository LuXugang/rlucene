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
use crate::store::NativeFSLock;
use crate::store::simple_fs_lock::SimpleFSLock;

/// An interprocess mutex lock.
///
/// # Example
/// Typical use might look like:
///
/// ```text
/// let lock = directory.obtain_lock("my.lock")?;
/// // ... code to execute while locked ...
/// ```
///
/// # See Also
/// [`Directory::obtain_lock`](crate::store::directory::Directory::obtain_lock)
///
/// # Note
/// This is an internal API.
pub trait Lock {
    /// Best effort check that this lock is still valid. Locks could become invalidated externally for
    /// a number of reasons, such as if a user deletes the lock file manually or when a network
    /// filesystem is in use.
    ///
    /// # Errors
    /// Returns an `std::io::Error` if the lock is no longer valid.
    fn ensure_valid();
}

pub enum FSLockEnum{
    Native(NativeFSLock), 
    Simple(SimpleFSLock)
}

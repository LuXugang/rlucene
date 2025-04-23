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
use std::fmt::Display;
use std::path::Path;

use crate::store::lock::FSLockEnum;
use crate::util::error::lucene_error::Result;

/// Base trait for locking implementations. `Directory` uses instances of this
/// trait to implement locking.
///
/// # Default Implementation
/// Lucene uses [`NativeFSLockFactory`](crate::store::NativeFSLockFactory) by
/// default for `FSDirectory`-based index directories.
///
/// # Note
/// Special care needs to be taken if you change the locking implementation:
/// First, ensure that no writer is actively writing to the index, as doing so
/// could corrupt the index. Be sure to change the `LockFactory` on all Lucene
/// instances and clean up any leftover lock files before starting with the new
/// configuration. Different implementations cannot work together.
pub trait LockFactory: Display {
    /// Returns a new got `Lock` instance identified by `lock_name`.
    ///
    /// # Arguments
    /// * `lock_name` - The name of the lock to be created.
    ///
    /// # Errors
    /// - Returns a `LockObtainFailedException` (optional specific exception) if
    ///   the lock could not be obtained because it is currently held elsewhere.
    /// - Returns an `std::io::Error` if any I/O error occurs attempting to gain
    ///   the lock.
    fn obtain_lock(&self, dir: &Path, lock_name: &str) -> Result<FSLockEnum>;
}

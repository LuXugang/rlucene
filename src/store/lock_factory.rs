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
use std::fmt::Display;
use std::path::Path;

use crate::store::lock::Lock;
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
    type Lock: Lock;
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
    fn obtain_lock(&self, dir: &Path, lock_name: &str) -> Result<Self::Lock>;
}

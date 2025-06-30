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
use std::path::Path;

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
#[allow(unused)]
pub(crate) fn get_default() -> impl FSLockFactory {
    NativeFSLockFactory::new()
}

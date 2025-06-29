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
use std::fmt::{Display, Formatter};

use crate::store::simple_fs_lock::SimpleFSLock;
use crate::store::NativeFSLock;
use crate::util::error::lucene_error::Result;

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
pub trait Lock: Display {
    /// Best effort check that this lock is still valid. Locks could become
    /// invalidated externally for a number of reasons, such as if a user
    /// deletes the lock file manually or when a network filesystem is in
    /// use.
    ///
    /// # Errors
    /// Returns an `LuceneError` if the lock is no longer valid.
    fn ensure_valid(&self) -> Result<()>;
}

pub enum FSLockEnum {
    Native(NativeFSLock),
    Simple(SimpleFSLock),
}

impl Display for FSLockEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FSLockEnum::Native(native_lock) => write!(f, "{}", native_lock),
            FSLockEnum::Simple(simple_lock) => write!(f, "{}", simple_lock),
        }
    }
}

impl Lock for FSLockEnum {
    fn ensure_valid(&self) -> Result<()> {
        match self {
            FSLockEnum::Native(native_lock) => native_lock.ensure_valid(),
            FSLockEnum::Simple(simple_lock) => simple_lock.ensure_valid(),
        }
    }
}

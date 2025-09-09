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
use std::fmt::{Display, Formatter};

use crate::core::util::error::lucene_error::Result;

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
/// [`Directory::obtain_lock`](crate::core::store::directory::Directory::obtain_lock)
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

pub enum Either2Lock<A, B> {
    A(A),
    B(B),
}

impl<A, B> Display for Either2Lock<A, B>
where
    A: Lock,
    B: Lock,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Either2Lock::A(f_lock) => f_lock.fmt(f),
            Either2Lock::B(s) => s.fmt(f),
        }
    }
}

impl<A, B> Lock for Either2Lock<A, B>
where
    A: Lock,
    B: Lock,
{
    fn ensure_valid(&self) -> Result<()> {
        match self {
            Either2Lock::A(f) => f.ensure_valid(),
            Either2Lock::B(s) => s.ensure_valid(),
        }
    }
}

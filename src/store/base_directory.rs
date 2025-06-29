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
use crate::store::directory::Directory;
use crate::store::lock::Lock;
use crate::util::error::lucene_error::Result;

/// Base implementation for a concrete [`Directory`] that uses a
/// [`LockFactory`](crate::store::lock_factory::LockFactory) for locking.
///
/// # Note
/// This is an experimental API.
///
/// # Special Note
/// This trait could actually be removed because `LockFactory` has been moved to
/// the implementation of `Directory`,
/// such as [`FSDirectory`](crate::store::fs_directory::FSDirectory). However,
/// it is temporarily retained to maintain consistency with the structure of
/// Java Lucene.
pub trait BaseDirectory: Directory {
    fn obtain_lock(&mut self, name: &str) -> Result<impl Lock>;
}

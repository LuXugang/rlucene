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
use crate::index::index_commit::IndexCommit;
use crate::util::error::lucene_error::Result;
/// This [`IndexDeletionPolicy`] implementation keeps only the most recent commit and
/// immediately removes all prior commits after a new commit is done. This is the default deletion
/// policy.
pub trait IndexDeletionPolicy {
    /// Deletes all commits except the most recent one.
    fn on_init<IC>(&mut self, commits: Vec<IC>) -> Result<()>;

    /// Deletes all commits except the most recent one.
    fn on_commit<IC>(&mut self, commits: Vec<IC>) -> Result<()>
    where
        IC: IndexCommit;
}

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
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use crate::index::standard_directory_reader::StandardDirectoryReader;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
use parking_lot::Mutex;

pub trait IndexCommit: PartialEq + Eq + PartialOrd + Ord + Display {
    /// Returns the segments file (`segments_N`) associated with this commit point.
    fn get_segments_file_name(&self) -> &str;
    /// Returns all index files referenced by this commit point.
    fn get_file_names(&self) -> Result<&[String]>;
    type Directory: Directory;
    /// Returns the [`Directory`] for the index.
    fn get_directory(&self) -> Arc<Mutex<Self::Directory>>;
    /// Delete this commit point. This only applies when using the commit point in the context of
    /// `IndexWriter`’s `IndexDeletionPolicy`.
    ///
    /// Upon calling this, the writer is notified that this commit point should be deleted.
    ///
    /// Decision that a commit-point should be deleted is taken by the [`IndexDeletionPolicy`](crate::index::index_deletion_policy::IndexDeletionPolicy)
    /// in effect and therefore this should only be called by its
    /// [`IndexDeletionPolicy::on_init()`](crate::index::index_deletion_policy::IndexDeletionPolicy::on_init) or [`IndexDeletionPolicy::on_commit()`](crate::index::index_deletion_policy::IndexDeletionPolicy::on_commit) methods.
    fn delete(&mut self) -> Result<()>;
    /// Returns `true` if this commit should be deleted; this is only used by [`IndexWriter`](crate::index::index_writer::IndexWriter) after
    /// invoking the [`IndexDeletionPolicy`](crate::index::index_deletion_policy::IndexDeletionPolicy).
    fn is_deleted(&self) -> bool;
    /// Returns number of segments referenced by this commit.
    fn get_segment_count(&self) -> usize;
    /// Returns the generation (the _N in segments_N) for this IndexCommit
    fn get_generation(&self) -> i64;
    /// Returns `user_data`, previously passed to [`IndexWriter::set_live_commit_data()`](crate::index::index_writer::IndexWriter::set_live_commit_data) for this commit. The map is `String` → `String`.
    fn user_data(&self) -> &HashMap<String, String>;
    fn get_reader(&self) -> Option<StandardDirectoryReader> {
        None
    }
}
pub mod index_commit_util {
    use crate::index::index_commit::IndexCommit;
    use std::cmp::Ordering;
    use std::sync::Arc;

    pub fn is_same_commit<T>(a: &T, b: &T) -> bool
    where
        T: IndexCommit,
    {
        Arc::ptr_eq(&a.get_directory(), &b.get_directory())
            && a.get_generation() == b.get_generation()
    }
    pub fn cmp_commit<T>(a: &T, b: &T) -> Option<Ordering>
    where
        T: IndexCommit,
    {
        debug_assert!(Arc::ptr_eq(&a.get_directory(), &b.get_directory()));
        Some(a.get_generation().cmp(&b.get_generation()))
    }
}

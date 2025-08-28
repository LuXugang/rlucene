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
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use crate::index::standard_directory_reader::StandardDirectoryReader;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;

pub trait IndexCommit: PartialEq + Eq + PartialOrd + Ord + Display {
    /// Returns the segments file (`segments_N`) associated with this commit point.
    fn get_segments_file_name(&self) -> &str;
    /// Returns all index files referenced by this commit point.
    fn get_file_names(&self) -> Result<&[String]>;
    type Directory: Directory;
    /// Returns the [`Directory`] for the index.
    fn get_directory(&self) -> Arc<Self::Directory>;
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
use std::cmp::Ordering;

pub fn is_same_commit<T>(a: &T, b: &T) -> bool
where
    T: IndexCommit,
{
    Arc::ptr_eq(&a.get_directory(), &b.get_directory()) && a.get_generation() == b.get_generation()
}
pub fn cmp_commit<T>(a: &T, b: &T) -> Option<Ordering>
where
    T: IndexCommit,
{
    debug_assert!(Arc::ptr_eq(&a.get_directory(), &b.get_directory()));
    Some(a.get_generation().cmp(&b.get_generation()))
}

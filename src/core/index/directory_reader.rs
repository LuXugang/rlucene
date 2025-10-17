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
use crate::core::index::base_composite_reader::BaseCompositeReader;
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;
/// [`DirectoryReader`] is an implementation of [`CompositeReader`](crate::core::index::composite_reader::CompositeReader) that can read indexes
/// from a [`Directory`].
///
/// [`DirectoryReader`] instances are usually constructed by calling one of the static
/// `open()` methods, for example `DirectoryReader::open(directory)`.
///
/// For efficiency, in this API documents are often referred to via *document numbers*,
/// non-negative integers that uniquely identify documents within the index.
/// These document numbers are ephemeral — they may change as documents are added to or
/// deleted from an index. Clients should therefore **not rely** on a document having the
/// same number between sessions.
///
///
/// ## Thread Safety
///
/// **NOTE:** [`IndexReader`](crate::core::index::index_reader::IndexReader) instances are completely thread-safe, meaning multiple threads
/// can invoke any of its methods concurrently.
/// If your application requires external synchronization, you should **not** synchronize
/// on the `IndexReader` instance itself; instead, use your own (non-Lucene) synchronization
/// objects.
pub trait DirectoryReader: BaseCompositeReader {
    type DirectoryReader: DirectoryReader;
    /// If this reader does not support reopen, return `None` so that client code behaves correctly.
    /// This should be consistent with [`is_current`](Self::is_current),
    /// which should always return `true` if reopen is not supported.
    ///
    /// # Returns
    ///
    /// - `Ok(None)` if there are no changes.
    /// - `Ok(Some(new_reader))` if a new [`DirectoryReader`] instance should be created.
    ///
    /// # Errors
    ///
    /// Returns an error if a low-level I/O failure occurs.
    fn do_open_if_changed(&self) -> Result<Option<Self::DirectoryReader>>;
    /// If this reader does not support reopen, return `None` so that client code behaves correctly.
    /// This should be consistent with [`is_current`](Self::is_current),
    /// which should always return `true` if reopen is not supported.
    ///
    /// # Returns
    ///
    /// - `Ok(None)` if there are no changes.
    /// - `Ok(Some(new_reader))` if a new [`DirectoryReader`] instance should be created.
    ///
    /// # Errors
    ///
    /// Returns an error if a low-level I/O failure occurs.
    fn do_open_if_changed_with_commit<IC>(
        &self,
        commit: IC,
    ) -> Result<Option<Self::DirectoryReader>>
    where
        IC: IndexCommit;
    /// If this reader does not support reopen from an [`IndexWriter`],
    /// this method should return an [`unsupported_operation`](crate::core::util::error::lucene_error::LuceneError::unsupported_operation) error.
    ///
    /// # Returns
    ///
    /// - `Ok(None)` if there are no changes.
    /// - `Ok(Some(new_reader))` if a new [`DirectoryReader`] instance should be created.
    ///
    /// # Errors
    ///
    /// Returns an error if a low-level I/O failure occurs.
    fn do_open_if_changed_with_index_writer<D, L, B>(
        &self,
        writer: IndexWriter<D, L, B>,
        apply_deletes: bool,
    ) -> Result<Self::DirectoryReader>
    where
        D: Directory,
        L: LiveIndexWriterConfig,
        B: IndexWriterBase;
    /// Version number when this `IndexReader` was opened.
    ///
    /// This method returns the version recorded in the commit that the reader opened.
    /// The version number is advanced every time a change is made using an [`IndexWriter`].
    fn get_version(&self) -> i64;
    /// Check whether any new changes have occurred to the index since this reader was opened.
    ///
    /// If this reader was created by calling `open`, then this method checks if any
    /// further commits (see [`IndexWriter::commit`]) have occurred in the directory.
    ///
    /// If instead this reader is a near real-time reader (ie, obtained by a call to
    /// `DirectoryReader::open` with an [`IndexWriter`], or by calling
    /// `open_if_changed_with_reader` on a near real-time reader), then this method checks
    /// if either a new commit has occurred, or any new uncommitted changes have taken place via the
    /// writer. Note that even if the writer has only performed merging, this method will still return
    /// false.
    ///
    /// In any event, if this returns false, you should call `open_if_changed_with_reader`
    /// to get a new reader that sees the changes.
    ///
    /// # Errors
    ///
    /// Returns an error if there is a low-level I/O error.
    fn is_current(&self) -> bool;
    type IndexCommit: IndexCommit;
    /// Expert: return the IndexCommit that this reader has opened.
    fn get_index_commit(&self) -> Result<Self::IndexCommit>;
    type Directory: Directory;
    /// The index directory
    fn directory(&self) -> Arc<Self::Directory>;
}

pub mod directory_reader_util {
    use crate::core::index::IndexFileNames;
    use crate::core::store::directory::Directory;
    use crate::core::util::error::lucene_error::Result;

    pub fn index_exists(directory: &impl Directory) -> Result<bool> {
        // LUCENE-2812, LUCENE-2727, LUCENE-4738: this logic will
        // return true in cases that should arguably be false,
        // such as only IW.prepareCommit has been called, or a
        // corrupt first commit, but it's too deadly to make
        // this logic "smarter" and risk accidentally returning
        // false due to various cases like file description
        // exhaustion, access denied, etc., because in that
        // case IndexWriter may delete the entire index.  It's
        // safer to err towards "index exists" than try to be
        // smart about detecting not-yet-fully-committed or
        // corrupt indices.  This means that IndexWriter will
        // throw an exception on such indices and the app must
        // resolve the situation manually:
        let files = directory.list_all()?; // returns Vec<String>

        let prefix = format!("{}_", IndexFileNames::SEGMENTS);
        for file in files {
            if file.starts_with(&prefix) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

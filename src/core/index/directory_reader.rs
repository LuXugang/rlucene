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
    fn do_open_if_changed(&mut self) -> Result<Option<Self::DirectoryReader>>;
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
        &mut self,
        commit: Option<&IC>,
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
    fn do_open_if_changed_with_index_writer<L, B>(
        &self,
        writer: IndexWriter<Self::Directory, L, B>,
        apply_deletes: bool,
    ) -> Result<Self::DirectoryReader>
    where
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
    fn is_current<D, L, B>(&self, index_writer: &IndexWriter<D, L, B>) -> Result<bool>
    where
        D: Directory,
        L: LiveIndexWriterConfig,
        B: IndexWriterBase;
    type IndexCommit: IndexCommit;
    /// Expert: return the IndexCommit that this reader has opened.
    fn get_index_commit(&self) -> Result<Self::IndexCommit>;
    type Directory: Directory;
    /// The index directory
    fn directory(&self) -> &DirectoryReaderBase<Self::Directory>;
}

pub mod directory_reader_util {
    use crate::core::index::IndexFileNames;
    use crate::core::index::dummy::dummy_index_commit::DummyIndexCommit;
    use crate::core::index::index_commit::IndexCommit;

    use crate::core::index::dummy::dummy_leaf_reader::DummyLeafReader;
    use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
    use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
    use crate::core::index::segment_reader::SegmentReader;
    use crate::core::index::standard_directory_reader::{
        StandardDirectoryReader, StandardDirectoryReaderType,
    };
    use crate::core::store::directory::Directory;
    use crate::core::util::Comparator;
    use crate::core::util::dummy::dummy_comparator::DummyComparator;
    use crate::core::util::error::lucene_error::Result;
    use std::sync::Arc;

    /// Returns an [`IndexReader`](crate::core::index::index_reader::IndexReader) reading the index in the given [`Directory`].
    ///
    /// # Parameters
    ///
    /// * `directory` – the index directory.
    ///
    /// # Errors
    ///
    /// Returns an error if there is a low-level I/O error.
    pub fn open<D>(
        directory: Arc<D>,
    ) -> Result<StandardDirectoryReader<Arc<SegmentReader<D>>, DummyComparator, D>>
    where
        D: Directory,
    {
        StandardDirectoryReader::<DummyLeafReader, _, _>::open::<DummyIndexCommit<D>>(
            directory, None, None,
        )
    }

    /// Returns an [`IndexReader`](crate::core::index::index_reader::IndexReader) for the index in the given [`Directory`].
    ///
    /// # Parameters
    ///
    /// * `directory` – the index directory.
    /// * `leaf_sorter` – a comparator for sorting leaf readers.
    ///   Providing `leaf_sorter` is useful for indices expected to run many queries with particular sort
    ///   criteria (e.g., for time-based indices this is usually a descending sort on timestamp).
    ///   In this case, `leaf_sorter` should sort leaves according to this sort criteria.
    ///   Providing `leaf_sorter` allows speeding up this particular type of sort queries by early
    ///   termination while iterating through segments and their documents.
    ///
    /// # Errors
    ///
    /// Returns an error if there is a low-level I/O error.
    pub fn open_with_sorter<D, C>(
        directory: Arc<D>,
        leaf_sorter: Option<C>,
    ) -> Result<StandardDirectoryReader<Arc<SegmentReader<D>>, C, D>>
    where
        D: Directory,
        C: Comparator<Arc<SegmentReader<D>>>,
    {
        StandardDirectoryReader::open::<DummyIndexCommit<D>>(directory, None, leaf_sorter)
    }
    /// Opens a near real-time `IndexReader` from the given [`IndexWriter`].
    ///
    /// # Arguments
    ///
    /// * `writer` - The [`IndexWriter`] to open from.
    ///
    /// # Returns
    ///
    /// The newly created `IndexReader`.
    ///
    /// # Errors
    ///
    /// * [`CorruptIndex`](crate::core::util::error::lucene_error::LuceneError::corrupt_index) – If the index is corrupt.
    /// * [`Io`](crate::core::util::error::lucene_error::LuceneError::io) – If a low-level I/O error occurs.
    pub fn open_with_writer<D, L, B>(
        writer: &IndexWriter<D, L, B>,
    ) -> Result<StandardDirectoryReaderType<D>>
    where
        D: Directory,
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
    {
        open_with_writer_deletes(writer, true, false)
    }
    /// Expert: Opens a near real-time `IndexReader` from the given [`IndexWriter`],
    /// controlling whether past deletions should be applied.
    ///
    /// # Arguments
    ///
    /// * `writer` - The [`IndexWriter`] to open from.
    /// * `apply_all_deletes` - If `true`, all buffered deletes will be applied (made visible)
    ///   in the returned reader.
    ///   If `false`, the deletes remain buffered in the `IndexWriter` and will be applied later.
    ///   Applying deletes can be costly, so if your application can tolerate deleted documents
    ///   being returned, you may gain some performance by passing `false`.
    /// * `write_all_deletes` - If `true`, new deletes will be written down to index files instead of
    ///   being carried over directly in heap from writer to reader.
    ///
    /// # See also
    ///
    /// [`open`](open_with_writer)
    ///
    /// # Lucene
    ///
    /// This API is marked as **experimental** in Lucene.
    pub fn open_with_writer_deletes<D, L, B>(
        writer: &IndexWriter<D, L, B>,
        apply_all_deletes: bool,
        write_all_deletes: bool,
    ) -> Result<StandardDirectoryReaderType<D>>
    where
        D: Directory,
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
    {
        writer.get_reader(apply_all_deletes, write_all_deletes)
    }

    /// Expert: returns an [`IndexReader`](crate::core::index::index_reader::IndexReader) reading the index in the given `IndexCommit`.
    ///
    /// # Parameters
    ///
    /// * `commit` – the commit point to open.
    ///
    /// # Errors
    ///
    /// Returns an error if there is a low-level I/O error.
    pub fn open_with_commit<D, C, IC>(
        commit: &IC,
    ) -> Result<StandardDirectoryReader<Arc<SegmentReader<D>>, C, D>>
    where
        D: Directory,
        C: Comparator<Arc<SegmentReader<D>>>,
        IC: IndexCommit<Directory = D>,
    {
        StandardDirectoryReader::open(commit.get_directory(), Some(commit), None)
    }

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
    /// Expert: returns an [`IndexReader`](crate::core::index::index_reader::IndexReader) reading the index on the given `IndexCommit`.
    ///
    /// This method allows opening indices that were created with a Lucene version older than N-1,
    /// provided that all codecs for this index are available in the classpath and the segment file
    /// format used was created with Lucene 7 or newer.
    /// Users of this API must be aware that Lucene does not guarantee semantic compatibility for
    /// indices created with versions older than N-1. All backwards compatibility aside from the file
    /// format is optional and applied on a best-effort basis.
    ///
    /// # Parameters
    ///
    /// * `commit` – the commit point to open
    /// * `min_supported_major_version` – the minimum supported major index version
    /// * `leaf_sorter` – a comparator for sorting leaf readers.
    ///   Providing `leaf_sorter` is useful for indices expected to run many queries with particular sort
    ///   criteria (e.g., for time-based indices, this is usually a descending sort on timestamp).
    ///   In this case, `leaf_sorter` should sort leaves according to this sort criteria.
    ///   Providing `leaf_sorter` allows speeding up this type of sort queries by early termination
    ///   while iterating through segments and their documents.
    ///
    /// # Errors
    ///
    /// Returns an error if there is a low-level I/O error.
    pub fn open_with_commit_version_sorter<D, C, IC>(
        commit: &IC,
        min_supported_major_version: i32,
        leaf_sorter: Option<C>,
    ) -> Result<StandardDirectoryReader<Arc<SegmentReader<D>>, C, D>>
    where
        D: Directory,
        C: Comparator<Arc<SegmentReader<D>>>,
        IC: IndexCommit<Directory = D>,
    {
        StandardDirectoryReader::open_with_version(
            commit.get_directory(),
            min_supported_major_version,
            Some(commit),
            leaf_sorter,
        )
    }
}
pub struct DirectoryReaderBase<D> {
    pub directory: Arc<D>,
}
impl<D> DirectoryReaderBase<D>
where
    D: Directory,
{
    pub fn new(directory: Arc<D>) -> Self {
        Self { directory }
    }
}

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
use crate::core::index::IndexFileNames;
use crate::core::index::base_composite_reader::BaseCompositeReader;
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::dummy::dummy_index_commit::DummyIndexCommit;
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::segment_infos::{SegmentInfos, generation_from_segments_file_name};
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::sync::Arc;

use crate::core::index::live_index_writer_config::LeafSorter;
use crate::core::index::standard_directory_reader::{ReaderCommit, StandardDirectoryReader};
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
pub trait DirectoryReader:
  BaseCompositeReader<SubReader = <Self as CompositeReader>::LeafReader>
{
  type DirectoryReader: DirectoryReader;
  type Directory: Directory;
  /// The index directory
  fn directory(&self) -> &DirectoryReaderBase<Self::Directory>;
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
  /// If this reader does not support reopening from a specific [`IndexCommit`], return an
  /// [`unsupported_operation`](crate::core::util::error::lucene_error::LuceneError::unsupported_operation)
  /// error.
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
    commit: Option<&IC>,
  ) -> Result<Option<Self::DirectoryReader>>
  where
    IC: IndexCommit<Directory = Arc<Self::Directory>>;
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
  fn do_open_if_changed_with_deletes(
    &self,
    writer: &Arc<IndexWriter<Self::Directory>>,
    apply_deletes: bool,
  ) -> Result<Option<Self::DirectoryReader>>;
  /// Version number when this `IndexReader` was opened.
  ///
  /// This method returns the version recorded in the commit that the reader opened.
  /// The version number is advanced every time a change is made using an [`IndexWriter`].
  fn get_version(&self) -> Result<i64>;
  /// Check whether any new changes have occurred to the index since this reader was opened.
  ///
  /// If this reader was created by calling `open`, then this method checks if any
  /// further commits (see `IndexWriter::commit`) have occurred in the directory.
  ///
  /// If instead this reader is a near real-time reader (ie, obtained by a call to
  /// [`open_from_writer`], or by calling [`open_if_changed`] on a near real-time reader), then this method checks
  /// if either a new commit has occurred, or any new uncommitted changes have taken place via the
  /// writer. Note that even if the writer has only performed merging, this method will still return
  /// false.
  ///
  /// In any event, if this returns false, you should call [`open_if_changed`]
  /// to get a new reader that sees the changes.
  ///
  /// # Errors
  ///
  /// Returns an error if there is a low-level I/O error.
  fn is_current(&self) -> Result<bool>;
  type IndexCommit: IndexCommit;
  /// Expert: return the IndexCommit that this reader has opened.
  fn get_index_commit(&self) -> Result<Self::IndexCommit>;
}

/// Returns an [`IndexReader`](crate::core::index::index_reader::IndexReader) reading the index in the given [`Directory`].
///
/// # Parameters
///
/// * `directory` – the index directory.
///
/// # Errors
///
/// Returns an error if there is a low-level I/O error.
pub fn open<D>(directory: Arc<D>) -> Result<StandardDirectoryReader<D>>
where
  D: Directory + 'static,
{
  StandardDirectoryReader::open::<DummyIndexCommit<D>>(directory, None, None)
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
pub fn open_with_sorter<D>(
  directory: Arc<D>,
  leaf_sorter: Option<LeafSorter<D>>,
) -> Result<StandardDirectoryReader<D>>
where
  D: Directory + 'static,
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
pub fn open_from_writer<D>(writer: &Arc<IndexWriter<D>>) -> Result<StandardDirectoryReader<D>>
where
  D: Directory + 'static,
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
/// [`open`](open_from_writer)
///
/// # Lucene
///
/// This API is marked as **experimental** in Lucene.
pub fn open_with_writer_deletes<D>(
  writer: &Arc<IndexWriter<D>>,
  apply_all_deletes: bool,
  write_all_deletes: bool,
) -> Result<StandardDirectoryReader<D>>
where
  D: Directory + 'static,
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
pub fn open_from_commit<D, IC>(commit: &IC) -> Result<StandardDirectoryReader<D>>
where
  D: Directory + 'static,
  IC: IndexCommit<Directory = Arc<D>>,
{
  StandardDirectoryReader::open(commit.get_directory(), Some(commit), None)
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
pub fn open_with_version<D, IC>(
  commit: &IC,
  min_supported_major_version: i32,
  leaf_sorter: Option<LeafSorter<D>>,
) -> Result<StandardDirectoryReader<D>>
where
  D: Directory + 'static,
  IC: IndexCommit<Directory = Arc<D>>,
{
  StandardDirectoryReader::open_with_version(
    commit.get_directory(),
    min_supported_major_version,
    Some(commit),
    leaf_sorter,
  )
}

/// If the index has changed since the provided reader was opened, open and return a new reader;
/// otherwise, return `None`. The new reader, if any, will be the same type of reader as the
/// previous one, i.e. an NRT reader will open a new NRT reader.
///
/// This method is typically far less costly than opening a fully new [`DirectoryReader`] as it
/// shares resources, for example sub-readers, with the provided reader when possible.
///
/// The provided reader is not closed; callers are responsible for closing it. If a new reader is
/// returned, callers must eventually close it too.
///
/// # Returns
///
/// `None` if there are no changes; otherwise, a new reader instance which must eventually be
/// closed.
///
/// # Errors
///
/// Returns an error if the index is corrupt or if there is a low-level I/O error.
pub fn open_if_changed<DR>(old_reader: &DR) -> Result<Option<DR::DirectoryReader>>
where
  DR: DirectoryReader,
{
  old_reader.do_open_if_changed()
}

/// If the `IndexCommit` differs from what the provided reader is searching, open and return a new
/// reader; otherwise, return `None`.
///
/// # Errors
///
/// Returns an error if there is a low-level I/O error.
pub fn open_if_changed_with_commit<DR, IC>(
  old_reader: &DR,
  commit: Option<&IC>,
) -> Result<Option<DR::DirectoryReader>>
where
  DR: DirectoryReader,
  IC: IndexCommit<Directory = Arc<DR::Directory>>,
{
  old_reader.do_open_if_changed_with_commit(commit)
}

/// Expert: If there are committed or uncommitted changes in the [`IndexWriter`] versus what the
/// provided reader is searching, open and return a new reader searching both committed and
/// uncommitted changes from the writer; otherwise, return `None`.
///
/// This provides near real-time searching: changes made during an [`IndexWriter`] session can be
/// quickly made available for searching without closing the writer or calling commit.
///
/// The first time this method is called, this writer instance will make every effort to pool the
/// readers it opens for merges, applying deletes, and related work. This means additional resources
/// such as RAM, file descriptors, and CPU time may be consumed.
///
/// Once the writer is closed, outstanding readers may continue to be used. However, attempting to
/// reopen any of those readers will hit an already-closed error.
///
/// # Returns
///
/// A reader that covers the entire index plus all changes made so far by this writer, or `None` if
/// there are no new changes.
///
/// # Errors
///
/// Returns an error if there is a low-level I/O error.
///
/// # Lucene
///
/// This API is marked as experimental in Lucene.
pub fn open_if_changed_with_writer<DR>(
  old_reader: &DR,
  writer: &Arc<IndexWriter<DR::Directory>>,
) -> Result<Option<DR::DirectoryReader>>
where
  DR: DirectoryReader,
{
  open_if_changed_with_writer_deletes(old_reader, writer, true)
}

/// Expert: Opens a new reader, if there are any changes, controlling whether past deletions should
/// be applied.
///
/// # Arguments
///
/// * `writer` - The [`IndexWriter`] to open from.
/// * `apply_all_deletes` - If `true`, all buffered deletes will be applied and made visible in the
///   returned reader. If `false`, deletes are not applied but remain buffered in the writer so that
///   they will be applied in the future. Applying deletes can be costly, so applications that can
///   tolerate deleted documents being returned may gain performance by passing `false`.
///
/// # Errors
///
/// Returns an error if there is a low-level I/O error.
///
/// # Lucene
///
/// This API is marked as experimental in Lucene.
pub fn open_if_changed_with_writer_deletes<DR>(
  old_reader: &DR,
  writer: &Arc<IndexWriter<DR::Directory>>,
  apply_all_deletes: bool,
) -> Result<Option<DR::DirectoryReader>>
where
  DR: DirectoryReader,
{
  old_reader.do_open_if_changed_with_deletes(writer, apply_all_deletes)
}

/// Returns all commit points that exist in the [`Directory`]. Normally, because the default is
/// `KeepOnlyLastCommitDeletionPolicy`, there would be only one commit point. But if you're using a
/// custom `IndexDeletionPolicy` then there could be many commits. Once you have a given commit, you
/// can open a reader on it by calling `DirectoryReader::open`. There must be at least one commit in
/// the [`Directory`], else this method returns `IndexNotFound`. Note that if a commit is in progress
/// while this method is running, that commit may or may not be returned.
///
/// # Returns
///
/// A sorted list of `IndexCommit`s, from oldest to latest.
pub fn list_commits<D>(dir: Arc<D>) -> Result<Vec<ReaderCommit<D>>>
where
  D: Directory,
{
  let files = dir.list_all()?;

  let mut commits = Vec::new();

  let latest = SegmentInfos::read_latest_commit(dir.clone())?;
  let current_gen = latest.get_generation();

  commits.push(ReaderCommit::new(&latest, dir.clone())?);

  for file_name in files {
    if file_name.starts_with(IndexFileNames::SEGMENTS)
      && generation_from_segments_file_name(&file_name)? < current_gen
    {
      let sis = match SegmentInfos::read_commit(dir.clone(), &file_name) {
        Ok(sis) => Some(sis),
        Err(LuceneError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
          None
        },
        Err(LuceneError::IoWithPath { source, .. })
          if source.kind() == std::io::ErrorKind::NotFound =>
        {
          None
        },
        Err(err) => return Err(err),
      };

      if let Some(sis) = sis {
        commits.push(ReaderCommit::new(&sis, dir.clone())?);
      }
    }
  }

  commits.sort();

  Ok(commits)
}
/// Returns `true` if an index likely exists at the specified directory. Note that if a
/// corrupt index exists, or if an index in the process of committing
///
/// # Parameters
///
/// - `directory`: the directory to check for an index
///
/// # Returns
///
/// `true` if an index exists; `false` otherwise
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
  // return an error on such indices and the app must
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
pub struct DirectoryReaderBase<D> {
  pub directory: Arc<D>,
}
impl<D> DirectoryReaderBase<D> {
  pub fn new(directory: Arc<D>) -> Self {
    Self { directory }
  }
}

impl<T> BaseCompositeReader for &T where T: DirectoryReader {}

impl<T> DirectoryReader for &T
where
  T: DirectoryReader,
{
  type DirectoryReader = T::DirectoryReader;
  type Directory = T::Directory;

  fn directory(&self) -> &DirectoryReaderBase<Self::Directory> {
    (**self).directory()
  }

  fn do_open_if_changed(&self) -> Result<Option<Self::DirectoryReader>> {
    (**self).do_open_if_changed()
  }

  fn do_open_if_changed_with_commit<IC>(
    &self,
    commit: Option<&IC>,
  ) -> Result<Option<Self::DirectoryReader>>
  where
    IC: IndexCommit<Directory = Arc<Self::Directory>>,
  {
    (**self).do_open_if_changed_with_commit(commit)
  }

  fn do_open_if_changed_with_deletes(
    &self,
    writer: &Arc<IndexWriter<Self::Directory>>,
    apply_deletes: bool,
  ) -> Result<Option<Self::DirectoryReader>> {
    (**self).do_open_if_changed_with_deletes(writer, apply_deletes)
  }

  fn get_version(&self) -> Result<i64> {
    (**self).get_version()
  }

  fn is_current(&self) -> Result<bool> {
    (**self).is_current()
  }

  type IndexCommit = T::IndexCommit;

  fn get_index_commit(&self) -> Result<Self::IndexCommit> {
    (**self).get_index_commit()
  }
}

impl<T> BaseCompositeReader for Arc<T> where T: DirectoryReader {}

impl<T> DirectoryReader for Arc<T>
where
  T: DirectoryReader,
{
  type DirectoryReader = T::DirectoryReader;
  type Directory = T::Directory;

  fn directory(&self) -> &DirectoryReaderBase<Self::Directory> {
    (**self).directory()
  }

  fn do_open_if_changed(&self) -> Result<Option<Self::DirectoryReader>> {
    (**self).do_open_if_changed()
  }

  fn do_open_if_changed_with_commit<IC>(
    &self,
    commit: Option<&IC>,
  ) -> Result<Option<Self::DirectoryReader>>
  where
    IC: IndexCommit<Directory = Arc<Self::Directory>>,
  {
    (**self).do_open_if_changed_with_commit(commit)
  }

  fn do_open_if_changed_with_deletes(
    &self,
    writer: &Arc<IndexWriter<Self::Directory>>,
    apply_deletes: bool,
  ) -> Result<Option<Self::DirectoryReader>> {
    (**self).do_open_if_changed_with_deletes(writer, apply_deletes)
  }

  fn get_version(&self) -> Result<i64> {
    (**self).get_version()
  }

  fn is_current(&self) -> Result<bool> {
    (**self).is_current()
  }

  type IndexCommit = T::IndexCommit;

  fn get_index_commit(&self) -> Result<Self::IndexCommit> {
    (**self).get_index_commit()
  }
}

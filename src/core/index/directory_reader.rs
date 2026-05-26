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
use crate::core::index::dummy::dummy_index_commit::DummyIndexCommit;
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::segment_infos::{SegmentInfos, generation_from_segments_file_name};
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::sync::Arc;

use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::index::standard_directory_reader::{
  ReaderCommit, StandardDirectoryReader, StandardDirectoryReaderType,
};
use crate::core::util::Comparator;
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
  fn do_open_if_changed(
    &self,
    writer: IndexWriter<Self::Directory>,
  ) -> Result<Option<Self::DirectoryReader>>;
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
    writer: IndexWriter<Self::Directory>,
    commit: Option<&IC>,
  ) -> Result<Option<Self::DirectoryReader>>
  where
    IC: IndexCommit<Directory = Self::Directory>;
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
  fn do_open_if_changed_with_writer(
    &self,
    writer: IndexWriter<Self::Directory>,
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
  fn is_current<D>(&self, index_writer: &IndexWriter<D>) -> Result<bool>
  where
    D: Directory;
  type IndexCommit: IndexCommit;
  /// Expert: return the IndexCommit that this reader has opened.
  fn get_index_commit(&self) -> Result<Self::IndexCommit>;
  type Directory: Directory;
  /// The index directory
  fn directory(&self) -> &DirectoryReaderBase<Self::Directory>;
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
pub fn open<D>(directory: Arc<D>) -> Result<StandardDirectoryReaderType<D>>
where
  D: Directory,
{
  StandardDirectoryReader::<_, _>::open::<DummyIndexCommit<D>>(directory, None, None)
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
) -> Result<StandardDirectoryReader<C, D>>
where
  D: Directory,
  C: Comparator<DefaultLeafReader<D>> + Clone,
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
pub fn open_from_writer<D>(writer: &IndexWriter<D>) -> Result<StandardDirectoryReaderType<D>>
where
  D: Directory,
{
  open_with_writer_deletes(writer, true, false)
}
pub fn open_from_writer_with_leaf_sorter<D, C>(
  writer: &IndexWriter<D>,
  leaf_sorter: C,
) -> Result<StandardDirectoryReader<C, D>>
where
  D: Directory,
  C: Comparator<DefaultLeafReader<D>> + Clone,
{
  open_with_writer_deletes_and_leaf_sorter(writer, true, false, leaf_sorter)
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
  writer: &IndexWriter<D>,
  apply_all_deletes: bool,
  write_all_deletes: bool,
) -> Result<StandardDirectoryReaderType<D>>
where
  D: Directory,
{
  writer.get_reader(apply_all_deletes, write_all_deletes)
}
pub fn open_with_writer_deletes_and_leaf_sorter<D, C>(
  writer: &IndexWriter<D>,
  apply_all_deletes: bool,
  write_all_deletes: bool,
  leaf_sorter: C,
) -> Result<StandardDirectoryReader<C, D>>
where
  D: Directory,
  C: Comparator<DefaultLeafReader<D>> + Clone,
{
  writer.get_reader_with_leaf_sorter(apply_all_deletes, write_all_deletes, Some(leaf_sorter))
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
pub fn open_from_commit<D, C, IC>(commit: &IC) -> Result<StandardDirectoryReader<C, D>>
where
  D: Directory,
  C: Comparator<DefaultLeafReader<D>> + Clone,
  IC: IndexCommit<Directory = D>,
{
  StandardDirectoryReader::open(commit.get_directory(), Some(commit), None)
}
/// Returns all commit points that exist in the [`Directory`]. Normally, because the default is
/// [`KeepOnlyLastCommitDeletionPolicy`], there would be only one commit point. But if you're using a
/// custom [`IndexDeletionPolicy`] then there could be many commits. Once you have a given commit, you
/// can open a reader on it by calling [`DirectoryReader::open`]. There must be at least one commit in
/// the [`Directory`], else this method returns [`IndexNotFound`]. Note that if a commit is in progress
/// while this method is running, that commit may or may not be returned.
///
/// # Returns
///
/// A sorted list of [`IndexCommit`]s, from oldest to latest.
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
        Err(LuceneError::Io(err)) if err.kind() == std::io::ErrorKind::NotFound => None,
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
pub fn open_with_version<D, C, IC>(
  commit: &IC,
  min_supported_major_version: i32,
  leaf_sorter: Option<C>,
) -> Result<StandardDirectoryReader<C, D>>
where
  D: Directory,
  C: Comparator<DefaultLeafReader<D>> + Clone,
  IC: IndexCommit<Directory = D>,
{
  StandardDirectoryReader::open_with_version(
    commit.get_directory(),
    min_supported_major_version,
    Some(commit),
    leaf_sorter,
  )
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

impl<T> BaseCompositeReader for Arc<T> where T: DirectoryReader {}

impl<T> DirectoryReader for Arc<T>
where
  T: DirectoryReader,
{
  type DirectoryReader = T::DirectoryReader;

  fn do_open_if_changed(
    &self,
    writer: IndexWriter<Self::Directory>,
  ) -> Result<Option<Self::DirectoryReader>> {
    (**self).do_open_if_changed(writer)
  }

  fn do_open_if_changed_with_commit<IC>(
    &self,
    writer: IndexWriter<Self::Directory>,
    commit: Option<&IC>,
  ) -> Result<Option<Self::DirectoryReader>>
  where
    IC: IndexCommit<Directory = Self::Directory>,
  {
    (**self).do_open_if_changed_with_commit(writer, commit)
  }

  fn do_open_if_changed_with_writer(
    &self,
    writer: IndexWriter<Self::Directory>,
    apply_deletes: bool,
  ) -> Result<Option<Self::DirectoryReader>> {
    (**self).do_open_if_changed_with_writer(writer, apply_deletes)
  }

  fn get_version(&self) -> Result<i64> {
    (**self).get_version()
  }

  fn is_current<D>(&self, index_writer: &IndexWriter<D>) -> Result<bool>
  where
    D: Directory,
  {
    (**self).is_current(index_writer)
  }

  type IndexCommit = T::IndexCommit;

  fn get_index_commit(&self) -> Result<Self::IndexCommit> {
    (**self).get_index_commit()
  }

  type Directory = T::Directory;

  fn directory(&self) -> &DirectoryReaderBase<Self::Directory> {
    (**self).directory()
  }
}
#[cfg(test)]
mod tests {
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::document::field_type::FieldType;
  use crate::core::document::text_field::{TextField, text_field_type};
  use crate::core::index::two_phase_commit::TwoPhaseCommit;

  use crate::core::index::composite_reader::get_context;
  use crate::core::index::fields::Fields;
  use crate::core::index::index_reader::IndexReader;
  use crate::core::index::index_reader_context::IndexReaderContext;
  use crate::core::index::index_writer::IndexWriter;
  use crate::core::index::index_writer_config::OpenMode;
  use crate::core::index::leaf_reader::LeafReader;
  use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
  use crate::core::index::multi_reader::MultiReader;

  use crate::core::document::stored_field::StoredField;
  use crate::core::index::stored_fields::StoredFields;
  use crate::core::index::term_vectors::TermVectors;
  use crate::core::index::terms::Terms;
  use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
  use crate::core::index::{BytesRef, directory_reader, field_infos, multi_terms};
  use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
  use crate::core::store::directory::{DirEnum, Directory};
  use crate::core::util::error::lucene_error::{LuceneError, Result};
  use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
  use crate::test::core::index::doc_helper;
  use crate::test::core::index::doc_helper::{DATA, DocHelper};
  use crate::test::core::index::random_index_writer::RandomIndexWriter;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    at_least, create_temp_dir_with_prefix, get_only_leaf_reader, new_directory_shared, new_field,
    new_fs_directory, new_index_writer_config_with_analyzer, new_log_merge_policy,
    new_string_field, new_text_field, random,
  };
  use crate::test::core::util::test_util::TestUtil;
  use rand::Rng;
  use std::collections::{HashMap, HashSet};
  use std::sync::Arc;
  use std::thread;
  use std::time::Duration;

  use crate::core::index::index_options::IndexOptions;
  use crate::core::index::indexable_field::IndexableField;
  use crate::core::index::log_merge_policy::LogMergePolicy;
  use crate::core::index::merge_policy::MergePolicyEnum;

  #[allow(dead_code)] // for quick search
  struct TestDirectoryReader;

  #[test]
  fn test_document() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let mut doc1 = Document::new();
    let mut doc2 = Document::new();

    DocHelper::setup_doc(&mut doc1);
    DocHelper::setup_doc(&mut doc2);

    DocHelper::write_doc(&mut random, dir.clone(), doc1.clone())?;
    DocHelper::write_doc(&mut random, dir.clone(), doc2.clone())?;

    let reader = directory_reader::open(dir.clone())?;
    let mut stored_fields = reader.stored_fields()?;

    let new_doc1 = stored_fields.document(0)?;
    assert_eq!(
      DocHelper::num_fields(&new_doc1),
      DocHelper::num_fields(&doc1) - DATA.unstored.len()
    );

    let new_doc2 = stored_fields.document(1)?;
    assert_eq!(
      DocHelper::num_fields(&new_doc2),
      DocHelper::num_fields(&doc2) - DATA.unstored.len()
    );

    let mut term_vectors = reader.term_vectors()?;
    let vector = term_vectors
      .get(0)?
      .unwrap()
      .terms(doc_helper::TEXT_FIELD_2_KEY)?;

    assert!(vector.is_some());
    reader.close()?;
    Ok(())
  }
  #[test]
  fn test_multi_term_docs() -> Result<()> {
    let mut random = random();

    let ram_dir1 = new_directory_shared(&mut random)?;
    let mut field_to_type = HashMap::new();
    add_doc_with_open_mode(
      &mut random,
      ram_dir1.clone(),
      "test foo",
      true,
      &mut field_to_type,
    )?;

    let ram_dir2 = new_directory_shared(&mut random)?;
    add_doc_with_open_mode(
      &mut random,
      ram_dir2.clone(),
      "test blah",
      true,
      &mut field_to_type,
    )?;

    let ram_dir3 = new_directory_shared(&mut random)?;
    add_doc_with_open_mode(
      &mut random,
      ram_dir3.clone(),
      "test wow",
      true,
      &mut field_to_type,
    )?;

    let reader1_0 = directory_reader::open(ram_dir1.clone())?;
    let reader1_1 = directory_reader::open(ram_dir3.clone())?;
    let reader2_0 = directory_reader::open(ram_dir1.clone())?;
    let reader2_1 = directory_reader::open(ram_dir2.clone())?;
    let reader2_2 = directory_reader::open(ram_dir3.clone())?;

    let mr2 = MultiReader::with_composite_reader(vec![reader1_0, reader1_1])?;
    let mr3 = MultiReader::with_composite_reader(vec![reader2_0, reader2_1, reader2_2])?;

    // test mixing up TermDocs and TermEnums from different readers.
    let terms2 = multi_terms::get_terms(&mr2, "body")?.unwrap();
    let mut te2 = terms2.iterator()?;
    assert_eq!(
      SeekStatus::Found,
      te2.seek_ceil(&BytesRef::from_string("wow"))?
    );
    let term = te2.term()?.into_owned();
    let mut td = TestUtil::docs_with_reader(&mut random, &mr2, "body", &term, None, 0)?.unwrap();

    let terms3 = multi_terms::get_terms(&mr3, "body")?.unwrap();
    let mut te3 = terms3.iterator()?;
    assert_eq!(
      SeekStatus::Found,
      te3.seek_ceil(&BytesRef::from_string("wow"))?
    );
    td = TestUtil::docs(&mut random, &mut te3, Some(td), 0)?;

    let mut ret = 0;

    // This should blow up if we forget to check that the TermEnum is from the same
    // reader as the TermDocs.
    while td.next_doc()? != NO_MORE_DOCS {
      ret += td.doc_id();
    }

    // really a dummy assert to ensure that we got some docs and to ensure that
    // nothing is eliminated by hotspot
    assert!(ret > 0);
    Ok(())
  }

  fn add_doc_with_open_mode<R: Rng + ?Sized>(
    random: &mut R,
    dir: Arc<DirEnum>,
    s: &str,
    create: bool,
    field_to_type: &mut HashMap<String, FieldType>,
  ) -> Result<()> {
    let mock = MockAnalyzer::new(random);
    let mut iwc = new_index_writer_config_with_analyzer(random, mock);
    iwc.set_open_mode(if create {
      OpenMode::Create
    } else {
      OpenMode::Append
    });

    let iw = IndexWriter::new(dir, iwc)?;
    let mut doc = Document::new();
    doc.add(new_text_field(random, "body", s, Store::No, field_to_type)?);
    iw.add_document(doc)?;
    iw.close()?;
    Ok(())
  }
  // TODO IMPORTANT StandardDirectoryReader#is_current()
  #[test]
  fn test_is_current() -> Result<()> {
    // let mut random = random();
    // let d = new_directory_shared(&mut random)?;
    // let mut field_to_type = HashMap::new();
    //
    // let mock = MockAnalyzer::new(&mut random);
    // let writer = IndexWriter::new(
    //   d.clone(),
    //   new_index_writer_config_with_analyzer(&mut random, mock),
    // )?;
    // add_document_with_fields(&mut random, &writer, &mut field_to_type)?;
    // writer.close()?;
    //
    // // set up reader:
    // let reader = directory_reader::open(d.clone())?;
    // assert!(reader.is_current(&writer)?);
    //
    // // modify index by adding another document:
    // let mock = MockAnalyzer::new(&mut random);
    // let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
    // config.set_open_mode(OpenMode::Append);
    // let writer = IndexWriter::new(d.clone(), config)?;
    // add_document_with_fields(&mut random, &writer, &mut field_to_type)?;
    // writer.close()?;
    //
    // assert!(!reader.is_current(&writer)?);
    //
    // // re-create index:
    // let mock = MockAnalyzer::new(&mut random);
    // let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
    // config.set_open_mode(OpenMode::Create);
    // let writer = IndexWriter::new(d.clone(), config)?;
    // add_document_with_fields(&mut random, &writer, &mut field_to_type)?;
    // writer.close()?;
    //
    // assert!(!reader.is_current(&writer)?);
    //
    // reader.close()?;
    Ok(())
  }
  fn test_get_field_names() -> Result<()> {
    let mut random = random();
    let d = new_directory_shared(&mut random)?;
    let mut field_to_type = HashMap::new();

    // set up writer
    let mock = MockAnalyzer::new(&mut random);
    let writer = IndexWriter::new(
      d.clone(),
      new_index_writer_config_with_analyzer(&mut random, mock),
    )?;

    let mut custom_type3 = FieldType::new();
    custom_type3.set_stored(true)?;

    let mut doc = Document::new();
    doc.add(new_string_field(
      &mut random,
      "keyword",
      "test1",
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(new_text_field(
      &mut random,
      "text",
      "test1",
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(new_field(
      &mut random,
      "unindexed",
      "test1",
      &custom_type3,
      &mut field_to_type,
    )?);
    doc.add(TextField::from_string("unstored", "test1", Store::No)?);
    writer.add_document(doc)?;

    writer.close()?;
    drop(writer);

    // set up reader
    let reader = directory_reader::open(d.clone())?;
    let field_infos = field_infos::get_merged_field_infos(&reader)?;
    assert!(field_infos.field_info_by_name("keyword").is_some());
    assert!(field_infos.field_info_by_name("text").is_some());
    assert!(field_infos.field_info_by_name("unindexed").is_some());
    assert!(field_infos.field_info_by_name("unstored").is_some());
    reader.close()?;

    // add more documents
    let mock = MockAnalyzer::new(&mut random);
    let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
    config.set_open_mode(OpenMode::Append);

    let merge_policy = LogMergePolicy::log_bytes_size();
    let merge_factor = merge_policy.get_merge_factor();
    config.set_merge_policy(merge_policy);

    let writer = IndexWriter::new(d.clone(), config)?;

    // want to get some more segments here
    for _ in 0..5 * merge_factor {
      let mut doc = Document::new();
      doc.add(new_string_field(
        &mut random,
        "keyword",
        "test1",
        Store::Yes,
        &mut field_to_type,
      )?);
      doc.add(new_text_field(
        &mut random,
        "text",
        "test1",
        Store::Yes,
        &mut field_to_type,
      )?);
      doc.add(new_field(
        &mut random,
        "unindexed",
        "test1",
        &custom_type3,
        &mut field_to_type,
      )?);
      doc.add(TextField::from_string("unstored", "test1", Store::No)?);
      writer.add_document(doc)?;
    }

    // new fields are in some different segments (we hope)
    for _ in 0..5 * merge_factor {
      let mut doc = Document::new();
      doc.add(new_string_field(
        &mut random,
        "keyword2",
        "test1",
        Store::Yes,
        &mut field_to_type,
      )?);
      doc.add(new_text_field(
        &mut random,
        "text2",
        "test1",
        Store::Yes,
        &mut field_to_type,
      )?);
      doc.add(new_field(
        &mut random,
        "unindexed2",
        "test1",
        &custom_type3,
        &mut field_to_type,
      )?);
      doc.add(TextField::from_string("unstored2", "test1", Store::No)?);
      writer.add_document(doc)?;
    }

    // new termvector fields
    let mut custom_type5 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
    custom_type5.set_store_term_vectors(true)?;

    let mut custom_type6 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
    custom_type6.set_store_term_vectors(true)?;
    custom_type6.set_store_term_vector_offsets(true)?;

    let mut custom_type7 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
    custom_type7.set_store_term_vectors(true)?;
    custom_type7.set_store_term_vector_positions(true)?;

    let mut custom_type8 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
    custom_type8.set_store_term_vectors(true)?;
    custom_type8.set_store_term_vector_offsets(true)?;
    custom_type8.set_store_term_vector_positions(true)?;

    for _ in 0..5 * merge_factor {
      let mut doc = Document::new();
      doc.add(TextField::from_string("tvnot", "tvnot", Store::Yes)?);
      doc.add(new_field(
        &mut random,
        "termvector",
        "termvector",
        &custom_type5,
        &mut field_to_type,
      )?);
      doc.add(new_field(
        &mut random,
        "tvoffset",
        "tvoffset",
        &custom_type6,
        &mut field_to_type,
      )?);
      doc.add(new_field(
        &mut random,
        "tvposition",
        "tvposition",
        &custom_type7,
        &mut field_to_type,
      )?);
      doc.add(new_field(
        &mut random,
        "tvpositionoffset",
        "tvpositionoffset",
        &custom_type8,
        &mut field_to_type,
      )?);

      writer.add_document(doc)?;
    }

    writer.close()?;
    drop(writer);

    // verify fields again
    let reader = directory_reader::open(d.clone())?;
    let field_infos = field_infos::get_merged_field_infos(&reader)?;

    let mut all_field_names = HashSet::new();
    let mut indexed_field_names = HashSet::new();
    let mut not_indexed_field_names = HashSet::new();
    let mut tv_field_names = HashSet::new();

    for field_info in field_infos.iter() {
      let name = field_info.name.clone();
      all_field_names.insert(name.clone());

      if field_info.get_index_options() != &IndexOptions::None {
        indexed_field_names.insert(name.clone());
      } else {
        not_indexed_field_names.insert(name.clone());
      }

      if field_info.has_term_vectors() {
        tv_field_names.insert(name);
      }
    }

    assert!(all_field_names.contains("keyword"));
    assert!(all_field_names.contains("text"));
    assert!(all_field_names.contains("unindexed"));
    assert!(all_field_names.contains("unstored"));
    assert!(all_field_names.contains("keyword2"));
    assert!(all_field_names.contains("text2"));
    assert!(all_field_names.contains("unindexed2"));
    assert!(all_field_names.contains("unstored2"));
    assert!(all_field_names.contains("tvnot"));
    assert!(all_field_names.contains("termvector"));
    assert!(all_field_names.contains("tvposition"));
    assert!(all_field_names.contains("tvoffset"));
    assert!(all_field_names.contains("tvpositionoffset"));

    // verify that only indexed fields were returned
    assert_eq!(11, indexed_field_names.len()); // 6 original + the 5 termvector fields
    assert!(indexed_field_names.contains("keyword"));
    assert!(indexed_field_names.contains("text"));
    assert!(indexed_field_names.contains("unstored"));
    assert!(indexed_field_names.contains("keyword2"));
    assert!(indexed_field_names.contains("text2"));
    assert!(indexed_field_names.contains("unstored2"));
    assert!(indexed_field_names.contains("tvnot"));
    assert!(indexed_field_names.contains("termvector"));
    assert!(indexed_field_names.contains("tvposition"));
    assert!(indexed_field_names.contains("tvoffset"));
    assert!(indexed_field_names.contains("tvpositionoffset"));

    // verify that only unindexed fields were returned
    assert_eq!(2, not_indexed_field_names.len()); // the following fields
    assert!(not_indexed_field_names.contains("unindexed"));
    assert!(not_indexed_field_names.contains("unindexed2"));

    // verify index term vector fields
    assert_eq!(4, tv_field_names.len(), "{:?}", tv_field_names); // 4 field has term vector only
    assert!(tv_field_names.contains("termvector"));

    reader.close()?;
    Ok(())
  }
  #[test]
  fn test_term_vectors() -> Result<()> {
    let mut random = random();
    let d = new_directory_shared(&mut random)?;
    let mut field_to_type = HashMap::new();

    // set up writer
    let mock = MockAnalyzer::new(&mut random);
    let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
    let merge_policy = new_log_merge_policy(&mut random)?;
    let merge_factor = match &merge_policy {
      MergePolicyEnum::LogBytesSize(v) => v.get_merge_factor(),
      MergePolicyEnum::LogDoc(v) => v.get_merge_factor(),
      _ => unreachable!(),
    };
    config.set_merge_policy(merge_policy);

    let writer = IndexWriter::new(d.clone(), config)?;

    // want to get some more segments here
    // new termvector fields
    let mut custom_type5 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
    custom_type5.set_store_term_vectors(true)?;

    let mut custom_type6 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
    custom_type6.set_store_term_vectors(true)?;
    custom_type6.set_store_term_vector_offsets(true)?;

    let mut custom_type7 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
    custom_type7.set_store_term_vectors(true)?;
    custom_type7.set_store_term_vector_positions(true)?;

    let mut custom_type8 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
    custom_type8.set_store_term_vectors(true)?;
    custom_type8.set_store_term_vector_offsets(true)?;
    custom_type8.set_store_term_vector_positions(true)?;

    for _ in 0..5 * merge_factor {
      let mut doc = Document::new();
      doc.add(TextField::from_string(
        "tvnot",
        "one two two three three three",
        Store::Yes,
      )?);
      doc.add(new_field(
        &mut random,
        "termvector",
        "one two two three three three",
        &custom_type5,
        &mut field_to_type,
      )?);
      doc.add(new_field(
        &mut random,
        "tvoffset",
        "one two two three three three",
        &custom_type6,
        &mut field_to_type,
      )?);
      doc.add(new_field(
        &mut random,
        "tvposition",
        "one two two three three three",
        &custom_type7,
        &mut field_to_type,
      )?);
      doc.add(new_field(
        &mut random,
        "tvpositionoffset",
        "one two two three three three",
        &custom_type8,
        &mut field_to_type,
      )?);

      writer.add_document(doc)?;
    }

    writer.close()?;
    Ok(())
  }
  #[test]
  fn test_binary_fields() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mut field_to_type = HashMap::new();

    let bin = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];

    let mock = MockAnalyzer::new(&mut random);
    let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
    config.set_merge_policy(new_log_merge_policy(&mut random)?);

    let writer = IndexWriter::new(dir.clone(), config)?;

    for i in 0..10 {
      add_doc(
        &mut random,
        &writer,
        &format!("document number {}", i + 1),
        &mut field_to_type,
      )?;
      add_document_with_fields(&mut random, &writer, &mut field_to_type)?;
      add_document_with_different_fields(&mut random, &writer, &mut field_to_type)?;
      add_document_with_term_vector_fields(&mut random, &writer, &mut field_to_type)?;
    }
    writer.close()?;
    drop(writer);

    let mock = MockAnalyzer::new(&mut random);
    let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
    config.set_open_mode(OpenMode::Append);
    config.set_merge_policy(new_log_merge_policy(&mut random)?);

    let writer = IndexWriter::new(dir.clone(), config)?;

    let mut doc = Document::new();
    doc.add(StoredField::from_binary("bin1", bin.clone())?);
    doc.add(TextField::from_string("junk", "junk text", Store::No)?);
    writer.add_document(doc)?;
    writer.close()?;
    drop(writer);

    let reader = directory_reader::open(dir.clone())?;
    let mut stored_fields = reader.stored_fields()?;
    let doc2 = stored_fields.document(reader.max_doc()? - 1)?;
    let fields = doc2.get_fields_with_name("bin1");

    assert_eq!(1, fields.len());
    let b1 = &fields[0];
    let bytes_ref = b1.binary_value()?.unwrap();
    assert_eq!(bin.len(), bytes_ref.length);
    for (i, expected) in bin.iter().enumerate() {
      assert_eq!(*expected, bytes_ref.bytes[bytes_ref.offset + i]);
    }
    reader.close()?;

    // force merge
    let mock = MockAnalyzer::new(&mut random);
    let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
    config.set_open_mode(OpenMode::Append);
    config.set_merge_policy(new_log_merge_policy(&mut random)?);

    let writer = IndexWriter::new(dir.clone(), config)?;
    writer.force_merge(1)?;
    writer.close()?;
    drop(writer);

    let reader = directory_reader::open(dir.clone())?;
    let mut stored_fields = reader.stored_fields()?;
    let doc2 = stored_fields.document(reader.max_doc()? - 1)?;
    let fields = doc2.get_fields_with_name("bin1");

    assert_eq!(1, fields.len());
    let b1 = &fields[0];
    let bytes_ref = b1.binary_value()?.unwrap();
    assert_eq!(bin.len(), bytes_ref.length);
    for (i, expected) in bin.iter().enumerate() {
      assert_eq!(*expected, bytes_ref.bytes[bytes_ref.offset + i]);
    }

    reader.close()?;
    Ok(())
  }
  #[test]
  fn test_files_open_close() -> Result<()> {
    // TODO IMPORTANT 需要优化new_fs_directory的参数
    Ok(())
  }
  #[test]
  fn test_open_reader_after_delete() -> Result<()> {
    // TODO IMPORTANT test_files_open_close未实现
    Ok(())
  }
  fn add_document_with_fields<D, R>(
    random: &mut R,
    writer: &IndexWriter<D>,
    field_to_type: &mut HashMap<String, FieldType>,
  ) -> Result<()>
  where
    D: Directory,
    R: Rng + ?Sized,
  {
    let mut doc = Document::new();

    let mut custom_type3 = FieldType::new();
    custom_type3.set_stored(true)?;

    doc.add(new_string_field(
      random,
      "keyword",
      "test1",
      Store::Yes,
      field_to_type,
    )?);
    doc.add(new_text_field(
      random,
      "text",
      "test1",
      Store::Yes,
      field_to_type,
    )?);
    doc.add(new_field(
      random,
      "unindexed",
      "test1",
      &custom_type3,
      field_to_type,
    )?);
    doc.add(TextField::from_string("unstored", "test1", Store::No)?);

    writer.add_document(doc)?;
    Ok(())
  }

  fn add_document_with_different_fields<D, R>(
    random: &mut R,
    writer: &IndexWriter<D>,
    field_to_type: &mut HashMap<String, FieldType>,
  ) -> Result<()>
  where
    D: Directory,
    R: Rng + ?Sized,
  {
    let mut doc = Document::new();

    let mut custom_type3 = FieldType::new();
    custom_type3.set_stored(true)?;

    doc.add(new_string_field(
      random,
      "keyword2",
      "test1",
      Store::Yes,
      field_to_type,
    )?);
    doc.add(new_text_field(
      random,
      "text2",
      "test1",
      Store::Yes,
      field_to_type,
    )?);
    doc.add(new_field(
      random,
      "unindexed2",
      "test1",
      &custom_type3,
      field_to_type,
    )?);
    doc.add(TextField::from_string("unstored2", "test1", Store::No)?);

    writer.add_document(doc)?;
    Ok(())
  }

  fn add_document_with_term_vector_fields<D, R>(
    random: &mut R,
    writer: &IndexWriter<D>,
    field_to_type: &mut HashMap<String, FieldType>,
  ) -> Result<()>
  where
    D: Directory,
    R: Rng + ?Sized,
  {
    let mut doc = Document::new();

    let mut custom_type5 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
    custom_type5.set_store_term_vectors(true)?;

    let mut custom_type6 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
    custom_type6.set_store_term_vectors(true)?;
    custom_type6.set_store_term_vector_offsets(true)?;

    let mut custom_type7 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
    custom_type7.set_store_term_vectors(true)?;
    custom_type7.set_store_term_vector_positions(true)?;

    let mut custom_type8 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
    custom_type8.set_store_term_vectors(true)?;
    custom_type8.set_store_term_vector_offsets(true)?;
    custom_type8.set_store_term_vector_positions(true)?;

    doc.add(new_text_field(
      random,
      "tvnot",
      "tvnot",
      Store::Yes,
      field_to_type,
    )?);
    doc.add(new_field(
      random,
      "termvector",
      "termvector",
      &custom_type5,
      field_to_type,
    )?);
    doc.add(new_field(
      random,
      "tvoffset",
      "tvoffset",
      &custom_type6,
      field_to_type,
    )?);
    doc.add(new_field(
      random,
      "tvposition",
      "tvposition",
      &custom_type7,
      field_to_type,
    )?);
    doc.add(new_field(
      random,
      "tvpositionoffset",
      "tvpositionoffset",
      &custom_type8,
      field_to_type,
    )?);

    writer.add_document(doc)?;
    Ok(())
  }

  fn add_doc<D, R>(
    random: &mut R,
    writer: &IndexWriter<D>,
    value: &str,
    field_to_type: &mut HashMap<String, FieldType>,
  ) -> Result<()>
  where
    D: Directory,
    R: Rng + ?Sized,
  {
    let mut doc = Document::new();

    doc.add(new_text_field(
      random,
      "content",
      value,
      Store::No,
      field_to_type,
    )?);

    writer.add_document(doc)?;
    Ok(())
  }
  #[test]
  fn test_get_index_commit() -> Result<()> {
    // TODO
    Ok(())
  }

  #[test]
  fn test_no_dir() -> Result<()> {
    let mut random = random();
    let temp_dir = create_temp_dir_with_prefix("doesnotexist")?;
    let dir = new_fs_directory(&mut random, temp_dir)?;
    match directory_reader::open(dir) {
      Ok(_) => panic!("expected IndexNotFound"),
      Err(err) => assert!(matches!(err, LuceneError::IndexNotFound(_))),
    }
    Ok(())
  }

  #[test]
  fn test_no_dup_commit_file_names() -> Result<()> {
    // TODO
    Ok(())
  }

  #[test]
  fn test_unique_term_count() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(TextField::from_string(
      "field",
      "a b c d e f g h i j k l m n o p q r s t u v w x y z",
      Store::No,
    )?);
    doc.add(TextField::from_string(
      "number",
      "0 1 2 3 4 5 6 7 8 9",
      Store::No,
    )?);
    writer.add_document(doc.clone())?;
    writer.add_document(doc.clone())?;
    writer.commit()?;

    let r = directory_reader::open(dir.clone())?;
    let r1 = get_only_leaf_reader(&r)?;
    assert_eq!(26, r1.terms("field")?.unwrap().size()?);
    assert_eq!(10, r1.terms("number")?.unwrap().size()?);
    writer.add_document(doc)?;
    writer.commit()?;
    // TODO IMPORTANT: openIfChanged 未实现
    let r2 = directory_reader::open_from_writer(&writer)?;
    r.close()?;

    let context = get_context(&r2)?;
    for leaf_context in context.leaves()? {
      assert_eq!(26, leaf_context.reader().terms("field")?.unwrap().size()?);
      assert_eq!(10, leaf_context.reader().terms("number")?.unwrap().size()?);
    }
    r2.close()?;
    writer.close()?;
    Ok(())
  }

  #[test]
  fn test_prepare_commit_is_current() -> Result<()> {
    // TODO
    Ok(())
  }

  #[test]
  fn test_list_commits() -> Result<()> {
    // TODO
    Ok(())
  }

  #[test]
  fn test_total_term_freq_cached() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir.clone(), iwc)?;
    let mut d = Document::new();
    d.add(TextField::from_string("f", "a a b", Store::No)?);
    writer.add_document(d)?;
    let r = directory_reader::open_from_writer(&writer)?;
    writer.close()?;

    if r.total_term_freq(&crate::core::index::term::Term::new(
      "f",
      BytesRef::from_string("b"),
    ))?
      != -1
    {
      assert_eq!(
        1,
        r.total_term_freq(&crate::core::index::term::Term::new(
          "f",
          BytesRef::from_string("b")
        ))?
      );
      assert_eq!(
        2,
        r.total_term_freq(&crate::core::index::term::Term::new(
          "f",
          BytesRef::from_string("a")
        ))?
      );
      assert_eq!(
        1,
        r.total_term_freq(&crate::core::index::term::Term::new(
          "f",
          BytesRef::from_string("b")
        ))?
      );
    }
    r.close()?;
    Ok(())
  }

  #[test]
  fn test_get_sum_doc_freq() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir.clone(), iwc)?;
    let mut d = Document::new();
    d.add(TextField::from_string("f", "a", Store::No)?);
    writer.add_document(d)?;
    let mut d = Document::new();
    d.add(TextField::from_string("f", "b", Store::No)?);
    writer.add_document(d)?;
    let r = directory_reader::open_from_writer(&writer)?;
    writer.close()?;

    if r.get_sum_doc_freq("f")? != -1 {
      assert_eq!(2, r.get_sum_doc_freq("f")?);
    }
    r.close()?;
    Ok(())
  }

  #[test]
  fn test_get_doc_count() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir.clone(), iwc)?;
    let mut d = Document::new();
    d.add(TextField::from_string("f", "a", Store::No)?);
    writer.add_document(d)?;
    let mut d = Document::new();
    d.add(TextField::from_string("f", "a", Store::No)?);
    writer.add_document(d)?;
    let r = directory_reader::open_from_writer(&writer)?;
    writer.close()?;

    if r.get_doc_count("f")? != -1 {
      assert_eq!(2, r.get_doc_count("f")?);
    }
    r.close()?;
    Ok(())
  }

  #[test]
  fn test_get_sum_total_term_freq() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir.clone(), iwc)?;
    let mut d = Document::new();
    d.add(TextField::from_string("f", "a b b", Store::No)?);
    writer.add_document(d)?;
    let mut d = Document::new();
    d.add(TextField::from_string("f", "a a b", Store::No)?);
    writer.add_document(d)?;
    let r = directory_reader::open_from_writer(&writer)?;
    writer.close()?;

    if r.get_sum_total_term_freq("f")? != -1 {
      assert_eq!(6, r.get_sum_total_term_freq("f")?);
    }
    r.close()?;
    Ok(())
  }

  #[test]
  fn test_reader_finished_listener() -> Result<()> {
    // TODO
    Ok(())
  }

  #[test]
  fn test_oob_doc_id() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir.clone(), iwc)?;
    writer.add_document(Document::new())?;
    let r = directory_reader::open_from_writer(&writer)?;
    writer.close()?;
    let mut stored_fields = r.stored_fields()?;
    stored_fields.document(0)?;
    assert!(stored_fields.document(1).is_err());
    r.close()?;
    Ok(())
  }

  #[test]
  fn test_try_inc_ref() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir.clone(), iwc)?;
    writer.add_document(Document::new())?;
    writer.commit()?;
    let r = directory_reader::open(dir.clone())?;
    assert!(r.try_inc_ref());
    r.dec_ref()?;
    r.close()?;
    assert!(!r.try_inc_ref());
    writer.close()?;
    Ok(())
  }

  #[test]
  fn test_stress_try_inc_ref() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir.clone(), iwc)?;
    writer.add_document(Document::new())?;
    writer.commit()?;
    let r = directory_reader::open(dir.clone())?;
    let num_threads = at_least(&mut random, 5);

    thread::scope(|s| -> Result<()> {
      let mut threads = Vec::new();
      for _ in 0..num_threads {
        threads.push(s.spawn(|| -> Result<()> {
          while r.try_inc_ref() {
            assert!(!r.has_deletions()?);
            r.dec_ref()?;
          }
          assert!(!r.try_inc_ref());
          Ok(())
        }));
      }

      thread::sleep(Duration::from_millis(100));

      assert!(r.try_inc_ref());
      r.dec_ref()?;
      r.close()?;

      for thread in threads {
        thread.join().expect("thread should not panic")?;
      }
      Ok(())
    })?;

    assert!(!r.try_inc_ref());
    writer.close()?;
    Ok(())
  }

  #[test]
  fn test_load_certain_fields() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let writer = RandomIndexWriter::new(&mut random, dir.clone());
    let mut field_to_type = HashMap::new();
    let mut doc = Document::new();
    doc.add(new_string_field(
      &mut random,
      "field1",
      "foobar",
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(new_string_field(
      &mut random,
      "field2",
      "foobaz",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;
    let r = writer.get_reader()?;
    writer.close()?;
    let mut fields_to_load = HashSet::new();
    let mut stored_fields = r.stored_fields()?;
    assert_eq!(
      0,
      stored_fields
        .document_with_fields(0, &fields_to_load)?
        .get_fields()
        .len()
    );
    fields_to_load.insert("field1".to_string());
    let doc2 = stored_fields.document_with_fields(0, &fields_to_load)?;
    assert_eq!(1, doc2.get_fields().len());
    assert_eq!("foobar", doc2.get("field1")?.unwrap().as_ref());
    r.close()?;
    Ok(())
  }

  #[test]
  fn test_index_exists_on_non_existent_directory() -> Result<()> {
    let mut random = random();
    let temp_dir = create_temp_dir_with_prefix("testIndexExistsOnNonExistentDirectory")?;
    let dir = new_fs_directory(&mut random, temp_dir)?;
    assert!(!directory_reader::index_exists(dir.as_ref())?);
    Ok(())
  }

  #[test]
  fn test_open_with_invalid_min_compat_version() -> Result<()> {
    // TODO
    Ok(())
  }
}

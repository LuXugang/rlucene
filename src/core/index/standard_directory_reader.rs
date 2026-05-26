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
use crate::core::codecs::Codec;
use crate::core::codecs::LATEST_CODEC;
use crate::core::codecs::live_docs_format::LiveDocsFormat;
use crate::core::index::base_composite_reader::{
  BCRStoredFieldsImpl, BCRTermVectorsImpl, BaseCompositeReader, BaseCompositeReaderBase,
};
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::directory_reader::{DirectoryReader, DirectoryReaderBase};
use crate::core::index::dummy::dummy_composite_reader::DummyCompositeReader;
use crate::core::index::dummy::dummy_index_commit::DummyIndexCommit;
use crate::core::index::index_commit::{IndexCommit, cmp_commit, is_same_commit};
use crate::core::index::index_reader::{
  CacheHelper, CacheKey, IndexReader, IndexReaderBase, IndexReaderEnum,
};
use crate::core::index::index_writer::{IndexWriter, Inner};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicy;
use crate::core::index::pending_deletes::DocBits;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::{FindSegmentsFile, SegmentInfos};
use crate::core::index::segment_reader::{DefaultLeafReader, SegmentReader};
use crate::core::index::term::Term;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_function::IOFunction;
use crate::core::util::{Comparator, LATEST, MIN_SUPPORTED_MAJOR};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;

/// Default implementation of DirectoryReader.
pub struct StandardDirectoryReader<C, D>
where
  C: Comparator<DefaultLeafReader<D>> + Clone,
  D: Directory,
{
  base_composite_reader_base:
    BaseCompositeReaderBase<DefaultLeafReader<D>, DummyCompositeReader<DefaultLeafReader<D>>>,
  directory_reader_base: DirectoryReaderBase<D>,
  apply_all_deletes: bool,
  write_all_deletes: bool,
  pub(crate) segment_infos: SegmentInfos<D>,
  sub_reader_sorter: Option<C>,
  index_base: IndexReaderBase,
  writer_closed: Option<Arc<AtomicBool>>,
  cache_helper: CacheHelperImpl,
}
impl<C, D> StandardDirectoryReader<C, D>
where
  C: Comparator<DefaultLeafReader<D>> + Clone,
  D: Directory,
{
  pub(crate) fn new(
    directory: Arc<D>,
    readers: Vec<DefaultLeafReader<D>>,
    segment_infos: SegmentInfos<D>,
    leaf_sorter: Option<C>,
    apply_all_deletes: bool,
    write_all_deletes: bool,
    closed: Option<Arc<AtomicBool>>,
  ) -> Result<Self> {
    let base_composite_reader_base =
      BaseCompositeReaderBase::with_leaf_readers(readers, leaf_sorter.as_ref())?;
    let directory_reader_base = DirectoryReaderBase::new(directory);
    Ok(StandardDirectoryReader {
      base_composite_reader_base,
      directory_reader_base,
      apply_all_deletes,
      write_all_deletes,
      segment_infos,
      sub_reader_sorter: leaf_sorter,
      index_base: IndexReaderBase::new(),
      writer_closed: closed,
      cache_helper: CacheHelperImpl::new(),
    })
  }

  pub(crate) fn open<IC>(
    directory: Arc<D>,
    commit: Option<&IC>,
    leaf_sorter: Option<C>,
  ) -> Result<StandardDirectoryReader<C, D>>
  where
    D: Directory,
    C: Comparator<DefaultLeafReader<D>> + Clone,
    IC: IndexCommit<Directory = D>,
  {
    Self::open_with_version(directory, *MIN_SUPPORTED_MAJOR, commit, leaf_sorter)
  }
  /// called from DirectoryReader.open(...) methods
  pub(crate) fn open_with_version<IC>(
    directory: Arc<D>,
    min_supported_major_version: i32,
    commit: Option<&IC>,
    leaf_sorter: Option<C>,
  ) -> Result<StandardDirectoryReader<C, D>>
  where
    D: Directory,
    C: Comparator<DefaultLeafReader<D>> + Clone,
    IC: IndexCommit<Directory = D>,
  {
    let mut finder =
      FindSegmentsFileImpl1::new(min_supported_major_version, directory.clone(), leaf_sorter);
    match commit {
      Some(c) => finder.run_with_commit(c),
      None => finder.run(),
    }
  }

  fn do_open_from_commit<IC>(&self, commit: Option<&IC>) -> Result<Self>
  where
    IC: IndexCommit<Directory = D>,
  {
    let mut leaf_reads = Vec::new();
    for v in self.get_sequential_sub_readers() {
      match v {
        IndexReaderEnum::Leaf(lr) => {
          leaf_reads.push(lr.clone());
        },
        _ => return Err(LuceneError::illegal_state("should leaf reader")),
      }
    }
    let mut finder = FindSegmentsFileImpl2::new(
      self.directory().directory.clone(),
      leaf_reads,
      self.sub_reader_sorter.clone(),
    );

    match commit {
      Some(commit) => finder.run_with_commit(commit),
      None => finder.run(),
    }
  }

  fn do_open_from_writer<IC>(
    &self,
    writer: &IndexWriter<D>,
    commit: Option<&IC>,
  ) -> Result<Option<Self>>
  where
    IC: IndexCommit<Directory = D>,
  {
    if let Some(commit) = commit {
      return Ok(Some(self.do_open_from_commit(Some(commit))?));
    }

    if writer.nrt_is_current(self.segment_infos.get_version())? {
      return Ok(None);
    }

    let reader = writer.get_reader_with_leaf_sorter(
      self.apply_all_deletes,
      self.write_all_deletes,
      self.sub_reader_sorter.clone(),
    )?;

    // If in fact no changes took place, return None:
    if reader.get_version()? == self.segment_infos.get_version() {
      reader.dec_ref()?;
      return Ok(None);
    }

    Ok(Some(reader))
  }

  fn do_open_no_writer<IC>(
    &self,
    writer: &IndexWriter<D>,
    commit: Option<&IC>,
  ) -> Result<Option<Self>>
  where
    IC: IndexCommit<Directory = D>,
  {
    if let Some(commit) = commit {
      if !Arc::ptr_eq(&self.directory().directory, &commit.get_directory()) {
        return Err(
          std::io::Error::other("the specified commit does not match the specified Directory")
            .into(),
        );
      }
      if let Some(segments_file_name) = self.segment_infos.get_segments_file_name()
        && commit.get_segments_file_name() == segments_file_name
      {
        return Ok(None);
      }
    } else if self.is_current(writer)? {
      return Ok(None);
    }

    Ok(Some(self.do_open_from_commit(commit)?))
  }
}
pub type StandardDirectoryReaderType<D> = StandardDirectoryReader<EmptyLeafSorter, D>;
pub(crate) fn open_with_reader_function<D, IO, C>(
  writer: &IndexWriter<D>,
  reader_function: &mut IO,
  infos: Option<&SegmentInfos<D>>,
  inner: &mut Inner<D>, // hold IndexWriter lock
  apply_all_deletes: bool,
  write_all_deletes: bool,
  leaf_sorter: Option<C>,
) -> Result<StandardDirectoryReader<C, D>>
where
  D: Directory,
  IO: IOFunction<SegmentCommitInfo<D>, DefaultLeafReader<D>>,
  C: Comparator<DefaultLeafReader<D>> + Clone,
{
  let (segment_infos, dir, readers) = {
    let infos = match infos {
      Some(infos) => infos,
      None => &inner.segment_infos,
    };
    // IndexWriter synchronizes externally before calling
    // us, which ensures infos will not change; so there's
    // no need to process segments in reverse order
    let num_segments = infos.size();
    let mut readers = Vec::with_capacity(num_segments);
    let dir = writer.get_directory();
    let result = (|| {
      let mut segment_infos = infos.try_clone()?;
      let mut infos_upto = 0;
      for i in 0..num_segments {
        // NOTE: important that we use infos not
        // segmentInfos here, so that we are passing the
        // actual instance of SegmentInfoPerCommit in
        // IndexWriter's segmentInfos:
        let info = match infos.info(i) {
          Some(info) => info,
          None => {
            return Err(LuceneError::illegal_argument(
              "SegmentInfoPerCommit at index {} is None".to_string(),
            ));
          },
        };
        debug_assert!(Arc::ptr_eq(&info.info.dir, &dir));
        let reader = reader_function.apply(info)?;
        if reader.num_docs()? > 0
          || writer
            .get_config()
            .get_merge_policy()
            .keep_fully_deleted_segment(|| Ok(reader.clone()))?
        {
          // Steal the ref
          readers.push(reader);
          infos_upto += 1;
        } else {
          reader.dec_ref()?;
          segment_infos.remove(infos_upto);
        }
      }
      Ok(segment_infos)
    })();
    match result {
      Ok(segment_infos) => (segment_infos, dir, readers),
      Err(e) => {
        for r in readers {
          let _ = r.dec_ref();
        }
        return Err(e);
      },
    }
  };
  // Clone pointer should be cheap
  let readers_backup = readers.clone();
  let result: Result<_> = (|| {
    writer.inc_ref_deleter(&segment_infos, Some(inner))?;
    StandardDirectoryReader::new(
      dir,
      readers,
      segment_infos,
      leaf_sorter,
      apply_all_deletes,
      write_all_deletes,
      Some(writer.closed.clone()),
    )
  })();
  match result {
    Ok(r) => Ok(r),
    Err(e) => {
      for r in readers_backup {
        let _ = r.dec_ref();
      }
      Err(e)
    },
  }
}
pub(crate) fn open_with_leaf_sorter<D, C>(
  directory: Arc<D>,
  infos: SegmentInfos<D>,
  old_readers: Vec<DefaultLeafReader<D>>,
  leaf_sorter: Option<C>,
) -> Result<StandardDirectoryReader<C, D>>
where
  D: Directory,
  C: Comparator<DefaultLeafReader<D>> + Clone,
{
  // we put the old SegmentReaders in a map, that allows us
  // to lookup a reader using its segment name
  let mut segment_readers = HashMap::with_capacity(old_readers.len());
  for (i, sr) in old_readers.iter().enumerate() {
    segment_readers.insert(sr.get_segment_name().to_string(), i);
  }

  let mut new_readers: Vec<Option<DefaultLeafReader<D>>> =
    (0..infos.size()).map(|_| None).collect();
  let result: Result<()> = (|| {
    for i in (0..infos.size()).rev() {
      let commit_info = infos
        .info(i)
        .ok_or_else(|| LuceneError::illegal_state("segment info is missing"))?;

      // find SegmentReader for this segment
      let old_reader = segment_readers
        .get(&commit_info.info.name)
        .map(|old_reader_index| old_readers[*old_reader_index].clone());

      // Make a best effort to detect when the app illegally "rm -rf" their
      // index while a reader was open, and then called openIfChanged:
      if let Some(old_reader) = &old_reader
        && commit_info.info.get_id() != old_reader.get_segment_info().info.get_id()
      {
        return Err(LuceneError::illegal_state(format!(
          "same segment {} has invalid doc count change; likely you are re-opening a reader after illegally removing index files yourself and building a new index in their place.  Use IndexWriter.deleteAll or open a new IndexWriter using OpenMode.CREATE instead",
          commit_info.info.name
        )));
      }

      let new_reader = match old_reader {
        None => Arc::new(SegmentReader::new(
          commit_info,
          infos.get_index_created_version_major(),
          &IOContext::default_io_context()?,
        )?),
        Some(old_reader)
          if commit_info.info.get_use_compound_file()
            != old_reader.get_segment_info().info.get_use_compound_file() =>
        {
          Arc::new(SegmentReader::new(
            commit_info,
            infos.get_index_created_version_major(),
            &IOContext::default_io_context()?,
          )?)
        },
        Some(old_reader) => {
          if old_reader.is_nrt {
            // We must load liveDocs/DV updates from disk:
            let (live_docs, hard_live_docs) = if commit_info.has_deletions() {
              let live_docs = Arc::new(LATEST_CODEC.live_docs_format().read_live_docs(
                commit_info.info.dir.as_ref(),
                commit_info,
                &IOContext::read_once_io_context()?,
              )?);
              (
                Some(DocBits::A(live_docs.clone())),
                Some(DocBits::A(live_docs)),
              )
            } else {
              (None, None)
            };
            Arc::new(SegmentReader::new_from_reader(
              commit_info,
              &old_reader,
              live_docs,
              hard_live_docs,
              commit_info.info.max_doc()? - commit_info.get_del_count(),
              false,
            )?)
          } else if old_reader.get_segment_info().get_del_gen() == commit_info.get_del_gen()
            && old_reader.get_segment_info().get_field_infos_gen()
              == commit_info.get_field_infos_gen()
          {
            // No change; this reader will be shared between
            // the old and the new one, so we must incRef
            // it:
            old_reader.inc_ref()?;
            old_reader
          } else if old_reader.get_segment_info().get_del_gen() == commit_info.get_del_gen() {
            // only DV updates
            Arc::new(SegmentReader::new_from_reader(
              commit_info,
              &old_reader,
              old_reader.get_live_docs()?,
              old_reader.get_hard_live_docs()?,
              old_reader.num_docs()?,
              false,
            )?)
          } else {
            // both DV and liveDocs have changed
            let (live_docs, hard_live_docs) = if commit_info.has_deletions() {
              let live_docs = Arc::new(LATEST_CODEC.live_docs_format().read_live_docs(
                commit_info.info.dir.as_ref(),
                commit_info,
                &IOContext::read_once_io_context()?,
              )?);
              (
                Some(DocBits::A(live_docs.clone())),
                Some(DocBits::A(live_docs)),
              )
            } else {
              (None, None)
            };
            Arc::new(SegmentReader::new_from_reader(
              commit_info,
              &old_reader,
              live_docs,
              hard_live_docs,
              commit_info.info.max_doc()? - commit_info.get_del_count(),
              false,
            )?)
          }
        },
      };
      new_readers[i] = Some(new_reader);
    }
    Ok(())
  })();

  if let Err(e) = result {
    dec_ref_while_handling_exception(new_readers);
    return Err(e);
  }

  let readers = new_readers
    .into_iter()
    .map(|reader| reader.ok_or_else(|| LuceneError::illegal_state("segment reader is missing")))
    .collect::<Result<Vec<_>>>()?;
  StandardDirectoryReader::new(directory, readers, infos, leaf_sorter, false, false, None)
}

fn dec_ref_while_handling_exception<D, I>(readers: I)
where
  D: Directory,
  I: IntoIterator<Item = Option<DefaultLeafReader<D>>>,
{
  for reader in readers.into_iter().flatten() {
    let _ = reader.dec_ref();
  }
}

impl<C, D> BaseCompositeReader for StandardDirectoryReader<C, D>
where
  C: Comparator<DefaultLeafReader<D>> + Clone,
  D: Directory,
{
}

impl<C, D> CompositeReader for StandardDirectoryReader<C, D>
where
  C: Comparator<DefaultLeafReader<D>> + Clone,
  D: Directory,
{
  type LeafReader = DefaultLeafReader<D>;

  type SubCompositeReader = DummyCompositeReader<DefaultLeafReader<D>>;

  fn get_sequential_sub_readers(
    &self,
  ) -> &[IndexReaderEnum<Self::LeafReader, Self::SubCompositeReader>] {
    self.base_composite_reader_base.get_sequential_sub_readers()
  }

  fn as_string(&self) -> String {
    let mut buffer = String::new();
    buffer.push_str("StandardDirectoryReader");
    buffer.push('(');
    if let Some(segments_file) = self.segment_infos.get_segments_file_name() {
      buffer.push_str(&segments_file);
      buffer.push(':');
      buffer.push_str(&self.segment_infos.get_version().to_string());
    }
    if self.writer_closed.is_some() {
      buffer.push_str(":nrt");
    }
    for r in self.get_sequential_sub_readers() {
      buffer.push(' ');
      buffer.push_str(&r.to_string());
    }
    buffer.push(')');
    buffer
  }
}

impl<C, D> IndexReader for StandardDirectoryReader<C, D>
where
  C: Comparator<DefaultLeafReader<D>> + Clone,
  D: Directory,
{
  type TermVectors =
    BCRTermVectorsImpl<DefaultLeafReader<D>, DummyCompositeReader<DefaultLeafReader<D>>>;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    self.base_composite_reader_base.term_vector(self)
  }

  fn max_doc(&self) -> Result<i32> {
    Ok(self.base_composite_reader_base.max_doc())
  }

  fn num_docs(&self) -> Result<i32> {
    self.base_composite_reader_base.num_docs()
  }

  type StoredFields =
    BCRStoredFieldsImpl<DefaultLeafReader<D>, DummyCompositeReader<DefaultLeafReader<D>>>;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    self.base_composite_reader_base.stored_fields(self)
  }

  fn do_close(&self) -> Result<()> {
    // TODO
    Ok(())
  }

  type ReaderCacheHelper = CacheHelperImpl;

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    Ok(Some(self.cache_helper.clone()))
  }

  fn doc_freq(&self, term: &Term) -> Result<i32> {
    self.base_composite_reader_base.doc_freq(term, self)
  }

  fn total_term_freq(&self, term: &Term) -> Result<i64> {
    self.base_composite_reader_base.total_term_freq(term, self)
  }

  fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
    self
      .base_composite_reader_base
      .get_sum_doc_freq(field, self)
  }

  fn get_doc_count(&self, field: &str) -> Result<i32> {
    self.base_composite_reader_base.get_doc_count(field, self)
  }

  fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
    self
      .base_composite_reader_base
      .get_sum_total_term_freq(field, self)
  }

  fn index_base(&self) -> &IndexReaderBase {
    &self.index_base
  }
}

impl<C, D> Display for StandardDirectoryReader<C, D>
where
  C: Comparator<DefaultLeafReader<D>> + Clone,
  D: Directory,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.as_string())
  }
}

impl<C, D> DirectoryReader for StandardDirectoryReader<C, D>
where
  C: Comparator<DefaultLeafReader<D>> + Clone,
  D: Directory,
{
  type DirectoryReader = StandardDirectoryReader<C, D>;

  fn do_open_if_changed(
    &self,
    writer: IndexWriter<Self::Directory>,
  ) -> Result<Option<Self::DirectoryReader>> {
    self.do_open_if_changed_with_commit::<DummyIndexCommit<D>>(writer, None)
  }

  fn do_open_if_changed_with_commit<IC>(
    &self,
    writer: IndexWriter<Self::Directory>,
    commit: Option<&IC>,
  ) -> Result<Option<Self::DirectoryReader>>
  where
    IC: IndexCommit<Directory = D>,
  {
    self.ensure_open()?;
    if self.writer_closed.is_some() {
      self.do_open_from_writer(&writer, commit)
    } else {
      self.do_open_no_writer(&writer, commit)
    }
  }

  fn do_open_if_changed_with_writer(
    &self,
    writer: IndexWriter<Self::Directory>,
    apply_deletes: bool,
  ) -> Result<Option<Self::DirectoryReader>> {
    self.ensure_open()?;
    if self
      .writer_closed
      .as_ref()
      .is_some_and(|closed| Arc::ptr_eq(closed, &writer.closed))
      && apply_deletes == self.apply_all_deletes
    {
      self.do_open_from_writer::<DummyIndexCommit<D>>(&writer, None)
    } else {
      Ok(Some(writer.get_reader_with_leaf_sorter(
        apply_deletes,
        self.write_all_deletes,
        self.sub_reader_sorter.clone(),
      )?))
    }
  }

  fn get_version(&self) -> Result<i64> {
    self.ensure_open()?;
    Ok(self.segment_infos.get_version())
  }

  fn is_current<D1>(&self, index_writer: &IndexWriter<D1>) -> Result<bool>
  where
    D1: Directory,
  {
    self.ensure_open()?;

    let reader_from_dir = match self.writer_closed {
      Some(ref closed) => closed.load(SeqCst),
      None => true,
    };
    if reader_from_dir {
      let latest = SegmentInfos::read_latest_commit(self.directory().directory.clone())?;
      let version = self.segment_infos.get_version();
      Ok(latest.get_version() == version)
    } else {
      index_writer.nrt_is_current(self.segment_infos.get_version())
    }
  }

  type IndexCommit = ReaderCommit<D>;

  fn get_index_commit(&self) -> Result<Self::IndexCommit> {
    self.ensure_open()?;
    ReaderCommit::new(&self.segment_infos, self.directory().directory.clone())
  }

  type Directory = D;

  fn directory(&self) -> &DirectoryReaderBase<Self::Directory> {
    &self.directory_reader_base
  }
}
#[derive(Clone)]
pub struct CacheHelperImpl {
  cache_key: CacheKey,
}
impl CacheHelperImpl {
  fn new() -> Self {
    Self {
      cache_key: CacheKey::new(),
    }
  }
}
impl CacheHelper for CacheHelperImpl {
  fn get_key(&self) -> CacheKey {
    self.cache_key.clone()
  }
}
pub struct FindSegmentsFileImpl1<D, C>
where
  D: Directory,
  C: Comparator<DefaultLeafReader<D>> + Clone,
{
  min_supported_major_version: i32,
  directory: Arc<D>,
  leaf_sorter: Option<C>,
}
impl<D, C> FindSegmentsFileImpl1<D, C>
where
  D: Directory,
  C: Comparator<DefaultLeafReader<D>> + Clone,
{
  pub fn new(min_supported_major_version: i32, directory: Arc<D>, leaf_sorter: Option<C>) -> Self {
    FindSegmentsFileImpl1 {
      min_supported_major_version,
      directory,
      leaf_sorter,
    }
  }
}
impl<D, C> FindSegmentsFile for FindSegmentsFileImpl1<D, C>
where
  D: Directory,
  C: Comparator<DefaultLeafReader<D>> + Clone,
{
  type V = StandardDirectoryReader<C, D>;
  type D = D;

  fn get_directory_point(&self) -> Arc<Self::D> {
    self.directory.clone()
  }

  fn do_body(&mut self, segment_file_name: &str) -> Result<Self::V> {
    if self.min_supported_major_version > LATEST.major || self.min_supported_major_version < 0 {
      return Err(LuceneError::illegal_argument(format!(
        "minSupportedMajorVersion must be positive and <= {} but was: {}",
        LATEST.major, self.min_supported_major_version
      )));
    }

    let sis = SegmentInfos::read_commit_with_file_min_version(
      self.directory.clone(),
      segment_file_name,
      self.min_supported_major_version,
    )?;

    let mut readers = Vec::with_capacity(sis.size());

    // ensure cleanup on failure
    for i in 0..sis.size() {
      debug_assert!(sis.info(i).is_some());
      let reader = SegmentReader::new(
        sis.info(i).as_ref().unwrap(),
        sis.get_index_created_version_major(),
        &IOContext::default_io_context()?,
      )?;
      readers.push(Arc::new(reader));
    }
    // This may throw CorruptIndexException if there are too many docs, so
    // it must be inside try clause so we close readers in that case:
    let reader = StandardDirectoryReader::new(
      self.directory.clone(),
      readers,
      sis,
      self.leaf_sorter.take(),
      false,
      false,
      None,
    )?;

    Ok(reader)
  }
}

pub struct FindSegmentsFileImpl2<D, C>
where
  D: Directory,
  C: Comparator<DefaultLeafReader<D>> + Clone,
{
  directory: Arc<D>,
  old_readers: Vec<DefaultLeafReader<D>>,
  leaf_sorter: Option<C>,
}
impl<D, C> FindSegmentsFileImpl2<D, C>
where
  D: Directory,
  C: Comparator<DefaultLeafReader<D>> + Clone,
{
  pub fn new(
    directory: Arc<D>,
    old_readers: Vec<DefaultLeafReader<D>>,
    leaf_sorter: Option<C>,
  ) -> Self {
    FindSegmentsFileImpl2 {
      directory,
      old_readers,
      leaf_sorter,
    }
  }
}
impl<D, C> FindSegmentsFile for FindSegmentsFileImpl2<D, C>
where
  D: Directory,
  C: Comparator<DefaultLeafReader<D>> + Clone,
{
  type V = StandardDirectoryReader<C, D>;
  type D = D;

  fn get_directory_point(&self) -> Arc<Self::D> {
    self.directory.clone()
  }

  fn do_body(&mut self, segment_file_name: &str) -> Result<Self::V> {
    let infos = SegmentInfos::read_commit(self.directory.clone(), segment_file_name)?;
    do_open_if_changed(
      infos,
      self.directory.clone(),
      self.old_readers.clone(),
      self.leaf_sorter.clone(),
    )
  }
}
pub(crate) fn do_open_if_changed<D, C>(
  infos: SegmentInfos<D>,
  directory: Arc<D>,
  old_readers: Vec<DefaultLeafReader<D>>,
  sub_readers_sorter: Option<C>,
) -> Result<StandardDirectoryReader<C, D>>
where
  D: Directory,
  C: Comparator<DefaultLeafReader<D>> + Clone,
{
  open_with_leaf_sorter(directory, infos, old_readers, sub_readers_sorter)
}
#[derive(Clone)]
pub struct EmptyLeafSorter;
impl<D> Comparator<DefaultLeafReader<D>> for EmptyLeafSorter
where
  D: Directory,
{
  const TYPE: &'static str = "EmptyLeafSorter";

  fn compare(&self, _a: &DefaultLeafReader<D>, _b: &DefaultLeafReader<D>) -> Result<i32> {
    Ok(0)
  }
}

pub struct ReaderCommit<D>
where
  D: Directory,
{
  segments_file_name: String,
  files: Vec<String>,
  dir: Arc<D>,
  generation: i64,
  user_data: HashMap<String, String>,
  segment_count: usize,
}

impl<D> ReaderCommit<D>
where
  D: Directory,
{
  pub(crate) fn new(infos: &SegmentInfos<D>, dir: Arc<D>) -> Result<Self> {
    let segments_file_name = infos
      .get_segments_file_name()
      .ok_or_else(|| LuceneError::illegal_state("segments file name is None"))?;
    let mut files: Vec<String> = infos.files(true)?.into_iter().collect();
    files.sort();
    let user_data = infos.get_user_data().clone();
    let generation = infos.get_generation();
    let segment_count = infos.size();

    // NOTE: we intentionally do not incRef this! Else we'd need to make IndexCommit Closeable.
    Ok(Self {
      segments_file_name,
      files,
      dir,
      generation,
      user_data,
      segment_count,
    })
  }
}

impl<D> Display for ReaderCommit<D>
where
  D: Directory,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "StandardDirectoryReader.ReaderCommit({} files={:?})",
      self.segments_file_name, self.files
    )
  }
}

impl<D> PartialEq for ReaderCommit<D>
where
  D: Directory,
{
  fn eq(&self, other: &Self) -> bool {
    is_same_commit(self, other)
  }
}

impl<D> Eq for ReaderCommit<D> where D: Directory {}

impl<D> PartialOrd for ReaderCommit<D>
where
  D: Directory,
{
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl<D> Ord for ReaderCommit<D>
where
  D: Directory,
{
  fn cmp(&self, other: &Self) -> Ordering {
    cmp_commit(self, other)
  }
}

impl<D> IndexCommit for ReaderCommit<D>
where
  D: Directory,
{
  fn get_segments_file_name(&self) -> &str {
    &self.segments_file_name
  }

  fn get_file_names(&self) -> Result<&[String]> {
    Ok(&self.files)
  }

  type Directory = D;

  fn get_directory(&self) -> Arc<Self::Directory> {
    self.dir.clone()
  }

  fn delete(&mut self) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "This IndexCommit does not support deletions",
    ))
  }

  fn is_deleted(&self) -> bool {
    false
  }

  fn get_segment_count(&self) -> usize {
    self.segment_count
  }

  fn get_generation(&self) -> i64 {
    self.generation
  }

  fn get_user_data(&self) -> &HashMap<String, String> {
    &self.user_data
  }
}

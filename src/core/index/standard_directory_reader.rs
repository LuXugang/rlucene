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
use crate::core::index::base_composite_reader::{
  BCRStoredFieldsImpl, BCRTermVectorsImpl, BaseCompositeReader, BaseCompositeReaderBase,
};
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::directory_reader::{DirectoryReader, DirectoryReaderBase};
use crate::core::index::dummy::dummy_composite_reader::DummyCompositeReader;
use crate::core::index::dummy::dummy_directory_reader::DummyDirectoryReader;
use crate::core::index::dummy::dummy_index_commit::DummyIndexCommit;
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_reader::{
  CacheHelper, CacheKey, IndexReader, IndexReaderBase, IndexReaderEnum,
};
use crate::core::index::index_writer::{IndexWriter, IndexWriterBase, Inner};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicy;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::{FindSegmentsFile, SegmentInfos};
use crate::core::index::segment_reader::SegmentReader;
use crate::core::index::term::Term;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::dummy::dummy_comparator::DummyComparator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_function::IOFunction;
use crate::core::util::{Comparator, LATEST, MIN_SUPPORTED_MAJOR};
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;

pub struct StandardDirectoryReader<C, D>
where
  C: Comparator<Arc<SegmentReader<D>>>,
  D: Directory,
{
  base_composite_reader_base:
    BaseCompositeReaderBase<Arc<SegmentReader<D>>, DummyCompositeReader<Arc<SegmentReader<D>>>>,
  directory_reader_base: DirectoryReaderBase<D>,
  apply_all_deletes: bool,
  write_all_deletes: bool,
  // if Some, this reader owns the SegmentInfos, else from IndexWriter
  pub(crate) segment_infos: Option<SegmentInfos<D>>,
  sub_reader_sorter: Option<C>,
  index_base: IndexReaderBase,
  closed: Option<Arc<AtomicBool>>,
  cache_helper: CacheHelperImpl,
}
impl<C, D> StandardDirectoryReader<C, D>
where
  C: Comparator<Arc<SegmentReader<D>>>,
  D: Directory,
{
  pub(crate) fn new(
    directory: Arc<D>,
    readers: Vec<Arc<SegmentReader<D>>>,
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
      segment_infos: Some(segment_infos),
      sub_reader_sorter: leaf_sorter,
      index_base: IndexReaderBase::new(),
      closed,
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
    C: Comparator<Arc<SegmentReader<D>>>,
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
    C: Comparator<Arc<SegmentReader<D>>>,
    IC: IndexCommit<Directory = D>,
  {
    let mut finder =
      FindSegmentsFileImpl1::new(min_supported_major_version, directory.clone(), leaf_sorter);
    match commit {
      Some(c) => finder.run_with_commit(c),
      None => finder.run(),
    }
  }
}
pub type StandardDirectoryReaderType<D> = StandardDirectoryReader<DummyComparator, D>;
pub(crate) fn open_with_reader_function<D, B, IO>(
  writer: &IndexWriter<D, B>,
  reader_function: &mut IO,
  infos: Option<&SegmentInfos<D>>,
  inner: &mut Inner<D>, // hold IndexWriter lock
  apply_all_deletes: bool,
  write_all_deletes: bool,
) -> Result<StandardDirectoryReaderType<D>>
where
  D: Directory,
  B: IndexWriterBase,
  IO: IOFunction<SegmentCommitInfo<D>, Arc<SegmentReader<D>>>,
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
      // TODO IMPORTANT 这里不对 要从LiveIndexWriterConfig中获取
      None,
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

impl<C, D> BaseCompositeReader for StandardDirectoryReader<C, D>
where
  C: Comparator<Arc<SegmentReader<D>>>,
  D: Directory,
{
}

impl<C, D> CompositeReader for StandardDirectoryReader<C, D>
where
  C: Comparator<Arc<SegmentReader<D>>>,
  D: Directory,
{
  type LeafReader = Arc<SegmentReader<D>>;

  type SubCompositeReader = DummyCompositeReader<Arc<SegmentReader<D>>>;

  fn get_sequential_sub_readers(
    &self,
  ) -> &[IndexReaderEnum<Self::LeafReader, Self::SubCompositeReader>] {
    self.base_composite_reader_base.get_sequential_sub_readers()
  }

  fn to_string(&self) -> String {
    todo!()
  }
}

impl<C, D> IndexReader for StandardDirectoryReader<C, D>
where
  C: Comparator<Arc<SegmentReader<D>>>,
  D: Directory,
{
  type TermVectors =
    BCRTermVectorsImpl<Arc<SegmentReader<D>>, DummyCompositeReader<Arc<SegmentReader<D>>>>;

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
    BCRStoredFieldsImpl<Arc<SegmentReader<D>>, DummyCompositeReader<Arc<SegmentReader<D>>>>;

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
  C: Comparator<Arc<SegmentReader<D>>>,
  D: Directory,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    // TODO
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl<C, D> DirectoryReader for StandardDirectoryReader<C, D>
where
  C: Comparator<Arc<SegmentReader<D>>>,
  D: Directory,
{
  type DirectoryReader = DummyDirectoryReader<D>;

  fn do_open_if_changed(&self) -> Result<Option<Self::DirectoryReader>> {
    self.do_open_if_changed_with_commit::<DummyIndexCommit<D>>(None)
  }

  fn do_open_if_changed_with_commit<IC>(
    &self,
    _commit: Option<&IC>,
  ) -> Result<Option<Self::DirectoryReader>>
  where
    IC: IndexCommit,
  {
    todo!()
  }

  fn do_open_if_changed_with_index_writer<B>(
    &self,
    _writer: IndexWriter<Self::Directory, B>,
    _apply_deletes: bool,
  ) -> Result<Option<Self::DirectoryReader>>
  where
    B: IndexWriterBase,
  {
    todo!()
  }

  fn get_version(&self) -> i64 {
    todo!()
  }

  fn is_current<D1, B>(&self, index_writer: &IndexWriter<D1, B>) -> Result<bool>
  where
    D1: Directory,
    B: IndexWriterBase,
  {
    self.ensure_open()?;

    let reader_from_dir = match self.closed {
      Some(ref closed) => closed.load(SeqCst),
      None => true,
    };
    if reader_from_dir {
      let latest = SegmentInfos::read_latest_commit(self.directory().directory.clone())?;
      let version = match self.segment_infos {
        // writer is null
        Some(ref sis) => sis.get_version(),
        // writer != null and writer.isClosed is true
        None => index_writer.get_segment_infos_version(),
      };
      Ok(latest.get_version() == version)
    } else {
      match self.segment_infos {
        Some(ref sis) => index_writer.nrt_is_current(sis.get_version()),
        None => Err(LuceneError::illegal_state(
          "StandardDirectoryReader should own segment_infos ",
        )),
      }
    }
  }

  type IndexCommit = DummyIndexCommit<D>;

  fn get_index_commit(&self) -> Result<Self::IndexCommit> {
    todo!()
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
  C: Comparator<Arc<SegmentReader<D>>>,
{
  min_supported_major_version: i32,
  directory: Arc<D>,
  leaf_sorter: Option<C>,
}
impl<D, C> FindSegmentsFileImpl1<D, C>
where
  D: Directory,
  C: Comparator<Arc<SegmentReader<D>>>,
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
  C: Comparator<Arc<SegmentReader<D>>>,
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

pub struct FindSegmentsFileImpl2<D>
where
  D: Directory,
{
  directory: Arc<D>,
}
impl<D> FindSegmentsFileImpl2<D>
where
  D: Directory,
{
  pub fn new(directory: Arc<D>) -> Self {
    FindSegmentsFileImpl2 { directory }
  }
}
impl<D> FindSegmentsFile for FindSegmentsFileImpl2<D>
where
  D: Directory,
{
  type V = ();
  type D = D;

  fn get_directory_point(&self) -> Arc<Self::D> {
    self.directory.clone()
  }

  fn do_body(&mut self, segment_file_name: &str) -> Result<Self::V> {
    let _infos = SegmentInfos::read_commit(self.directory.clone(), segment_file_name)?;
    todo!()
  }
}

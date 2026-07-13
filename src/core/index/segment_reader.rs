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
use crate::core::codecs::compound_directory::CompoundDirectory;
use crate::core::codecs::doc_values_producer::{
  DefaultDocValuesProducer, DocValuesProducer, DocValuesProducerEnum2,
};
use crate::core::codecs::field_infos_format::FieldInfosFormat;
use crate::core::codecs::live_docs_format::LiveDocsFormat;
use crate::core::codecs::norms_producer::{DefaultNormProducer, NormsProducer};
use crate::core::codecs::points_reader::{DefaultPointsReader, PointsReader};

use crate::core::codecs::Codec;
use crate::core::codecs::fields_producer::DefaultFieldsProducer;
use crate::core::codecs::knn_vectors_reader::{DefaultKnnVectorsReader, KnnVectorsReader};
use crate::core::codecs::stored_fields_reader::DefaultStoredFieldsReader;
use crate::core::codecs::term_vectors_reader::DefaultTermVectorsReader;
use crate::core::index::codec_reader::{CodecReader, StoredFieldsType, TermVectorsType};
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::fields::Fields;
use crate::core::index::index_reader::{CacheHelper, CacheKey, IndexReader, IndexReaderBase};
use crate::core::index::leaf_metadata::LeafMetaData;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::pending_deletes::DocBits;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_core_readers::{
  SegmentCoreReaders, SegmentCoreReadersCacheHelperImpl,
};
use crate::core::index::segment_doc_values::SegmentDocValues;
use crate::core::index::segment_doc_values_producer::SegmentDocValuesProducer;
use crate::core::index::term::Term;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::bits::{Bits, BitsEnum2};
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

/// IndexReader implementation over a single segment.
/// Instances pointing to the same segment
/// (but with different deletes, etc.) may share the same core data
pub struct SegmentReader<D>
where
  D: Directory,
{
  pub(crate) si: SegmentCommitInfo<D>,
  pub(crate) original_si_id: String,
  pub(crate) original_si_dir: Arc<D>,
  meta_data: LeafMetaData,
  live_docs: Option<DocBits>,
  hard_live_docs: Option<DocBits>,
  // Normally set to si.maxDoc - si.delDocCount, unless we
  // were created as an NRT reader from IW, in which case IW
  // tells us the number of live docs:
  num_docs: i32,
  core: Arc<SegmentCoreReaders<D>>,
  seg_doc_values: Arc<SegmentDocValues<D>>,
  /// True if we are holding RAM only liveDocs or DV updates,
  /// i.e. the SegmentCommitInfo delGen doesn't match our liveDocs.
  pub(crate) is_nrt: bool,
  doc_values_producer: Option<Arc<DocValuesProducers<D>>>,
  field_infos: Arc<FieldInfos>,
  index_base: IndexReaderBase,
  reader_cache_helper: CacheHelperImpl,
}
impl<D> SegmentReader<D>
where
  D: Directory,
{
  pub(crate) fn new(
    si: &SegmentCommitInfo<D>,
    created_version_major: i32,
    context: &IOContext,
  ) -> Result<Self> {
    let si = si.clone();
    let meta_data = LeafMetaData::new(
      created_version_major,
      si.info.get_min_version(),
      si.info.get_index_sort().clone(),
      si.info.get_has_blocks(),
    )?;

    let is_nrt = false;
    let core = Arc::new(SegmentCoreReaders::new(si.info.dir.as_ref(), &si, context)?);
    let seg_doc_values = Arc::new(SegmentDocValues::new());
    let num_docs = si.info.max_doc()? - si.get_del_count();
    let info_id = si.info.get_id_key().to_string();
    let dir = si.info.dir.clone();
    let mut segment_reader = Self {
      si,
      original_si_id: info_id,
      original_si_dir: dir,
      meta_data,
      is_nrt,
      core,
      seg_doc_values,
      hard_live_docs: None,
      live_docs: None,
      num_docs,
      field_infos: Arc::new(FieldInfos::default()),
      doc_values_producer: None,
      index_base: IndexReaderBase::new(),
      reader_cache_helper: CacheHelperImpl::new(),
    };
    let result = (|| {
      let si = &segment_reader.si;
      let (hard_live_docs, live_docs) = if si.has_deletions() {
        // NOTE: the bitvector is stored using the regular directory, not cfs
        let ld = Arc::new(si.info.get_codec()?.live_docs_format().read_live_docs(
          si.info.dir.as_ref(),
          si,
          &IOContext::read_once_io_context()?,
        )?);
        (Some(DocBits::A(ld.clone())), Some(DocBits::A(ld)))
      } else {
        debug_assert_eq!(si.get_del_count(), 0);
        (None, None)
      };

      let field_infos = Self::init_field_infos(si, segment_reader.core.core_field_infos.clone())?;
      let doc_values_producer = Self::init_doc_values_producer(
        si,
        field_infos.clone(),
        &segment_reader.seg_doc_values,
        &segment_reader.core,
      )?;

      debug_assert!(Self::assert_live_docs(
        is_nrt,
        hard_live_docs.as_ref(),
        live_docs.as_ref()
      )?);

      Ok((hard_live_docs, live_docs, field_infos, doc_values_producer))
    })();
    match result {
      Ok(r) => {
        segment_reader.hard_live_docs = r.0;
        segment_reader.live_docs = r.1;
        segment_reader.field_infos = r.2;
        segment_reader.doc_values_producer = r.3;
        Ok(segment_reader)
      },
      Err(e) => {
        segment_reader.do_close()?;
        Err(e)
      },
    }
  }
  /// Create new SegmentReader sharing core from a previous SegmentReader and using the provided liveDocs,
  /// and recording whether those liveDocs were carried in ram (isNRT=true).
  pub(crate) fn new_from_reader(
    si: &SegmentCommitInfo<D>,
    sr: &SegmentReader<D>,
    live_docs: Option<DocBits>,
    hard_live_docs: Option<DocBits>,
    num_docs: i32,
    is_nrt: bool,
  ) -> Result<Self> {
    let si = si.clone();
    let max_doc = si.info.max_doc()?;
    if num_docs > max_doc {
      return Err(LuceneError::illegal_argument(format!(
        "numDocs={} but maxDoc={}",
        num_docs, max_doc
      )));
    }
    if let Some(ld) = &live_docs {
      let len = ld.length();
      if len != max_doc as usize {
        return Err(LuceneError::illegal_argument(format!(
          "maxDoc={} but liveDocs.size()={}",
          max_doc, len
        )));
      }
    }

    let meta_data = sr.meta_data.clone();
    let core = sr.core.clone();
    let seg_doc_values = sr.seg_doc_values.clone();
    core.inc_ref()?;
    debug_assert!(Self::assert_live_docs(
      is_nrt,
      hard_live_docs.as_ref(),
      live_docs.as_ref()
    )?);
    let info_id = si.info.get_id_key().to_string();
    let dir = si.info.dir.clone();
    let mut segment_reader = Self {
      si,
      original_si_id: info_id,
      original_si_dir: dir,
      meta_data,
      is_nrt,
      core: core.clone(),
      seg_doc_values: seg_doc_values.clone(),
      hard_live_docs,
      live_docs,
      num_docs,
      field_infos: Arc::new(FieldInfos::default()),
      doc_values_producer: None,
      index_base: IndexReaderBase::new(),
      reader_cache_helper: CacheHelperImpl::new(),
    };
    let result = (|| {
      let si = &segment_reader.si;
      let field_infos = Self::init_field_infos(si, core.core_field_infos.clone())?;
      let doc_values_producer =
        Self::init_doc_values_producer(si, field_infos.clone(), &seg_doc_values, &core)?;
      Ok((field_infos, doc_values_producer))
    })();
    match result {
      Ok(r) => {
        segment_reader.field_infos = r.0;
        segment_reader.doc_values_producer = r.1;
        Ok(segment_reader)
      },
      Err(e) => {
        segment_reader.do_close()?;
        Err(e)
      },
    }
  }
  fn assert_live_docs(
    is_nrt: bool,
    hard_live_docs: Option<&DocBits>,
    live_docs: Option<&DocBits>,
  ) -> Result<bool> {
    match is_nrt {
      true => debug_assert!(
        hard_live_docs.is_none() || live_docs.is_some(),
        "liveDocs must be non-null if hardLiveDocs are non-null"
      ),
      false => debug_assert!(
        match (hard_live_docs, live_docs) {
          (None, None) => true,
          (Some(reader_bits), Some(current_bits)) => match (reader_bits, current_bits) {
            (BitsEnum2::A(r_bits), BitsEnum2::A(c_bits)) => Arc::ptr_eq(r_bits, c_bits),
            (BitsEnum2::B(r_bits), BitsEnum2::B(c_bits)) => match (r_bits, c_bits) {
              (BitsEnum2::A(r_fixed), BitsEnum2::A(c_fixed)) => {
                Arc::ptr_eq(r_fixed, c_fixed)
              },
              (BitsEnum2::B(_), BitsEnum2::B(_)) => {
                return Err(LuceneError::illegal_state(
                  "live docs should be FixedBitSet",
                ));
              },
              _ => false,
            },
            _ => false,
          },
          _ => false,
        },
        "non-nrt case must have identical liveDocs"
      ),
    }
    Ok(true)
  }
  /// init most recent DocValues for the current commit
  fn init_doc_values_producer(
    si: &SegmentCommitInfo<D>,
    field_infos: Arc<FieldInfos>,
    seg_doc_values: &SegmentDocValues<D>,
    core: &SegmentCoreReaders<D>,
  ) -> Result<Option<Arc<DocValuesProducers<D>>>> {
    if !field_infos.has_doc_values() {
      return Ok(None);
    }
    let dir = &core.cfs_reader;

    let producer = match si.has_field_updates() {
      true => DocValuesProducerEnum2::A(SegmentDocValuesProducer::new(
        si,
        dir.as_ref(),
        Arc::clone(&core.core_field_infos),
        &field_infos,
        seg_doc_values,
      )?),
      // simple case, no DocValues updates
      false => DocValuesProducerEnum2::B(seg_doc_values.get_doc_values_producer(
        -1,
        si,
        dir.as_ref(),
        field_infos,
      )?),
    };

    Ok(Some(Arc::new(producer)))
  }
  /// init most recent FieldInfos for the current commit
  fn init_field_infos(
    si: &SegmentCommitInfo<D>,
    core_field_infos: Arc<FieldInfos>,
  ) -> Result<Arc<FieldInfos>> {
    if !si.has_field_updates() {
      return Ok(core_field_infos);
    }

    // updates always outside of CFS
    let fis_format = si.info.get_codec()?.field_infos_format();
    let segment_suffix = num_bigint::BigInt::from(si.get_field_infos_gen()).to_str_radix(36);

    let infos = fis_format.read(
      si.info.dir.as_ref(),
      &si.info,
      &segment_suffix,
      &IOContext::read_once_io_context()?,
    )?;

    Ok(Arc::new(infos))
  }
  /// Return the name of the segment this reader is reading.
  pub fn get_segment_name(&self) -> &str {
    &self.si.info.name
  }
  /// Return the SegmentInfoPerCommit of the segment this reader is reading.
  pub fn get_segment_info(&self) -> &SegmentCommitInfo<D> {
    &self.si
  }
  #[cfg(test)]
  #[allow(invalid_reference_casting)]
  #[allow(clippy::mut_from_ref)]
  pub(crate) fn get_segment_info_mut(&self) -> &mut SegmentCommitInfo<D> {
    unsafe { &mut *(&self.si as *const SegmentCommitInfo<D> as *mut SegmentCommitInfo<D>) }
  }
  /// Returns the directory this index resides in.
  pub fn directory(&self) -> &D {
    self.si.info.dir.as_ref()
  }
  pub fn get_original_segment_info_id(&self) -> &str {
    &self.original_si_id
  }
  pub fn get_original_dir(&self) -> Arc<D> {
    self.original_si_dir.clone()
  }

  pub fn get_hard_live_docs(&self) -> Result<Option<DocBits>> {
    match &self.hard_live_docs {
      Some(DocBits::A(a)) => Ok(Some(DocBits::A(Arc::clone(a)))),
      Some(DocBits::B(b)) => match b {
        BitsEnum2::A(a) => Ok(Some(DocBits::B(BitsEnum2::A(Arc::clone(a))))),
        BitsEnum2::B(_) => Err(LuceneError::illegal_state(
          "hard live docs should be FixedBitSet",
        )),
      },
      None => Ok(None),
    }
  }
}
pub type DocValuesProducers<D> = DocValuesProducerEnum2<
  SegmentDocValuesProducer<D>,
  Arc<DefaultDocValuesProducer<<D as Directory>::IndexInput>>,
>;

impl<D> Display for SegmentReader<D>
where
  D: Directory,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.si.info.max_doc() {
      Ok(max_doc) => {
        let pending_del_count = max_doc - self.num_docs - self.si.get_del_count();
        write!(
          f,
          "{}",
          self.si.to_string_with_pending_del_count(pending_del_count)
        )
      },
      Err(e) => write!(f, "{}", e),
    }
  }
}

impl<D> IndexReader for SegmentReader<D>
where
  D: Directory,
{
  type TermVectors = TermVectorsType<<Self as CodecReader>::TermVectorsReader>;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    CodecReader::term_vectors(self)
  }

  fn max_doc(&self) -> Result<i32> {
    self.si.info.max_doc()
  }

  fn num_docs(&self) -> Result<i32> {
    Ok(self.num_docs)
  }

  type StoredFields = StoredFieldsType<<Self as CodecReader>::StoredFieldsReader>;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    CodecReader::stored_fields(self)
  }

  fn do_close(&self) -> Result<()> {
    let core_result =
      std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.core.dec_ref()));
    let doc_values_result = match &self.doc_values_producer {
      Some(dv) => match dv.as_ref() {
        DocValuesProducerEnum2::A(a) => self.seg_doc_values.dec_ref(&a.dv_gens),
        DocValuesProducerEnum2::B(_) => self.seg_doc_values.dec_ref(&[-1]),
      },
      None => Ok(()),
    };

    doc_values_result?;
    match core_result {
      Ok(result) => result,
      Err(payload) => std::panic::resume_unwind(payload),
    }
  }

  type ReaderCacheHelper = CacheHelperImpl;

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    Ok(Some(self.reader_cache_helper.clone()))
  }

  fn doc_freq(&self, term: &Term) -> Result<i32> {
    LeafReader::doc_freq(self, term)
  }

  fn total_term_freq(&self, term: &Term) -> Result<i64> {
    LeafReader::get_total_term_freq(self, term)
  }

  fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
    LeafReader::get_sum_doc_freq(self, field)
  }

  fn get_doc_count(&self, field: &str) -> Result<i32> {
    LeafReader::get_doc_count(self, field)
  }

  fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
    LeafReader::get_sum_total_term_freq(self, field)
  }

  fn index_base(&self) -> &IndexReaderBase {
    &self.index_base
  }
}
impl<D> LeafReader for SegmentReader<D>
where
  D: Directory,
{
  type CacheHelper = SegmentCoreReadersCacheHelperImpl;

  fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>> {
    Ok(Option::from(self.core.get_cache_helper()))
  }

  type Terms = <<Self as CodecReader>::FieldsProducer as Fields>::Terms;

  fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
    CodecReader::terms(self, field)
  }

  type NumericDocValues =
    <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::NumericDocValues;

  fn get_numeric_doc_values(&self, field: &str) -> Result<Option<Self::NumericDocValues>> {
    CodecReader::get_numeric_doc_values(self, field)
  }

  type BinaryDocValues =
    <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::BinaryDocValues;

  fn get_binary_doc_values(&self, field: &str) -> Result<Option<Self::BinaryDocValues>> {
    CodecReader::get_binary_doc_values(self, field)
  }

  type SortedDocValues =
    <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::SortedDocValues;

  fn get_sorted_doc_values(&self, field: &str) -> Result<Option<Self::SortedDocValues>> {
    CodecReader::get_sorted_doc_values(self, field)
  }

  type SortedNumericDocValues =
    <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::SortedNumericDocValues;

  fn get_sorted_numeric_doc_values(
    &self,
    field: &str,
  ) -> Result<Option<Self::SortedNumericDocValues>> {
    CodecReader::get_sorted_numeric_doc_values(self, field)
  }

  type SortedSetDocValues =
    <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::SortedSetDocValues;

  fn get_sorted_set_doc_values(&self, field: &str) -> Result<Option<Self::SortedSetDocValues>> {
    CodecReader::get_sorted_set_doc_values(self, field)
  }

  type NormNumericDocValues =
    <<Self as CodecReader>::NormsProducer as NormsProducer>::NumericDocValues;

  fn get_norm_values(&self, field: &str) -> Result<Option<Self::NormNumericDocValues>> {
    CodecReader::get_norm_values(self, field)
  }

  type DocValuesSkipper =
    <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::DocValuesSkipper;

  fn get_doc_values_skipper(&self, field: &str) -> Result<Option<Self::DocValuesSkipper>> {
    CodecReader::get_doc_values_skipper(self, field)
  }

  type FloatVectorValues =
    <<Self as CodecReader>::KnnVectorsReader as KnnVectorsReader>::FloatVectorValues;

  fn get_float_vector_values(&self, field: &str) -> Result<Option<Self::FloatVectorValues>> {
    CodecReader::get_float_vector_values(self, field)
  }

  type ByteVectorValues =
    <<Self as CodecReader>::KnnVectorsReader as KnnVectorsReader>::ByteVectorValues;

  fn get_byte_vector_values(&self, field: &str) -> Result<Option<Self::ByteVectorValues>> {
    CodecReader::get_byte_vector_values(self, field)
  }

  fn search_nearest_vectors_f32<B, K>(
    &self,
    field: &str,
    target: Vec<f32>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    CodecReader::search_nearest_vectors_f32(self, field, target, knn_collector, accept_docs)
  }

  fn search_nearest_vectors_u8<B, K>(
    &self,
    field: &str,
    target: Vec<u8>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    CodecReader::search_nearest_vectors_u8(self, field, target, knn_collector, accept_docs)
  }

  fn get_field_infos(&self) -> Result<Arc<FieldInfos>> {
    Ok(self.field_infos.clone())
  }

  type Bits = DocBits;

  fn get_live_docs(&self) -> Result<Option<Self::Bits>> {
    match &self.live_docs {
      Some(DocBits::A(a)) => Ok(Some(DocBits::A(Arc::clone(a)))),
      Some(DocBits::B(b)) => match b {
        BitsEnum2::A(a) => Ok(Some(DocBits::B(BitsEnum2::A(Arc::clone(a))))),
        BitsEnum2::B(_) => Err(LuceneError::illegal_state(
          "live docs should be FixedBitSet",
        )),
      },
      None => Ok(None),
    }
  }

  type PointValues = <<Self as CodecReader>::PointsReader as PointsReader>::PointValuesType;

  fn get_point_values(&self, field: &str) -> Result<Option<Self::PointValues>> {
    CodecReader::get_point_values(self, field)
  }

  fn check_integrity(&self) -> Result<()> {
    CodecReader::default_check_integrity(self)?;
    if let Some(dv) = self.core.cfs_reader.as_ref() {
      dv.check_integrity()?;
    }
    Ok(())
  }

  fn get_metadata(&self) -> Result<&LeafMetaData> {
    Ok(&self.meta_data)
  }
}
impl<D> CodecReader for SegmentReader<D>
where
  D: Directory,
{
  type StoredFieldsReader = DefaultStoredFieldsReader<D::IndexInput>;
  type TermVectorsReader = DefaultTermVectorsReader<D::IndexInput>;
  type NormsProducer = Arc<DefaultNormProducer<D::IndexInput>>;
  type DocValuesProducer = Arc<DocValuesProducers<D>>;
  type FieldsProducer = Arc<DefaultFieldsProducer<D::IndexInput>>;
  type PointsReader = Arc<DefaultPointsReader<D::IndexInput>>;
  type KnnVectorsReader = Arc<DefaultKnnVectorsReader<D::IndexInput>>;

  fn get_fields_reader(&self) -> Result<Option<Self::StoredFieldsReader>> {
    self.ensure_open()?;
    Ok(Some(self.core.fields_reader_orig.try_clone()?))
  }

  fn get_term_vectors_reader(&self) -> Result<Option<Self::TermVectorsReader>> {
    self.ensure_open()?;
    match &self.core.term_vectors_reader_orig {
      Some(reader) => Ok(Some(reader.try_clone()?)),
      None => Ok(None),
    }
  }

  fn get_norms_reader(&self) -> Result<Option<Self::NormsProducer>> {
    self.ensure_open()?;
    Ok(self.core.norms_producer.clone())
  }

  fn get_doc_values_reader(&self) -> Result<Option<Self::DocValuesProducer>> {
    self.ensure_open()?;
    Ok(self.doc_values_producer.clone())
  }

  fn get_postings_reader(&self) -> Result<Option<Self::FieldsProducer>> {
    self.ensure_open()?;
    Ok(self.core.fields.clone())
  }

  fn get_points_reader(&self) -> Result<Option<Self::PointsReader>> {
    self.ensure_open()?;
    Ok(self.core.points_reader.clone())
  }

  fn get_vector_reader(&self) -> Result<Option<Self::KnnVectorsReader>> {
    Ok(self.core.knn_vectors_reader.clone())
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
pub type DefaultLeafReader<D> = Arc<SegmentReader<D>>;

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

use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use parking_lot::Mutex;
use rand::Rng;

use crate::core::codecs::doc_values_producer::DocValuesProducer;
use crate::core::codecs::fields_producer::FieldsProducer;
use crate::core::codecs::hnsw::hnsw_graph_provider::HnswGraphProvider;
use crate::core::codecs::knn_vectors_reader::KnnVectorsReader;
use crate::core::codecs::norms_producer::NormsProducer;
use crate::core::codecs::points_reader::PointsReader;
use crate::core::codecs::stored_fields_reader::StoredFieldsReader;
use crate::core::codecs::stored_fields_writer::StoredFieldsWriter;
use crate::core::codecs::term_vectors_reader::{DefaultTermVectorsReader, TermVectorsReader};
use crate::core::document::document::Document;
use crate::core::index::codec_reader::{CodecReader, StoredFieldsType, TermVectorsType};
use crate::core::index::field_info::FieldInfo;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::fields::Fields;
use crate::core::index::index_reader::{IndexReader, IndexReaderBase, LeafReaderContextKind};
use crate::core::index::leaf_metadata::LeafMetaData;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::index::stored_field_visitor::StoredFieldVisitor;
use crate::core::index::stored_fields::{RawStoredFieldsReader, StoredFields};
use crate::core::index::term::Term;
use crate::core::index::term_vectors::{RawTermVectors, TermVectors};
use crate::core::search::knn_collector::KnnCollector;
use crate::core::store::directory::Directory;
use crate::core::util::bits::Bits;
use crate::core::util::clone::TryClone;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::iterator::{IteratorExt, VecIter, VecIteratorExt};
use crate::core::util::quantization::scalar_quantizer::ScalarQuantizer;
use crate::test_framework::core::index::mismatched_leaf_reader::{
  MismatchedVisitor, shuffle_infos,
};

type BaseStoredFieldsReader<D> = <DefaultLeafReader<D> as CodecReader>::StoredFieldsReader;
type BaseTermVectorsReader<D> = <DefaultLeafReader<D> as CodecReader>::TermVectorsReader;
type BaseNormsProducer<D> = <DefaultLeafReader<D> as CodecReader>::NormsProducer;
type BaseDocValuesProducer<D> = <DefaultLeafReader<D> as CodecReader>::DocValuesProducer;
type BaseFieldsProducer<D> = <DefaultLeafReader<D> as CodecReader>::FieldsProducer;
type BasePointsReader<D> = <DefaultLeafReader<D> as CodecReader>::PointsReader;
type BaseKnnVectorsReader<D> = <DefaultLeafReader<D> as CodecReader>::KnnVectorsReader;

#[derive(Clone, Copy, Eq, PartialEq)]
enum ReaderMode {
  Default,
  Slow,
  Mismatched,
}

struct SlowReaderState<D>
where
  D: Directory,
{
  fields: Option<BaseFieldsProducer<D>>,
  norms: Option<BaseNormsProducer<D>>,
  doc_values: Option<BaseDocValuesProducer<D>>,
  store: Option<Arc<Mutex<BaseStoredFieldsReader<D>>>>,
  vectors: Option<BaseTermVectorsReader<D>>,
}

impl<D> SlowReaderState<D>
where
  D: Directory,
{
  fn new(reader: &DefaultLeafReader<D>) -> Result<Self> {
    let fields = match reader.get_postings_reader()? {
      Some(fields) => {
        let merge_instance = fields.get_merge_instance()?;
        Some(merge_instance.unwrap_or(fields))
      },
      None => None,
    };

    let norms = match reader.get_norms_reader()? {
      Some(norms) => {
        let merge_instance = norms.get_merge_instance()?;
        Some(merge_instance.unwrap_or(norms))
      },
      None => None,
    };

    let doc_values = match reader.get_doc_values_reader()? {
      Some(doc_values) => {
        let merge_instance = doc_values.get_merge_instance()?;
        Some(merge_instance.unwrap_or(doc_values))
      },
      None => None,
    };

    let store = match reader.get_fields_reader()? {
      Some(store) => {
        let merge_instance = store.get_merge_instance()?;
        Some(Arc::new(Mutex::new(merge_instance.unwrap_or(store))))
      },
      None => None,
    };

    let vectors = match reader.get_term_vectors_reader()? {
      Some(vectors) => {
        let merge_instance = vectors.get_merge_instance()?;
        Some(merge_instance.unwrap_or(vectors))
      },
      None => None,
    };

    Ok(Self {
      fields,
      norms,
      doc_values,
      store,
      vectors,
    })
  }
}

enum ReaderHook<D>
where
  D: Directory,
{
  Default,
  Slow(Arc<SlowReaderState<D>>),
  Mismatched(Arc<FieldInfos>),
}

impl<D> Clone for ReaderHook<D>
where
  D: Directory,
{
  fn clone(&self) -> Self {
    match self {
      Self::Default => Self::Default,
      Self::Slow(state) => Self::Slow(state.clone()),
      Self::Mismatched(shuffled) => Self::Mismatched(shuffled.clone()),
    }
  }
}

pub(crate) struct MockRandomWrappedReader<D>
where
  D: Directory,
{
  reader: DefaultLeafReader<D>,
  hook: ReaderHook<D>,
  slow_index_base: IndexReaderBase,
  is_wrapped: bool,
}

impl<D> MockRandomWrappedReader<D>
where
  D: Directory,
{
  pub(crate) fn unchanged(reader: DefaultLeafReader<D>) -> Self {
    Self {
      reader,
      hook: ReaderHook::Default,
      slow_index_base: IndexReaderBase::new(),
      is_wrapped: false,
    }
  }

  pub(crate) fn unchanged_with_status(reader: DefaultLeafReader<D>, is_wrapped: bool) -> Self {
    Self {
      reader,
      hook: ReaderHook::Default,
      slow_index_base: IndexReaderBase::new(),
      is_wrapped,
    }
  }

  pub(crate) fn slow(reader: DefaultLeafReader<D>) -> Result<Self> {
    reader.check_integrity()?;
    let state = Arc::new(SlowReaderState::new(&reader)?);
    Ok(Self {
      reader,
      hook: ReaderHook::Slow(state),
      slow_index_base: IndexReaderBase::new(),
      is_wrapped: true,
    })
  }

  pub(crate) fn mismatched<R>(reader: DefaultLeafReader<D>, random: &mut R) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    let shuffled = shuffle_infos(reader.get_field_infos()?.as_ref(), random)?;
    Ok(Self {
      reader,
      hook: ReaderHook::Mismatched(Arc::new(shuffled)),
      slow_index_base: IndexReaderBase::new(),
      is_wrapped: true,
    })
  }

  pub(crate) fn is_wrapped(&self) -> bool {
    self.is_wrapped
  }

  fn mode(&self) -> ReaderMode {
    match self.hook {
      ReaderHook::Default => ReaderMode::Default,
      ReaderHook::Slow(_) => ReaderMode::Slow,
      ReaderHook::Mismatched(_) => ReaderMode::Mismatched,
    }
  }

  fn shuffled_field_infos(&self) -> Option<Arc<FieldInfos>> {
    match &self.hook {
      ReaderHook::Mismatched(shuffled) => Some(shuffled.clone()),
      _ => None,
    }
  }
}

impl<D> Clone for MockRandomWrappedReader<D>
where
  D: Directory,
{
  fn clone(&self) -> Self {
    Self {
      reader: self.reader.clone(),
      hook: self.hook.clone(),
      slow_index_base: IndexReaderBase::new(),
      is_wrapped: self.is_wrapped,
    }
  }
}

impl<D> Display for MockRandomWrappedReader<D>
where
  D: Directory,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.hook {
      ReaderHook::Default => Display::fmt(&self.reader, f),
      ReaderHook::Slow(_) => write!(
        f,
        "SlowCodecReaderWrapper(MergeReaderWrapper({}))",
        self.reader
      ),
      ReaderHook::Mismatched(_) => write!(f, "MismatchedCodecReader({})", self.reader),
    }
  }
}

enum StoredFieldsReaderInner<D>
where
  D: Directory,
{
  Default(BaseStoredFieldsReader<D>),
  Slow(Arc<Mutex<BaseStoredFieldsReader<D>>>),
  Mismatched {
    inner: BaseStoredFieldsReader<D>,
    shuffled: Arc<FieldInfos>,
  },
}

pub(crate) struct MockRandomStoredFieldsReader<D>
where
  D: Directory,
{
  inner: StoredFieldsReaderInner<D>,
}

impl<D> MockRandomStoredFieldsReader<D>
where
  D: Directory,
{
  fn default(inner: BaseStoredFieldsReader<D>) -> Self {
    Self {
      inner: StoredFieldsReaderInner::Default(inner),
    }
  }

  fn slow(inner: Arc<Mutex<BaseStoredFieldsReader<D>>>) -> Self {
    Self {
      inner: StoredFieldsReaderInner::Slow(inner),
    }
  }

  fn mismatched(inner: BaseStoredFieldsReader<D>, shuffled: Arc<FieldInfos>) -> Self {
    Self {
      inner: StoredFieldsReaderInner::Mismatched { inner, shuffled },
    }
  }
}

impl<D> RawStoredFieldsReader for MockRandomStoredFieldsReader<D>
where
  D: Directory,
{
  type IndexInput = D::IndexInput;

  fn raw_stored_fields_mut(
    &mut self,
  ) -> Result<
    &mut crate::core::codecs::stored_fields_reader::DefaultStoredFieldsReader<Self::IndexInput>,
  > {
    match &mut self.inner {
      StoredFieldsReaderInner::Default(inner) => inner.raw_stored_fields_mut(),
      StoredFieldsReaderInner::Slow(_) | StoredFieldsReaderInner::Mismatched { .. } => Err(
        LuceneError::unsupported_operation("raw stored fields are not available"),
      ),
    }
  }

  fn raw_stored_fields(
    &self,
  ) -> Result<&crate::core::codecs::stored_fields_reader::DefaultStoredFieldsReader<Self::IndexInput>>
  {
    match &self.inner {
      StoredFieldsReaderInner::Default(inner) => inner.raw_stored_fields(),
      StoredFieldsReaderInner::Slow(_) | StoredFieldsReaderInner::Mismatched { .. } => Err(
        LuceneError::unsupported_operation("raw stored fields are not available"),
      ),
    }
  }
}

impl<D> StoredFields for MockRandomStoredFieldsReader<D>
where
  D: Directory,
{
  fn prefetch(&mut self, doc_id: i32) -> Result<()> {
    match &mut self.inner {
      StoredFieldsReaderInner::Default(inner) => inner.prefetch(doc_id),
      StoredFieldsReaderInner::Slow(inner) => inner.lock().prefetch(doc_id),
      StoredFieldsReaderInner::Mismatched { .. } => Ok(()),
    }
  }

  fn document_with_visitor<S>(
    &mut self,
    doc_id: i32,
    visitor: &mut impl StoredFieldVisitor,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    match &mut self.inner {
      StoredFieldsReaderInner::Default(inner) => {
        inner.document_with_visitor(doc_id, visitor, writer)
      },
      StoredFieldsReaderInner::Slow(inner) => {
        inner.lock().document_with_visitor(doc_id, visitor, writer)
      },
      StoredFieldsReaderInner::Mismatched { inner, shuffled } => {
        let mut mismatched_visitor = MismatchedVisitor::new(visitor, shuffled.clone());
        inner.document_with_visitor(doc_id, &mut mismatched_visitor, writer)
      },
    }
  }

  fn document(&mut self, doc_id: i32) -> Result<Document> {
    match &mut self.inner {
      StoredFieldsReaderInner::Default(inner) => inner.document(doc_id),
      StoredFieldsReaderInner::Slow(inner) => inner.lock().document(doc_id),
      StoredFieldsReaderInner::Mismatched { .. } => {
        let mut visitor =
          crate::core::document::document_stored_field_visitor::DocumentStoredFieldVisitor::new();
        self.document_with_visitor(
          doc_id,
          &mut visitor,
          Some(&mut crate::core::codecs::dummy::stored_fields_writer::DummyStoredFieldsWriter),
        )?;
        Ok(visitor.get_document_owner())
      },
    }
  }

  fn document_with_fields(
    &mut self,
    doc_id: i32,
    fields_to_load: &HashSet<String>,
  ) -> Result<Document> {
    match &mut self.inner {
      StoredFieldsReaderInner::Default(inner) => inner.document_with_fields(doc_id, fields_to_load),
      StoredFieldsReaderInner::Slow(inner) => {
        inner.lock().document_with_fields(doc_id, fields_to_load)
      },
      StoredFieldsReaderInner::Mismatched { .. } => {
        let mut visitor =
          crate::core::document::document_stored_field_visitor::DocumentStoredFieldVisitor::with_fields(
            fields_to_load,
          );
        self.document_with_visitor(
          doc_id,
          &mut visitor,
          Some(&mut crate::core::codecs::dummy::stored_fields_writer::DummyStoredFieldsWriter),
        )?;
        Ok(visitor.get_document_owner())
      },
    }
  }
}

impl<D> TryClone for MockRandomStoredFieldsReader<D>
where
  D: Directory,
{
  fn try_clone(&self) -> Result<Self> {
    let inner = match &self.inner {
      StoredFieldsReaderInner::Default(inner) => {
        StoredFieldsReaderInner::Default(inner.try_clone()?)
      },
      StoredFieldsReaderInner::Slow(inner) => StoredFieldsReaderInner::Slow(inner.clone()),
      StoredFieldsReaderInner::Mismatched { inner, shuffled } => {
        StoredFieldsReaderInner::Mismatched {
          inner: inner.try_clone()?,
          shuffled: shuffled.clone(),
        }
      },
    };
    Ok(Self { inner })
  }
}

impl<D> CloseableRef for MockRandomStoredFieldsReader<D>
where
  D: Directory,
{
  fn close(&self) -> Result<()> {
    match &self.inner {
      StoredFieldsReaderInner::Default(inner)
      | StoredFieldsReaderInner::Mismatched { inner, .. } => inner.close(),
      StoredFieldsReaderInner::Slow(_) => Ok(()),
    }
  }
}

impl<D> StoredFieldsReader for MockRandomStoredFieldsReader<D>
where
  D: Directory,
{
  fn check_integrity(&self) -> Result<()> {
    match &self.inner {
      StoredFieldsReaderInner::Default(inner)
      | StoredFieldsReaderInner::Mismatched { inner, .. } => inner.check_integrity(),
      StoredFieldsReaderInner::Slow(_) => Ok(()),
    }
  }

  fn get_merge_instance(&self) -> Result<Option<Self>> {
    match &self.inner {
      StoredFieldsReaderInner::Default(inner) => Ok(
        inner
          .get_merge_instance()?
          .map(MockRandomStoredFieldsReader::default),
      ),
      StoredFieldsReaderInner::Slow(_) | StoredFieldsReaderInner::Mismatched { .. } => Ok(None),
    }
  }
}

enum TermVectorsReaderInner<D>
where
  D: Directory,
{
  Direct(BaseTermVectorsReader<D>),
  Slow(Option<BaseTermVectorsReader<D>>),
}

pub(crate) struct MockRandomTermVectorsReader<D>
where
  D: Directory,
{
  inner: TermVectorsReaderInner<D>,
}

impl<D> RawTermVectors for MockRandomTermVectorsReader<D>
where
  D: Directory,
{
  type IndexInput = D::IndexInput;

  fn raw_term_vectors_mut(&mut self) -> Result<&mut DefaultTermVectorsReader<Self::IndexInput>> {
    match &mut self.inner {
      TermVectorsReaderInner::Direct(inner) => inner.raw_term_vectors_mut(),
      TermVectorsReaderInner::Slow(_) => Err(LuceneError::unsupported_operation(
        "raw term vectors are not available for SlowCodecReaderWrapper",
      )),
    }
  }

  fn raw_term_vectors(&self) -> Result<&DefaultTermVectorsReader<Self::IndexInput>> {
    match &self.inner {
      TermVectorsReaderInner::Direct(inner) => inner.raw_term_vectors(),
      TermVectorsReaderInner::Slow(_) => Err(LuceneError::unsupported_operation(
        "raw term vectors are not available for SlowCodecReaderWrapper",
      )),
    }
  }
}

impl<D> TermVectors for MockRandomTermVectorsReader<D>
where
  D: Directory,
{
  type Fields = <BaseTermVectorsReader<D> as TermVectors>::Fields;
  type Terms = <BaseTermVectorsReader<D> as TermVectors>::Terms;

  fn prefetch(&mut self, doc_id: i32) -> Result<()> {
    match &mut self.inner {
      TermVectorsReaderInner::Direct(inner) => inner.prefetch(doc_id),
      TermVectorsReaderInner::Slow(Some(inner)) => inner.prefetch(doc_id),
      TermVectorsReaderInner::Slow(None) => Ok(()),
    }
  }

  fn get(&mut self, doc: i32) -> Result<Option<Self::Fields>> {
    match &mut self.inner {
      TermVectorsReaderInner::Direct(inner) => inner.get(doc),
      TermVectorsReaderInner::Slow(Some(inner)) => inner.get(doc),
      TermVectorsReaderInner::Slow(None) => Ok(None),
    }
  }

  fn get_field_terms(
    &mut self,
    doc: i32,
    field: &str,
  ) -> Result<Option<<Self::Fields as Fields>::Terms>> {
    match &mut self.inner {
      TermVectorsReaderInner::Direct(inner) => inner.get_field_terms(doc, field),
      TermVectorsReaderInner::Slow(_) => self.default_get_field_terms(doc, field),
    }
  }
}

impl<D> TryClone for MockRandomTermVectorsReader<D>
where
  D: Directory,
{
  fn try_clone(&self) -> Result<Self> {
    let inner = match &self.inner {
      TermVectorsReaderInner::Direct(inner) => TermVectorsReaderInner::Direct(inner.try_clone()?),
      TermVectorsReaderInner::Slow(inner) => {
        TermVectorsReaderInner::Slow(inner.as_ref().map(TryClone::try_clone).transpose()?)
      },
    };
    Ok(Self { inner })
  }
}

impl<D> CloseableRef for MockRandomTermVectorsReader<D>
where
  D: Directory,
{
  fn close(&self) -> Result<()> {
    match &self.inner {
      TermVectorsReaderInner::Direct(inner) => inner.close(),
      TermVectorsReaderInner::Slow(_) => Ok(()),
    }
  }
}

impl<D> TermVectorsReader for MockRandomTermVectorsReader<D>
where
  D: Directory,
{
  fn check_integrity(&self) -> Result<()> {
    match &self.inner {
      TermVectorsReaderInner::Direct(inner) => inner.check_integrity(),
      TermVectorsReaderInner::Slow(_) => Ok(()),
    }
  }

  fn get_merge_instance(&self) -> Result<Option<Self>> {
    match &self.inner {
      TermVectorsReaderInner::Direct(inner) => Ok(inner.get_merge_instance()?.map(|inner| Self {
        inner: TermVectorsReaderInner::Direct(inner),
      })),
      TermVectorsReaderInner::Slow(_) => Ok(None),
    }
  }
}

pub(crate) struct MockRandomNormsProducer<D>
where
  D: Directory,
{
  inner: BaseNormsProducer<D>,
  mode: ReaderMode,
  shuffled: Option<Arc<FieldInfos>>,
  original: Option<Arc<FieldInfos>>,
}

impl<D> MockRandomNormsProducer<D>
where
  D: Directory,
{
  fn remap_field_info(&self, field: &Arc<FieldInfo>) -> Result<Arc<FieldInfo>> {
    let Some(shuffled) = &self.shuffled else {
      return Ok(field.clone());
    };
    let shuffled_field = shuffled.field_info_by_name(&field.name)?.ok_or_else(|| {
      LuceneError::illegal_state(format!("missing shuffled field info for {}", field.name))
    })?;
    assert_eq!(shuffled_field.number, field.number);
    self
      .original
      .as_ref()
      .expect("original field infos")
      .field_info_by_name(&field.name)?
      .ok_or_else(|| {
        LuceneError::illegal_state(format!("missing original field info for {}", field.name))
      })
  }
}

impl<D> CloseableRef for MockRandomNormsProducer<D>
where
  D: Directory,
{
  fn close(&self) -> Result<()> {
    match self.mode {
      ReaderMode::Slow => Ok(()),
      ReaderMode::Default | ReaderMode::Mismatched => self.inner.close(),
    }
  }
}

impl<D> NormsProducer for MockRandomNormsProducer<D>
where
  D: Directory,
{
  type NumericDocValues = <BaseNormsProducer<D> as NormsProducer>::NumericDocValues;

  fn get_norms(&self, field: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
    self.inner.get_norms(&self.remap_field_info(field)?)
  }

  fn check_integrity(&self) -> Result<()> {
    match self.mode {
      ReaderMode::Slow => Ok(()),
      ReaderMode::Default | ReaderMode::Mismatched => self.inner.check_integrity(),
    }
  }

  fn get_merge_instance(&self) -> Result<Option<Self>> {
    if self.mode != ReaderMode::Default {
      return Ok(None);
    }
    Ok(self.inner.get_merge_instance()?.map(|inner| Self {
      inner,
      mode: self.mode,
      shuffled: None,
      original: None,
    }))
  }
}

pub(crate) struct MockRandomDocValuesProducer<D>
where
  D: Directory,
{
  inner: BaseDocValuesProducer<D>,
  mode: ReaderMode,
  shuffled: Option<Arc<FieldInfos>>,
  original: Option<Arc<FieldInfos>>,
}

impl<D> MockRandomDocValuesProducer<D>
where
  D: Directory,
{
  fn remap_field_info(&self, field: &Arc<FieldInfo>) -> Result<Arc<FieldInfo>> {
    let Some(shuffled) = &self.shuffled else {
      return Ok(field.clone());
    };
    let shuffled_field = shuffled.field_info_by_name(&field.name)?.ok_or_else(|| {
      LuceneError::illegal_state(format!("missing shuffled field info for {}", field.name))
    })?;
    assert_eq!(shuffled_field.number, field.number);
    self
      .original
      .as_ref()
      .expect("original field infos")
      .field_info_by_name(&field.name)?
      .ok_or_else(|| {
        LuceneError::illegal_state(format!("missing original field info for {}", field.name))
      })
  }
}

impl<D> CloseableRef for MockRandomDocValuesProducer<D>
where
  D: Directory,
{
  fn close(&self) -> Result<()> {
    match self.mode {
      ReaderMode::Slow => Ok(()),
      ReaderMode::Default | ReaderMode::Mismatched => self.inner.close(),
    }
  }
}

impl<D> DocValuesProducer for MockRandomDocValuesProducer<D>
where
  D: Directory,
{
  type NumericDocValues = <BaseDocValuesProducer<D> as DocValuesProducer>::NumericDocValues;
  type BinaryDocValues = <BaseDocValuesProducer<D> as DocValuesProducer>::BinaryDocValues;
  type SortedDocValues = <BaseDocValuesProducer<D> as DocValuesProducer>::SortedDocValues;
  type SortedNumericDocValues =
    <BaseDocValuesProducer<D> as DocValuesProducer>::SortedNumericDocValues;
  type SortedSetDocValues = <BaseDocValuesProducer<D> as DocValuesProducer>::SortedSetDocValues;
  type DocValuesSkipper = <BaseDocValuesProducer<D> as DocValuesProducer>::DocValuesSkipper;

  fn get_numeric(&self, field: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
    self.inner.get_numeric(&self.remap_field_info(field)?)
  }

  fn get_binary(&self, field: &Arc<FieldInfo>) -> Result<Self::BinaryDocValues> {
    self.inner.get_binary(&self.remap_field_info(field)?)
  }

  fn get_sorted(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedDocValues> {
    self.inner.get_sorted(&self.remap_field_info(field)?)
  }

  fn get_sorted_numeric(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedNumericDocValues> {
    self
      .inner
      .get_sorted_numeric(&self.remap_field_info(field)?)
  }

  fn get_sorted_set(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedSetDocValues> {
    self.inner.get_sorted_set(&self.remap_field_info(field)?)
  }

  fn get_skipper(&self, field: &Arc<FieldInfo>) -> Result<Option<Self::DocValuesSkipper>> {
    self.inner.get_skipper(&self.remap_field_info(field)?)
  }

  fn check_integrity(&self) -> Result<()> {
    match self.mode {
      ReaderMode::Slow => Ok(()),
      ReaderMode::Default | ReaderMode::Mismatched => self.inner.check_integrity(),
    }
  }

  fn get_merge_instance(&self) -> Result<Option<Self>> {
    if self.mode != ReaderMode::Default {
      return Ok(None);
    }
    Ok(self.inner.get_merge_instance()?.map(|inner| Self {
      inner,
      mode: self.mode,
      shuffled: None,
      original: None,
    }))
  }
}

pub(crate) struct MockRandomFieldsProducer<D>
where
  D: Directory,
{
  state: Option<Arc<SlowReaderState<D>>>,
  inner: Option<BaseFieldsProducer<D>>,
  mode: ReaderMode,
  indexed_fields: Vec<String>,
}

impl<D> Fields for MockRandomFieldsProducer<D>
where
  D: Directory,
{
  type FieldIter<'a>
    = VecIter<'a, String>
  where
    D: 'a;
  type Terms = <BaseFieldsProducer<D> as Fields>::Terms;

  fn iterator(&self) -> Result<Self::FieldIter<'_>> {
    Ok(self.indexed_fields.iter_ext())
  }

  fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
    match &self.state {
      Some(state) => match &state.fields {
        Some(fields) => fields.terms(field),
        None => Ok(None),
      },
      None => self
        .inner
        .as_ref()
        .ok_or_else(|| LuceneError::illegal_state("postings reader is None"))?
        .terms(field),
    }
  }

  fn size(&self) -> Result<i32> {
    Ok(self.indexed_fields.len() as i32)
  }
}

impl<D> CloseableRef for MockRandomFieldsProducer<D>
where
  D: Directory,
{
  fn close(&self) -> Result<()> {
    match self.mode {
      ReaderMode::Slow => Ok(()),
      ReaderMode::Default | ReaderMode::Mismatched => self
        .inner
        .as_ref()
        .ok_or_else(|| LuceneError::illegal_state("postings reader is None"))?
        .close(),
    }
  }
}

impl<D> FieldsProducer for MockRandomFieldsProducer<D>
where
  D: Directory,
{
  fn check_integrity(&self) -> Result<()> {
    match self.mode {
      ReaderMode::Slow => Ok(()),
      ReaderMode::Default | ReaderMode::Mismatched => self
        .inner
        .as_ref()
        .ok_or_else(|| LuceneError::illegal_state("postings reader is None"))?
        .check_integrity(),
    }
  }

  fn get_merge_instance(&self) -> Result<Option<Self>> {
    if self.mode == ReaderMode::Slow {
      return Ok(None);
    }
    let Some(inner) = &self.inner else {
      return Ok(None);
    };
    Ok(inner.get_merge_instance()?.map(|inner| Self {
      state: None,
      inner: Some(inner),
      mode: self.mode,
      indexed_fields: self.indexed_fields.clone(),
    }))
  }
}

enum PointsReaderInner<D>
where
  D: Directory,
{
  Direct(BasePointsReader<D>),
  Slow(DefaultLeafReader<D>),
}

pub(crate) struct MockRandomPointsReader<D>
where
  D: Directory,
{
  inner: PointsReaderInner<D>,
}

impl<D> CloseableRef for MockRandomPointsReader<D>
where
  D: Directory,
{
  fn close(&self) -> Result<()> {
    match &self.inner {
      PointsReaderInner::Direct(inner) => inner.close(),
      PointsReaderInner::Slow(_) => Ok(()),
    }
  }
}

impl<D> PointsReader for MockRandomPointsReader<D>
where
  D: Directory,
{
  type PointValuesType = <BasePointsReader<D> as PointsReader>::PointValuesType;

  fn check_integrity(&self) -> Result<()> {
    match &self.inner {
      PointsReaderInner::Direct(inner) => inner.check_integrity(),
      PointsReaderInner::Slow(_) => Ok(()),
    }
  }

  fn get_values(&self, field: &str) -> Result<Option<Self::PointValuesType>> {
    match &self.inner {
      PointsReaderInner::Direct(inner) => inner.get_values(field),
      PointsReaderInner::Slow(reader) => LeafReader::get_point_values(reader, field),
    }
  }

  fn get_merge_instance(&self) -> Result<Option<Self>> {
    match &self.inner {
      PointsReaderInner::Direct(inner) => Ok(inner.get_merge_instance()?.map(|inner| Self {
        inner: PointsReaderInner::Direct(inner),
      })),
      PointsReaderInner::Slow(_) => Ok(None),
    }
  }
}

enum KnnVectorsReaderInner<D>
where
  D: Directory,
{
  Direct(BaseKnnVectorsReader<D>),
  Slow(DefaultLeafReader<D>),
}

pub(crate) struct MockRandomKnnVectorsReader<D>
where
  D: Directory,
{
  inner: KnnVectorsReaderInner<D>,
}

impl<D> CloseableRef for MockRandomKnnVectorsReader<D>
where
  D: Directory,
{
  fn close(&self) -> Result<()> {
    match &self.inner {
      KnnVectorsReaderInner::Direct(inner) => inner.close(),
      KnnVectorsReaderInner::Slow(_) => Ok(()),
    }
  }
}

impl<D> HnswGraphProvider for MockRandomKnnVectorsReader<D>
where
  D: Directory,
{
  type HnswGraph = <BaseKnnVectorsReader<D> as HnswGraphProvider>::HnswGraph;

  fn is_hnsw_graph_provider(&self, field: &str) -> bool {
    match &self.inner {
      KnnVectorsReaderInner::Direct(inner) => inner.is_hnsw_graph_provider(field),
      KnnVectorsReaderInner::Slow(_) => false,
    }
  }

  fn get_graph(&self, field: &str) -> Result<Self::HnswGraph> {
    match &self.inner {
      KnnVectorsReaderInner::Direct(inner) => inner.get_graph(field),
      KnnVectorsReaderInner::Slow(_) => Err(LuceneError::unsupported_operation("")),
    }
  }
}

impl<D> KnnVectorsReader for MockRandomKnnVectorsReader<D>
where
  D: Directory,
{
  type FloatVectorValues = <BaseKnnVectorsReader<D> as KnnVectorsReader>::FloatVectorValues;
  type ByteVectorValues = <BaseKnnVectorsReader<D> as KnnVectorsReader>::ByteVectorValues;

  fn check_integrity(&self) -> Result<()> {
    match &self.inner {
      KnnVectorsReaderInner::Direct(inner) => inner.check_integrity(),
      KnnVectorsReaderInner::Slow(_) => Ok(()),
    }
  }

  fn get_float_vector_values(&self, field: &str) -> Result<Self::FloatVectorValues> {
    match &self.inner {
      KnnVectorsReaderInner::Direct(inner) => inner.get_float_vector_values(field),
      KnnVectorsReaderInner::Slow(reader) => LeafReader::get_float_vector_values(reader, field)?
        .ok_or_else(|| {
          LuceneError::illegal_state(
            "FloatVectorValues from leaf reader does not support get_float_vector_values ",
          )
        }),
    }
  }

  fn get_byte_vector_values(&self, field: &str) -> Result<Self::ByteVectorValues> {
    match &self.inner {
      KnnVectorsReaderInner::Direct(inner) => inner.get_byte_vector_values(field),
      KnnVectorsReaderInner::Slow(reader) => LeafReader::get_byte_vector_values(reader, field)?
        .ok_or_else(|| {
          LuceneError::illegal_state(
            "ByteVectorValues from leaf reader does not support get_float_vector_values ",
          )
        }),
    }
  }

  fn get_quantization_state(&self, field: &str) -> Result<Option<ScalarQuantizer>> {
    match &self.inner {
      KnnVectorsReaderInner::Direct(inner) => inner.get_quantization_state(field),
      KnnVectorsReaderInner::Slow(_) => Ok(None),
    }
  }

  fn is_flat_vectors_reader(&self, field: &str) -> bool {
    match &self.inner {
      KnnVectorsReaderInner::Direct(inner) => inner.is_flat_vectors_reader(field),
      KnnVectorsReaderInner::Slow(_) => false,
    }
  }

  fn search_f32<B, K>(
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
    match &self.inner {
      KnnVectorsReaderInner::Direct(inner) => {
        inner.search_f32(field, target, knn_collector, accept_docs)
      },
      KnnVectorsReaderInner::Slow(reader) => {
        LeafReader::search_nearest_vectors_f32(reader, field, target, knn_collector, accept_docs)
      },
    }
  }

  fn search_u8<B, K>(
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
    match &self.inner {
      KnnVectorsReaderInner::Direct(inner) => {
        inner.search_u8(field, target, knn_collector, accept_docs)
      },
      KnnVectorsReaderInner::Slow(reader) => {
        LeafReader::search_nearest_vectors_u8(reader, field, target, knn_collector, accept_docs)
      },
    }
  }

  fn get_merge_instance(&self) -> Result<Option<Self>> {
    match &self.inner {
      KnnVectorsReaderInner::Direct(inner) => Ok(inner.get_merge_instance()?.map(|inner| Self {
        inner: KnnVectorsReaderInner::Direct(inner),
      })),
      KnnVectorsReaderInner::Slow(_) => Ok(None),
    }
  }

  fn finish_merge(&self) -> Result<()> {
    match &self.inner {
      KnnVectorsReaderInner::Direct(inner) => inner.finish_merge(),
      KnnVectorsReaderInner::Slow(_) => Ok(()),
    }
  }
}

impl<D> IndexReader for MockRandomWrappedReader<D>
where
  D: Directory,
{
  type ContextKind = LeafReaderContextKind;
  type TermVectors = TermVectorsType<<Self as CodecReader>::TermVectorsReader>;
  type StoredFields = StoredFieldsType<<Self as CodecReader>::StoredFieldsReader>;
  type ReaderCacheHelper = <DefaultLeafReader<D> as IndexReader>::ReaderCacheHelper;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    CodecReader::term_vectors(self)
  }

  fn max_doc(&self) -> Result<i32> {
    self.reader.max_doc()
  }

  fn num_docs(&self) -> Result<i32> {
    self.reader.num_docs()
  }

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    CodecReader::stored_fields(self)
  }

  fn do_close(&self) -> Result<()> {
    match self.hook {
      ReaderHook::Slow(_) => Ok(()),
      ReaderHook::Default | ReaderHook::Mismatched(_) => self.reader.do_close(),
    }
  }

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    self.reader.get_reader_cache_helper()
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
    match self.hook {
      ReaderHook::Slow(_) => &self.slow_index_base,
      ReaderHook::Default | ReaderHook::Mismatched(_) => self.reader.index_base(),
    }
  }
}

impl<D> LeafReader for MockRandomWrappedReader<D>
where
  D: Directory,
{
  type CacheHelper = <DefaultLeafReader<D> as LeafReader>::CacheHelper;
  type Terms = <BaseFieldsProducer<D> as Fields>::Terms;
  type NumericDocValues = <BaseDocValuesProducer<D> as DocValuesProducer>::NumericDocValues;
  type BinaryDocValues = <BaseDocValuesProducer<D> as DocValuesProducer>::BinaryDocValues;
  type SortedDocValues = <BaseDocValuesProducer<D> as DocValuesProducer>::SortedDocValues;
  type SortedNumericDocValues =
    <BaseDocValuesProducer<D> as DocValuesProducer>::SortedNumericDocValues;
  type SortedSetDocValues = <BaseDocValuesProducer<D> as DocValuesProducer>::SortedSetDocValues;
  type NormNumericDocValues = <BaseNormsProducer<D> as NormsProducer>::NumericDocValues;
  type DocValuesSkipper = <BaseDocValuesProducer<D> as DocValuesProducer>::DocValuesSkipper;
  type FloatVectorValues = <BaseKnnVectorsReader<D> as KnnVectorsReader>::FloatVectorValues;
  type ByteVectorValues = <BaseKnnVectorsReader<D> as KnnVectorsReader>::ByteVectorValues;
  type Bits = <DefaultLeafReader<D> as LeafReader>::Bits;
  type PointValues = <BasePointsReader<D> as PointsReader>::PointValuesType;

  fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>> {
    self.reader.get_core_cache_helper()
  }

  fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
    match &self.hook {
      ReaderHook::Slow(state) => {
        self.ensure_open()?;
        match &state.fields {
          Some(fields) => fields.terms(field),
          None => Ok(None),
        }
      },
      ReaderHook::Default | ReaderHook::Mismatched(_) => LeafReader::terms(&self.reader, field),
    }
  }

  fn get_numeric_doc_values(&self, field: &str) -> Result<Option<Self::NumericDocValues>> {
    CodecReader::get_numeric_doc_values(self, field)
  }

  fn get_binary_doc_values(&self, field: &str) -> Result<Option<Self::BinaryDocValues>> {
    CodecReader::get_binary_doc_values(self, field)
  }

  fn get_sorted_doc_values(&self, field: &str) -> Result<Option<Self::SortedDocValues>> {
    CodecReader::get_sorted_doc_values(self, field)
  }

  fn get_sorted_numeric_doc_values(
    &self,
    field: &str,
  ) -> Result<Option<Self::SortedNumericDocValues>> {
    CodecReader::get_sorted_numeric_doc_values(self, field)
  }

  fn get_sorted_set_doc_values(&self, field: &str) -> Result<Option<Self::SortedSetDocValues>> {
    CodecReader::get_sorted_set_doc_values(self, field)
  }

  fn get_norm_values(&self, field: &str) -> Result<Option<Self::NormNumericDocValues>> {
    CodecReader::get_norm_values(self, field)
  }

  fn get_doc_values_skipper(&self, field: &str) -> Result<Option<Self::DocValuesSkipper>> {
    CodecReader::get_doc_values_skipper(self, field)
  }

  fn get_float_vector_values(&self, field: &str) -> Result<Option<Self::FloatVectorValues>> {
    LeafReader::get_float_vector_values(&self.reader, field)
  }

  fn get_byte_vector_values(&self, field: &str) -> Result<Option<Self::ByteVectorValues>> {
    LeafReader::get_byte_vector_values(&self.reader, field)
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
    LeafReader::search_nearest_vectors_f32(&self.reader, field, target, knn_collector, accept_docs)
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
    LeafReader::search_nearest_vectors_u8(&self.reader, field, target, knn_collector, accept_docs)
  }

  fn get_field_infos(&self) -> Result<Arc<FieldInfos>> {
    match &self.hook {
      ReaderHook::Mismatched(shuffled) => Ok(shuffled.clone()),
      ReaderHook::Default | ReaderHook::Slow(_) => self.reader.get_field_infos(),
    }
  }

  fn get_live_docs(&self) -> Result<Option<Self::Bits>> {
    self.reader.get_live_docs()
  }

  fn get_point_values(&self, field: &str) -> Result<Option<Self::PointValues>> {
    LeafReader::get_point_values(&self.reader, field)
  }

  fn check_integrity(&self) -> Result<()> {
    match self.hook {
      ReaderHook::Slow(_) => Ok(()),
      ReaderHook::Default | ReaderHook::Mismatched(_) => self.reader.check_integrity(),
    }
  }

  fn get_metadata(&self) -> Result<&LeafMetaData> {
    self.reader.get_metadata()
  }
}

impl<D> CodecReader for MockRandomWrappedReader<D>
where
  D: Directory,
{
  type StoredFieldsReader = MockRandomStoredFieldsReader<D>;
  type TermVectorsReader = MockRandomTermVectorsReader<D>;
  type NormsProducer = MockRandomNormsProducer<D>;
  type DocValuesProducer = MockRandomDocValuesProducer<D>;
  type FieldsProducer = MockRandomFieldsProducer<D>;
  type PointsReader = MockRandomPointsReader<D>;
  type KnnVectorsReader = MockRandomKnnVectorsReader<D>;

  fn get_fields_reader(&self) -> Result<Option<Self::StoredFieldsReader>> {
    match &self.hook {
      ReaderHook::Default => Ok(
        self
          .reader
          .get_fields_reader()?
          .map(MockRandomStoredFieldsReader::default),
      ),
      ReaderHook::Slow(state) => {
        let inner = state
          .store
          .as_ref()
          .ok_or_else(|| LuceneError::illegal_state("stored fields reader is None"))?
          .clone();
        Ok(Some(MockRandomStoredFieldsReader::slow(inner)))
      },
      ReaderHook::Mismatched(shuffled) => Ok(
        self
          .reader
          .get_fields_reader()?
          .map(|inner| MockRandomStoredFieldsReader::mismatched(inner, shuffled.clone())),
      ),
    }
  }

  fn get_term_vectors_reader(&self) -> Result<Option<Self::TermVectorsReader>> {
    match &self.hook {
      ReaderHook::Default | ReaderHook::Mismatched(_) => Ok(
        self
          .reader
          .get_term_vectors_reader()?
          .map(|inner| MockRandomTermVectorsReader {
            inner: TermVectorsReaderInner::Direct(inner),
          }),
      ),
      ReaderHook::Slow(state) => Ok(Some(MockRandomTermVectorsReader {
        inner: TermVectorsReaderInner::Slow(
          state
            .vectors
            .as_ref()
            .map(TryClone::try_clone)
            .transpose()?,
        ),
      })),
    }
  }

  fn get_norms_reader(&self) -> Result<Option<Self::NormsProducer>> {
    let mode = self.mode();
    let shuffled = self.shuffled_field_infos();
    let original = if shuffled.is_some() {
      Some(self.reader.get_field_infos()?)
    } else {
      None
    };
    let inner = match &self.hook {
      ReaderHook::Slow(state) => state.norms.clone(),
      ReaderHook::Default | ReaderHook::Mismatched(_) => self.reader.get_norms_reader()?,
    };
    Ok(inner.map(|inner| MockRandomNormsProducer {
      inner,
      mode,
      shuffled,
      original,
    }))
  }

  fn get_doc_values_reader(&self) -> Result<Option<Self::DocValuesProducer>> {
    let mode = self.mode();
    let shuffled = self.shuffled_field_infos();
    let original = if shuffled.is_some() {
      Some(self.reader.get_field_infos()?)
    } else {
      None
    };
    let inner = match &self.hook {
      ReaderHook::Slow(state) => state.doc_values.clone(),
      ReaderHook::Default | ReaderHook::Mismatched(_) => self.reader.get_doc_values_reader()?,
    };
    Ok(inner.map(|inner| MockRandomDocValuesProducer {
      inner,
      mode,
      shuffled,
      original,
    }))
  }

  fn get_postings_reader(&self) -> Result<Option<Self::FieldsProducer>> {
    match &self.hook {
      ReaderHook::Slow(state) => {
        let field_infos = self.get_field_infos()?;
        let mut indexed_fields = Vec::new();
        for field_info in field_infos.iter() {
          if *field_info.get_index_options()
            != crate::core::index::index_options::IndexOptions::None
          {
            indexed_fields.push(field_info.name.clone());
          }
        }
        indexed_fields.sort();
        Ok(Some(MockRandomFieldsProducer {
          state: Some(state.clone()),
          inner: None,
          mode: ReaderMode::Slow,
          indexed_fields,
        }))
      },
      ReaderHook::Default | ReaderHook::Mismatched(_) => {
        let Some(inner) = self.reader.get_postings_reader()? else {
          return Ok(None);
        };
        let mut iterator = inner.iterator()?;
        let mut indexed_fields = Vec::new();
        while iterator.has_next()? {
          let field = iterator.next()?.ok_or_else(|| {
            LuceneError::illegal_state("Fields.iterator().has_next returned true")
          })?;
          indexed_fields.push(field.clone());
        }
        Ok(Some(MockRandomFieldsProducer {
          state: None,
          inner: Some(inner),
          mode: self.mode(),
          indexed_fields,
        }))
      },
    }
  }

  fn get_points_reader(&self) -> Result<Option<Self::PointsReader>> {
    match self.hook {
      ReaderHook::Slow(_) => Ok(Some(MockRandomPointsReader {
        inner: PointsReaderInner::Slow(self.reader.clone()),
      })),
      ReaderHook::Default | ReaderHook::Mismatched(_) => Ok(self.reader.get_points_reader()?.map(
        |inner| MockRandomPointsReader {
          inner: PointsReaderInner::Direct(inner),
        },
      )),
    }
  }

  fn get_vector_reader(&self) -> Result<Option<Self::KnnVectorsReader>> {
    match self.hook {
      ReaderHook::Slow(_) => Ok(Some(MockRandomKnnVectorsReader {
        inner: KnnVectorsReaderInner::Slow(self.reader.clone()),
      })),
      ReaderHook::Default | ReaderHook::Mismatched(_) => Ok(self.reader.get_vector_reader()?.map(
        |inner| MockRandomKnnVectorsReader {
          inner: KnnVectorsReaderInner::Direct(inner),
        },
      )),
    }
  }
}

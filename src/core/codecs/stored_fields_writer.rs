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
use crate::core::codecs::DefaultStoredFieldsFormat;
use crate::core::codecs::stored_fields_format::StoredFieldsFormat;
use crate::core::codecs::stored_fields_reader::StoredFieldsReader;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::merge_state::{DocMap, MergeState, MergeStateDocMap};
use crate::core::index::stored_field_visitor::{Status, StoredFieldVisitor};
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::{BytesRef, DocIDMerger, Sub, SubBase, of};
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::store::DataInput;
use crate::core::store::directory::Directory;
use crate::core::util::accountable::Accountable;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::rc::Rc;
use std::sync::Arc;

/// Codec API for writing stored fields:
///
/// 1. For every document,
///    [`start_document`](StoredFieldsWriter::start_document) is called,
///    informing the Codec that a new document has started.
/// 2. `write_field_*` is called for each field in the document.
/// 3. After all documents have been written,
///    [`finish`](StoredFieldsWriter::finish) is called for
///    verification/sanity-checks.
/// 4. Finally, the writer is closed.
pub trait StoredFieldsWriter: Accountable + Closeable {
  /// Called before writing the stored fields of the document.
  /// `write_field` will be called for each stored field.
  /// This is called even if the document has no stored fields.
  fn start_document(&mut self) -> Result<()>;

  /// Called when a document and all its fields have been added.
  fn finish_document(&mut self) -> Result<()> {
    Ok(())
  }

  /// Writes a stored `i32` value.
  fn write_field_i32(&mut self, field_info: &FieldInfo, value: i32) -> Result<()>;

  /// Writes a stored `i64` value.
  fn write_field_i64(&mut self, field_info: &FieldInfo, value: i64) -> Result<()>;

  /// Writes a stored `f32` value.
  fn write_field_f32(&mut self, field_info: &FieldInfo, value: f32) -> Result<()>;

  /// Writes a stored `f64` value.
  fn write_field_f64(&mut self, field_info: &FieldInfo, value: f64) -> Result<()>;

  /// Writes a stored binary value from a [`DataInput`] and a `length`.
  fn write_field_with_input(
    &mut self,
    field_info: &FieldInfo,
    input: &mut impl DataInput,
    length: i32,
  ) -> Result<()> {
    let length = length as usize;
    let mut buf = vec![0u8; length];
    input.read_bytes(&mut buf, 0, length)?;
    self.write_field_bytes(field_info, &BytesRef::from_slice(buf, 0, length))
  }

  /// Writes a stored binary value.
  fn write_field_bytes(&mut self, field_info: &FieldInfo, value: &BytesRef<Vec<u8>>) -> Result<()>;

  /// Writes a stored string value.
  fn write_field_str(&mut self, field_info: &FieldInfo, value: &str) -> Result<()>;

  /**
   * Called before `Drop`, passing in the number of documents that were
   * written. Note that this is intentionally redundant (equivalent to
   * the number of calls to
   * [`start_document`](StoredFieldsWriter::start_document),
   * but a codec should check that this is the case to detect the runtime bug
   * described in LUCENE-1282.
   */
  fn finish<D>(&mut self, num_docs: i32, dir: &D) -> Result<()>
  where
    D: Directory;

  /// Merges in the stored fields from the readers in `merge_state`. The
  /// default implementation skips over deleted documents, and uses
  /// [`start_document`](StoredFieldsWriter::start_document), `write_field_*`,
  /// and [`finish`](StoredFieldsWriter::finish), returning the number of
  /// documents that were written. Implementations can provide this
  /// method for more sophisticated merging (bulk-byte copying, etc.).
  fn merge<D, D1, CR>(&mut self, merge_state: &mut MergeState<D, CR>, dir: &D1) -> Result<i32>
  where
    D1: Directory,
    CR: CodecReader,
    Self: Sized,
  {
    StoredFieldsWriterDefaults::merge(self, merge_state, dir)
  }
}

pub struct StoredFieldsWriterDefaults;

impl StoredFieldsWriterDefaults {
  pub fn merge<W, D, D1, CR>(
    writer: &mut W,
    merge_state: &mut MergeState<D, CR>,
    dir: &D1,
  ) -> Result<i32>
  where
    W: StoredFieldsWriter,
    D1: Directory,
    CR: CodecReader,
  {
    let mut subs = Vec::with_capacity(merge_state.stored_fields_readers.len());

    for i in 0..merge_state.stored_fields_readers.len() {
      {
        let reader = match merge_state.stored_fields_readers[i] {
          Some(ref mut r) => r,
          _ => {
            return Err(LuceneError::illegal_state(
              "Expected Lucene90CompressingStoredFieldsReader",
            ));
          },
        };
        reader.check_integrity()?;
      }
      let visitor = MergeVisitor::new(merge_state, i)?;

      subs.push(Sub::new(StoredFieldsMergeSub::<MergeStateDocMap<CR>>::new(
        visitor,
        merge_state.doc_maps[i].clone(),
        i,
        merge_state.max_docs[i],
      )));
    }

    let mut doc_count = 0;
    let mut doc_id_merger = of(subs, merge_state.needs_index_sort)?;

    while let Some(sub_idx) = doc_id_merger.next()? {
      let sub = &mut doc_id_merger.get_subs_mut()[sub_idx];
      debug_assert_eq!(sub.mapped_doc_id, doc_count);

      writer.start_document()?;
      let reader = match merge_state.stored_fields_readers[sub.sub.reader_index] {
        Some(ref mut r) => r,
        _ => {
          return Err(LuceneError::illegal_state(
            "Expected Lucene90CompressingStoredFieldsReader",
          ));
        },
      };
      reader.document_with_visitor(sub.sub.doc_id, &mut sub.sub.visitor, Some(&mut *writer))?;
      writer.finish_document()?;
      doc_count += 1;
    }

    writer.finish(doc_count, dir)?;
    Ok(doc_count)
  }
}
pub type DefaultStoredFieldsWriter<D> =
  <DefaultStoredFieldsFormat as StoredFieldsFormat>::StoredFieldsWriter<D>;
struct StoredFieldsMergeSub<DM> {
  pub reader_index: usize,
  pub max_doc: i32,
  pub visitor: MergeVisitor,
  pub doc_id: i32,
  pub doc_map: Rc<DM>,
}

impl<DM> StoredFieldsMergeSub<DM> {
  fn new(visitor: MergeVisitor, doc_map: Rc<DM>, reader_index: usize, max_doc: i32) -> Self {
    Self {
      reader_index,
      max_doc,
      visitor,
      doc_id: -1,
      doc_map,
    }
  }
}
impl<DM> SubBase for StoredFieldsMergeSub<DM>
where
  DM: DocMap,
{
  fn next_doc(&mut self) -> Result<i32> {
    self.doc_id += 1;
    if self.doc_id == self.max_doc {
      Ok(NO_MORE_DOCS)
    } else {
      Ok(self.doc_id)
    }
  }

  type DocMap = DM;

  fn get_doc_map(&self) -> Result<&Self::DocMap> {
    Ok(self.doc_map.as_ref())
  }
}
/// A visitor that adds every field it sees.
#[derive(Default, Clone)]
pub(crate) struct MergeVisitor {
  remapper: Option<Arc<FieldInfos>>,
}
impl MergeVisitor {
  pub(crate) fn new<D, CR>(merge_state: &MergeState<D, CR>, reader_index: usize) -> Result<Self>
  where
    CR: CodecReader,
  {
    for fi in merge_state.field_infos[reader_index].as_ref() {
      if let Some(other) = merge_state
        .merge_field_infos
        .field_info_by_number(fi.number)?
      {
        if other.name != fi.name {
          return Ok(Self {
            remapper: Some(Arc::clone(&merge_state.merge_field_infos)),
          });
        }
      } else {
        return Ok(Self {
          remapper: Some(Arc::clone(&merge_state.merge_field_infos)),
        });
      }
    }
    Ok(Self { remapper: None })
  }
  fn remap(&self, field: Arc<FieldInfo>) -> Result<Arc<FieldInfo>> {
    if let Some(ref remapper) = self.remapper {
      // field numbers are not aligned, we need to remap to the new field
      // number
      match remapper.field_info_by_name(&field.name)? {
        Some(new_field) => Ok(new_field),
        None => Err(LuceneError::illegal_state(format!(
          "FieldInfo not found in remapper with filed_name: {}",
          field.name
        ))),
      }
    } else {
      Ok(field)
    }
  }
}
impl StoredFieldVisitor for MergeVisitor {
  fn binary_field_with_input<S>(
    &mut self,
    field_info: Arc<FieldInfo>,
    input: &mut impl DataInput,
    length: i32,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    let writer =
      writer.ok_or_else(|| LuceneError::illegal_state("StoredFieldsWriter is required"))?;
    writer.write_field_with_input(self.remap(field_info)?.as_ref(), input, length)
  }

  fn binary_field<S>(
    &mut self,
    field_info: Arc<FieldInfo>,
    value: Vec<u8>,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    let writer =
      writer.ok_or_else(|| LuceneError::illegal_state("StoredFieldsWriter is required"))?;
    writer.write_field_bytes(
      self.remap(field_info)?.as_ref(),
      &BytesRef::from_bytes(value),
    )
  }

  fn string_field<S>(
    &mut self,
    field_info: Arc<FieldInfo>,
    value: String,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    let writer =
      writer.ok_or_else(|| LuceneError::illegal_state("StoredFieldsWriter is required"))?;
    writer.write_field_str(self.remap(field_info)?.as_ref(), &value)
  }

  fn int_field<S>(
    &mut self,
    field_info: Arc<FieldInfo>,
    value: i32,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    let writer =
      writer.ok_or_else(|| LuceneError::illegal_state("StoredFieldsWriter is required"))?;
    writer.write_field_i32(self.remap(field_info)?.as_ref(), value)
  }

  fn long_field<S>(
    &mut self,
    field_info: Arc<FieldInfo>,
    value: i64,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    let writer =
      writer.ok_or_else(|| LuceneError::illegal_state("StoredFieldsWriter is required"))?;
    writer.write_field_i64(self.remap(field_info)?.as_ref(), value)
  }

  fn float_field<S>(
    &mut self,
    field_info: Arc<FieldInfo>,
    value: f32,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    let writer =
      writer.ok_or_else(|| LuceneError::illegal_state("StoredFieldsWriter is required"))?;
    writer.write_field_f32(self.remap(field_info)?.as_ref(), value)
  }

  fn double_field<S>(
    &mut self,
    field_info: Arc<FieldInfo>,
    value: f64,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    let writer =
      writer.ok_or_else(|| LuceneError::illegal_state("StoredFieldsWriter is required"))?;
    writer.write_field_f64(self.remap(field_info)?.as_ref(), value)
  }

  fn needs_field<S>(
    &mut self,
    _field_info: Arc<FieldInfo>,
    _writer: Option<&mut S>,
  ) -> Result<Status>
  where
    S: StoredFieldsWriter,
  {
    Ok(Status::Yes)
  }
}

pub enum StoredFieldsWriterEnum2<A, B> {
  A(A),
  B(B),
}

impl<A, B> Closeable for StoredFieldsWriterEnum2<A, B>
where
  A: Closeable,
  B: Closeable,
{
  fn close(&mut self) -> Result<()> {
    match self {
      Self::A(inner) => inner.close(),
      Self::B(inner) => inner.close(),
    }
  }
}

impl<A, B> Accountable for StoredFieldsWriterEnum2<A, B>
where
  A: Accountable,
  B: Accountable,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    match self {
      Self::A(inner) => inner.ram_bytes_used(),
      Self::B(inner) => inner.ram_bytes_used(),
    }
  }
}

impl<A, B> StoredFieldsWriter for StoredFieldsWriterEnum2<A, B>
where
  A: StoredFieldsWriter,
  B: StoredFieldsWriter,
{
  fn start_document(&mut self) -> Result<()> {
    match self {
      Self::A(inner) => inner.start_document(),
      Self::B(inner) => inner.start_document(),
    }
  }

  fn finish_document(&mut self) -> Result<()> {
    match self {
      Self::A(inner) => inner.finish_document(),
      Self::B(inner) => inner.finish_document(),
    }
  }

  fn write_field_i32(&mut self, field_info: &FieldInfo, value: i32) -> Result<()> {
    match self {
      Self::A(inner) => inner.write_field_i32(field_info, value),
      Self::B(inner) => inner.write_field_i32(field_info, value),
    }
  }

  fn write_field_i64(&mut self, field_info: &FieldInfo, value: i64) -> Result<()> {
    match self {
      Self::A(inner) => inner.write_field_i64(field_info, value),
      Self::B(inner) => inner.write_field_i64(field_info, value),
    }
  }

  fn write_field_f32(&mut self, field_info: &FieldInfo, value: f32) -> Result<()> {
    match self {
      Self::A(inner) => inner.write_field_f32(field_info, value),
      Self::B(inner) => inner.write_field_f32(field_info, value),
    }
  }

  fn write_field_f64(&mut self, field_info: &FieldInfo, value: f64) -> Result<()> {
    match self {
      Self::A(inner) => inner.write_field_f64(field_info, value),
      Self::B(inner) => inner.write_field_f64(field_info, value),
    }
  }

  fn write_field_with_input(
    &mut self,
    field_info: &FieldInfo,
    input: &mut impl DataInput,
    length: i32,
  ) -> Result<()> {
    match self {
      Self::A(inner) => inner.write_field_with_input(field_info, input, length),
      Self::B(inner) => inner.write_field_with_input(field_info, input, length),
    }
  }

  fn write_field_bytes(&mut self, field_info: &FieldInfo, value: &BytesRef<Vec<u8>>) -> Result<()> {
    match self {
      Self::A(inner) => inner.write_field_bytes(field_info, value),
      Self::B(inner) => inner.write_field_bytes(field_info, value),
    }
  }

  fn write_field_str(&mut self, field_info: &FieldInfo, value: &str) -> Result<()> {
    match self {
      Self::A(inner) => inner.write_field_str(field_info, value),
      Self::B(inner) => inner.write_field_str(field_info, value),
    }
  }

  fn finish<D>(&mut self, num_docs: i32, dir: &D) -> Result<()>
  where
    D: Directory,
  {
    match self {
      Self::A(inner) => inner.finish(num_docs, dir),
      Self::B(inner) => inner.finish(num_docs, dir),
    }
  }

  fn merge<D, D1, CR>(&mut self, merge_state: &mut MergeState<D, CR>, dir: &D1) -> Result<i32>
  where
    D1: Directory,
    CR: CodecReader,
  {
    match self {
      Self::A(inner) => inner.merge(merge_state, dir),
      Self::B(inner) => inner.merge(merge_state, dir),
    }
  }
}

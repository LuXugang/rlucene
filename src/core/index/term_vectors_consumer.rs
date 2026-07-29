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
#[cfg(test)]
use crate::core::codecs::codec;
use crate::core::codecs::term_vectors_format::TermVectorsFormat;
use crate::core::codecs::term_vectors_writer::{DefaultTermVectorsWriter, TermVectorsWriter};
use crate::core::codecs::{Codec, Codecs};
use crate::core::index::field_info::FieldInfo;
use crate::core::index::indexing_chain::PerField;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::sorter::DocMap;
use crate::core::index::sorting_term_vectors_consumer::SortingTermVectorsConsumer;
use crate::core::index::term_vectors_consumer_per_field::TermVectorsConsumerPerField;
use crate::core::index::terms_hash::TermsHash;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
#[cfg(test)]
use crate::core::store::dummy::dummy_directory::DummyDirectory;
use crate::core::store::flush_info::FlushInfo;
use crate::core::util::accountable::Accountable;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::int_block_pool::IntBlockPool;
use crate::core::util::{AtomicCounter, ByteBlockPool, Counter, IOUtils, TryIntoInt};
use std::cmp::Ordering;
use std::sync::Arc;

pub(crate) struct TermVectorsConsumer<D>
where
  D: Directory,
{
  directory: D,
  codec: Codecs,
  pub(crate) writer: Option<DefaultTermVectorsWriter<D>>,
  has_vectors: bool,
  num_vector_fields: i32,
  pub(crate) last_doc_id: i32,
  per_fields_idxs: Vec<PerFieldMeta>,
  sub: Option<SortingTermVectorsConsumer<D>>,
  pub(crate) base: TermsHash,
}

/// Parameter `idx` is the index of the [`PerField`] where the [`TermVectorsConsumerPerField`] resides.
/// [`PerField`] itself is located in the [`IndexingChain`](crate::core::index::indexing_chain::IndexingChain)'s `doc_fields` array.
///
/// Parameter `field_name` is the field name.
#[derive(Clone, Default)]
pub(crate) struct PerFieldMeta {
  pub(crate) idx: i32,
  pub(crate) field_name: String,
}

impl Eq for PerFieldMeta {}

impl PartialEq for PerFieldMeta {
  fn eq(&self, other: &Self) -> bool {
    self.cmp(other) == Ordering::Equal
  }
}

impl PartialOrd<Self> for PerFieldMeta {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for PerFieldMeta {
  fn cmp(&self, other: &Self) -> Ordering {
    self.field_name.cmp(&other.field_name)
  }
}

#[cfg(test)]
impl Default for TermVectorsConsumer<DummyDirectory> {
  fn default() -> Self {
    let directory = DummyDirectory;
    TermVectorsConsumer::new(codec::get_default(), directory, None)
  }
}

impl<D> TermVectorsConsumer<D>
where
  D: Directory + Clone,
{
  pub(crate) fn new(
    codec: Codecs,
    directory: D,
    sub: Option<SortingTermVectorsConsumer<D>>,
  ) -> Self {
    let base = TermsHash::new(Arc::new(AtomicCounter::new()));

    let per_fields = vec![PerFieldMeta::default(); 1];

    TermVectorsConsumer {
      directory,
      codec,
      writer: None,
      has_vectors: false,
      num_vector_fields: 0,
      last_doc_id: 0,
      per_fields_idxs: per_fields,
      base,
      sub,
    }
  }
  fn reset_fields(&mut self) {
    self.per_fields_idxs.clear();
    self.num_vector_fields = 0;
  }
  fn fill(&mut self, doc_id: i32) -> Result<()> {
    while self.last_doc_id < doc_id {
      match self.sub {
        Some(ref mut sub) => {
          let writer = sub.writer.as_mut().ok_or_else(|| {
            LuceneError::illegal_state("TermVectorsConsumer writer is not initialized")
          })?;
          writer.start_document(0)?;
          writer.finish_document()?;
        },
        None => {
          let writer = self.writer.as_mut().ok_or_else(|| {
            LuceneError::illegal_state("TermVectorsConsumer writer is not initialized")
          })?;
          writer.start_document(0)?;
          writer.finish_document()?;
        },
      }
      self.last_doc_id += 1;
    }
    Ok(())
  }

  pub(crate) fn set_has_vectors(&mut self) {
    self.has_vectors = true;
  }
  pub(crate) fn finish_document<D1>(
    &mut self,
    doc_id: i32,
    info: &SegmentInfo<D1>,
    per_fields: &mut [PerField],
    int_pool: &mut IntBlockPool,
    byte_pool: &mut ByteBlockPool,
  ) -> Result<()>
  where
    D1: Directory,
  {
    if !self.has_vectors {
      return Ok(());
    }

    ArrayUtil::intro_sort_with_range(
      &mut self.per_fields_idxs,
      0,
      self.num_vector_fields.try_convert()?,
    )?;

    self.init_term_vectors_writer(info)?;
    self.fill(doc_id)?;
    // Append term vectors to the real outputs:
    match self.sub {
      Some(ref mut sub) => {
        if let Some(writer) = sub.writer.as_mut() {
          writer.start_document(self.num_vector_fields)?;
        }
      },
      None => {
        self
          .writer
          .as_mut()
          .ok_or_else(|| LuceneError::illegal_state("writer not initialized"))?
          .start_document(self.num_vector_fields)?;
      },
    }
    let idxs = std::mem::take(&mut self.per_fields_idxs);
    for per_field_idx in idxs.into_iter().take(self.num_vector_fields as usize) {
      let v = &mut per_fields[per_field_idx.idx as usize];
      let terms_hash_per_field = v
        .terms_hash_per_field
        .as_mut()
        .ok_or_else(|| LuceneError::illegal_state("terms_hash_per_field not initialized"))?;
      let next_per_field = terms_hash_per_field
        .next_per_field
        .as_mut()
        .ok_or_else(|| LuceneError::illegal_state("next_per_field not initialized"))?;
      next_per_field.finish_document(self, int_pool, byte_pool)?;
      next_per_field.reset(byte_pool)
    }

    match self.sub {
      Some(ref mut sub) => {
        sub
          .writer
          .as_mut()
          .ok_or_else(|| LuceneError::illegal_state("writer not initialized"))?
          .finish_document()?;
      },
      None => {
        self
          .writer
          .as_mut()
          .ok_or_else(|| LuceneError::illegal_state("writer not initialized"))?
          .finish_document()?;
      },
    }
    debug_assert_eq!(
      self.last_doc_id, doc_id,
      "last_doc_id = {}, doc_id = {}",
      self.last_doc_id, doc_id
    );

    self.last_doc_id += 1;
    self.reset_fields();
    Ok(())
  }
  pub(crate) fn start_document(&mut self) -> Result<()> {
    self.reset_fields();
    self.num_vector_fields = 0;
    Ok(())
  }
  pub(crate) fn add_field(
    &self,
    field_info: Arc<FieldInfo>,
  ) -> Result<TermVectorsConsumerPerField> {
    TermVectorsConsumerPerField::new(self, field_info)
  }
  pub(crate) fn write_per_field(
    &mut self,
    per_field: &mut TermVectorsConsumerPerField,
    int_pool: &mut IntBlockPool,
    byte_pool: &ByteBlockPool,
  ) -> Result<()> {
    match self.sub {
      Some(ref mut sub) => {
        let writer = sub
          .writer
          .as_mut()
          .ok_or_else(|| LuceneError::illegal_state("writer not initialized"))?;
        per_field.write_to_writer(writer, int_pool, byte_pool)
      },
      None => {
        let writer = self
          .writer
          .as_mut()
          .ok_or_else(|| LuceneError::illegal_state("writer not initialized"))?;
        per_field.write_to_writer(writer, int_pool, byte_pool)
      },
    }
  }
  pub(crate) fn add_field_to_flush(&mut self, meta: PerFieldMeta) -> Result<()> {
    let num_vector_fields = self.num_vector_fields as usize;
    if num_vector_fields == self.per_fields_idxs.len() {
      ArrayUtil::grow_with_len(&mut self.per_fields_idxs, num_vector_fields + 1)?;
    }

    self.per_fields_idxs[num_vector_fields] = meta;
    self.num_vector_fields += 1;
    Ok(())
  }
  pub(crate) fn flush<DM, D1>(
    &mut self,
    state: &SegmentWriteState<D>,
    sort_map: Option<&DM>,
    info: &SegmentInfo<D1>,
  ) -> Result<()>
  where
    DM: DocMap,
    D1: Directory,
  {
    if self.writer.is_some() || self.sub.as_ref().is_some_and(|sub| sub.writer.is_some()) {
      let num_docs = info.max_doc()?;
      debug_assert!(num_docs > 0);
      // At least one doc in this run had term vectors enabled
      let finish_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        self.fill(num_docs)?;
        match self.sub {
          Some(ref mut sub) => {
            let writer = sub
              .writer
              .as_mut()
              .ok_or_else(|| LuceneError::illegal_state("writer not initialized"))?;
            writer.finish(num_docs, &sub.tmp_directory)
          },
          None => {
            let writer = self
              .writer
              .as_mut()
              .ok_or_else(|| LuceneError::illegal_state("writer not initialized"))?;
            writer.finish(num_docs, state.directory)
          },
        }
      }));
      match self.sub {
        Some(ref mut sub) => {
          let writer = sub
            .writer
            .as_mut()
            .ok_or_else(|| LuceneError::illegal_state("writer not initialized"))?;
          let close_result = writer.close();
          close_result?;
        },
        None => {
          let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| LuceneError::illegal_state("writer not initialized"))?;
          let close_result = writer.close();
          close_result?;
        },
      }
      match finish_result {
        Ok(result) => result?,
        Err(payload) => std::panic::resume_unwind(payload),
      }

      if let Some(ref mut sub) = self.sub {
        sub.flush(state, sort_map, info)?;
      }
    }

    Ok(())
  }

  fn init_term_vectors_writer<D1>(&mut self, info: &SegmentInfo<D1>) -> Result<()>
  where
    D1: Directory,
  {
    match self.sub {
      Some(ref mut sub) => {
        if sub.writer.is_none() {
          sub.init_term_vectors_writer(self.last_doc_id, info, self.base.bytes_used.get())?;
          self.last_doc_id = 0;
        }
      },
      None => {
        if self.writer.is_none() {
          let flush_info = FlushInfo::new(self.last_doc_id, self.base.bytes_used.get());
          let context = IOContext::with_flush(flush_info)?;

          self.writer = Option::from(self.codec.term_vectors_format().vectors_writer(
            self.directory.clone(),
            info,
            &context,
          )?);
          self.last_doc_id = 0;
        }
      },
    }
    Ok(())
  }

  pub(crate) fn abort(&mut self) -> Result<()> {
    let close_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match self.sub {
      Some(ref mut sub) => IOUtils::close_resources_while_handling_error(sub.writer.as_mut()),
      None => IOUtils::close_resources_while_handling_error(self.writer.as_mut()),
    }));
    if let Some(ref mut sub) = self.sub {
      sub.abort()?;
    }
    match close_result {
      Ok(result) => result,
      Err(payload) => std::panic::resume_unwind(payload),
    }
  }
}

impl<D> Accountable for TermVectorsConsumer<D>
where
  D: Directory,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    match self.sub {
      Some(ref sub) => sub
        .writer
        .as_ref()
        .map_or(Ok(0), Accountable::ram_bytes_used),
      None => self
        .writer
        .as_ref()
        .map_or(Ok(0), Accountable::ram_bytes_used),
    }
  }
}

pub(crate) trait TermVectorsConsumerBase {
  type Directory: Directory;
  fn flush<DM, D1>(
    &mut self,
    state: &SegmentWriteState<Self::Directory>,
    sort_map: Option<&DM>,
    info: &SegmentInfo<D1>,
  ) -> Result<()>
  where
    DM: DocMap,
    D1: Directory;
  fn init_term_vectors_writer<D1>(
    &mut self,
    last_doc_id: i32,
    info: &SegmentInfo<D1>,
    bytes_used: i64,
  ) -> Result<()>
  where
    D1: Directory;
  fn abort(&mut self) -> Result<()>;
}

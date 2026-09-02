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
use crate::core::codecs::knn_vectors_format::KnnVectorsFormat;
use crate::core::codecs::knn_vectors_writer::KnnVectorsWriter;
use crate::core::codecs::{Codec, CodecKnnVectorsWriter, Codecs};
use crate::core::index::field_info::FieldInfo;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::sorter::DocMap;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::IOUtils;
use crate::core::util::accountable::Accountable;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::info_stream::InfoStreamMT;
use std::sync::Arc;
/// Streams vector values for indexing to the given codec's vectors writer.
/// The codec's vectors writer is responsible for buffering and processing vectors
pub(crate) struct VectorValuesConsumer<D>
where
  D: Directory,
{
  pub(crate) writer: Option<CodecKnnVectorsWriter<D::IndexOutput>>,
  codec: Codecs,
  info_stream: InfoStreamMT,
  dir: D,
  field_infos: Arc<FieldInfos>,
  context: IOContext,
}
impl<D> VectorValuesConsumer<D>
where
  D: Directory,
{
  pub(crate) fn new(codec: Codecs, dir: D, info_stream: InfoStreamMT) -> Result<Self> {
    Ok(Self {
      writer: None,
      codec,
      info_stream,
      dir,
      field_infos: Arc::new(FieldInfos::default()),
      context: IOContext::default_io_context()?,
    })
  }
  fn init_knn_vectors_writer<D2>(&mut self, segment_info: &SegmentInfo<D2>) -> Result<()> {
    if self.writer.is_none() {
      let fmt = self.codec.knn_vectors_format()?;
      let initial_write_state = SegmentWriteState::new(
        self.info_stream.clone(),
        &self.dir,
        Arc::clone(&self.field_infos),
        &self.context,
      );
      self.writer = Some(fmt.fields_writer(&initial_write_state, segment_info)?);
    }
    Ok(())
  }
  pub(crate) fn add_field<D2>(
    &mut self,
    field_info: Arc<FieldInfo>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<usize> {
    self.init_knn_vectors_writer(segment_info)?;
    let write_state = SegmentWriteState::new(
      self.info_stream.clone(),
      &self.dir,
      Arc::clone(&self.field_infos),
      &self.context,
    );
    let writer = self
      .writer
      .as_mut()
      .ok_or_else(|| LuceneError::illegal_state("writer not initialized"))?;
    writer.add_field(&write_state, segment_info, field_info)
  }
  pub(crate) fn flush<DM, D2>(
    &mut self,
    segment_info: &mut SegmentInfo<D2>,
    sort_map: Option<&DM>,
  ) -> Result<()>
  where
    DM: DocMap,
  {
    if let Some(writer) = self.writer.as_mut() {
      let body_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        writer.flush(segment_info.max_doc()?, sort_map)?;
        writer.finish()
      }));
      let close_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| writer.close()));
      IOUtils::finally_caught_result(body_result, close_result)?;
    }
    Ok(())
  }
  pub(crate) fn abort(&mut self) {
    if let Some(writer) = self.writer.as_mut() {
      IOUtils::close_while_handling_exception(writer);
    }
  }
  pub(crate) fn get_accountable(&self) -> &Self {
    self
  }
}

impl<D> Accountable for VectorValuesConsumer<D>
where
  D: Directory,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    let writer_bytes = self
      .writer
      .as_ref()
      .map_or(Ok(0), Accountable::ram_bytes_used)?;
    // This consumer is the accounting root for its always-empty FieldInfos allocation.
    Ok(writer_bytes.saturating_add(std::mem::size_of_val(self.field_infos.as_ref()) as i64))
  }
}

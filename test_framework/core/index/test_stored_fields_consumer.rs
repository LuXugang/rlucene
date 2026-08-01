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
use crate::core::codecs::{CodecStoredFieldsWriter, Codecs};
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::sorter::DocMap;
use crate::core::index::stored_fields_consumer::{
  StoredFieldsConsumerBase, StoredFieldsConsumerDefaults,
};
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

#[allow(dead_code)] // for quick search
struct TestStoredFieldsConsumer;

pub(crate) struct TestStoredFieldsConsumerHook<D>
where
  D: Directory,
{
  pub(crate) writer: Option<CodecStoredFieldsWriter<D>>,
  start_doc_counter: Arc<AtomicI32>,
  finish_doc_counter: Arc<AtomicI32>,
}

impl<D> TestStoredFieldsConsumerHook<D>
where
  D: Directory,
{
  pub(crate) fn new(start_doc_counter: Arc<AtomicI32>, finish_doc_counter: Arc<AtomicI32>) -> Self {
    Self {
      writer: None,
      start_doc_counter,
      finish_doc_counter,
    }
  }
}

impl<D> StoredFieldsConsumerBase for TestStoredFieldsConsumerHook<D>
where
  D: Directory + Clone,
{
  type Directory = D;

  fn init_stored_fields_writer<D1>(
    &mut self,
    directory: &Self::Directory,
    codec: &Codecs,
    info: &mut SegmentInfo<D1>,
  ) -> Result<()>
  where
    D1: Directory,
  {
    StoredFieldsConsumerDefaults::init_stored_fields_writer(
      &mut self.writer,
      directory,
      codec,
      info,
    )
  }

  fn start_document(&mut self, last_doc: &mut i32, doc_id: i32) -> Result<()> {
    StoredFieldsConsumerDefaults::start_document(&mut self.writer, last_doc, doc_id)?;
    self.start_doc_counter.fetch_add(1, Ordering::SeqCst);
    Ok(())
  }

  fn finish_document(&mut self) -> Result<()> {
    StoredFieldsConsumerDefaults::finish_document(&mut self.writer)?;
    self.finish_doc_counter.fetch_add(1, Ordering::SeqCst);
    Ok(())
  }

  fn flush<DM, D1>(
    &mut self,
    _codec: &Codecs,
    state: &SegmentWriteState<Self::Directory>,
    _sort_map: Option<&DM>,
    info: &mut SegmentInfo<D1>,
  ) -> Result<()>
  where
    DM: DocMap,
    D1: Directory,
  {
    StoredFieldsConsumerDefaults::flush(&mut self.writer, state.directory, info)
  }

  fn abort(&mut self) -> Result<()> {
    StoredFieldsConsumerDefaults::abort(&mut self.writer)
  }
}

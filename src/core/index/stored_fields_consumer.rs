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
use crate::core::codecs::stored_fields_format::StoredFieldsFormat;
use crate::core::codecs::stored_fields_writer::{DefaultStoredFieldsWriter, StoredFieldsWriter};
use crate::core::codecs::{Codec, LATEST_CODEC};
use crate::core::document::field::FieldDataEnum;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::sorter::DocMap;
use crate::core::index::sorting_stored_fields_consumer::SortingStoredFieldsConsumer;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::accountable::Accountable;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;

pub(crate) struct StoredFieldsConsumer<D>
where
  D: Directory,
{
  directory: D,
  pub(crate) writer: Option<DefaultStoredFieldsWriter<D::IndexOutput>>,
  last_doc: i32,
  sub: Option<SortingStoredFieldsConsumer<D>>,
}
impl<D> StoredFieldsConsumer<D>
where
  D: Directory,
{
  pub(crate) fn new(directory: D, sub: Option<SortingStoredFieldsConsumer<D>>) -> Self {
    Self {
      directory,
      writer: None,
      last_doc: -1,
      sub,
    }
  }
  fn init_stored_fields_writer<D1>(&mut self, info: &mut SegmentInfo<D1>) -> Result<()>
  where
    D1: Directory,
  {
    match self.sub {
      Some(ref mut sub) => {
        if sub.writer.is_none() {
          sub.init_stored_fields_writer(info)?;
        }
      },
      None => {
        if self.writer.is_none() {
          let writer = LATEST_CODEC.stored_fields_format().fields_writer(
            &self.directory,
            info,
            &IOContext::default_io_context()?,
          )?;
          self.writer = Some(writer);
        }
      },
    }
    Ok(())
  }

  pub(crate) fn start_document<D1>(&mut self, doc_id: i32, info: &mut SegmentInfo<D1>) -> Result<()>
  where
    D1: Directory,
  {
    debug_assert!(self.last_doc < doc_id);
    self.init_stored_fields_writer(info)?;

    match self.sub {
      Some(ref mut sub) => {
        while self.last_doc + 1 < doc_id {
          self.last_doc += 1;
          if let Some(writer) = &mut sub.writer {
            writer.start_document()?;
            writer.finish_document()?;
          }
        }
        self.last_doc += 1;
        if let Some(writer) = &mut sub.writer {
          writer.start_document()?;
        }
      },
      None => {
        while self.last_doc + 1 < doc_id {
          self.last_doc += 1;
          if let Some(writer) = &mut self.writer {
            writer.start_document()?;
            writer.finish_document()?;
          }
        }
        self.last_doc += 1;
        match self.writer {
          None => return Err(LuceneError::illegal_state("writer must be initialized")),
          Some(ref mut v) => v.start_document()?,
        }
      },
    }
    Ok(())
  }

  pub(crate) fn write_field(&mut self, info: &FieldInfo, value: &FieldDataEnum) -> Result<()> {
    match self.sub {
      Some(ref mut sub) => {
        let writer = sub
          .writer
          .as_mut()
          .ok_or_else(|| LuceneError::illegal_state("sub writer must be initialized"))?;
        Self::do_write_field(writer, info, value)?;
      },
      None => {
        let writer = self
          .writer
          .as_mut()
          .ok_or_else(|| LuceneError::illegal_state("writer must be initialized"))?;
        Self::do_write_field(writer, info, value)?;
      },
    }

    Ok(())
  }
  fn do_write_field(
    writer: &mut impl StoredFieldsWriter,
    info: &FieldInfo,
    value: &FieldDataEnum,
  ) -> Result<()> {
    match value {
      FieldDataEnum::Binary(bytes) => {
        writer.write_field_bytes(info, bytes)?;
      },
      FieldDataEnum::String(s) => {
        writer.write_field_str(info, s)?;
      },
      FieldDataEnum::Number(num) => {
        match num {
          Number::I32(n) => writer.write_field_i32(info, *n),
          Number::I64(n) => writer.write_field_i64(info, *n),
          Number::F32(n) => writer.write_field_f32(info, *n),
          Number::F64(n) => writer.write_field_f64(info, *n),
          _ => return Err(LuceneError::illegal_argument("unsupported number type")),
        }
      }?,
      _ => return Err(LuceneError::illegal_argument("unsupported field type")),
    }
    Ok(())
  }

  pub(crate) fn finish_document(&mut self) -> Result<()> {
    match self.sub {
      Some(ref mut sub) => {
        let writer = sub
          .writer
          .as_mut()
          .ok_or_else(|| LuceneError::illegal_state("sub writer must be initialized"))?;
        writer.finish_document()?;
      },
      None => {
        let writer = self
          .writer
          .as_mut()
          .ok_or_else(|| LuceneError::illegal_state("writer must be initialized"))?;
        writer.finish_document()?;
      },
    }
    Ok(())
  }

  pub(crate) fn finish<D1>(&mut self, max_doc: i32, info: &mut SegmentInfo<D1>) -> Result<()>
  where
    D1: Directory,
  {
    while self.last_doc < max_doc - 1 {
      self.start_document(self.last_doc + 1, info)?;
      self.finish_document()?;
    }
    Ok(())
  }

  pub(crate) fn flush<DM, D1>(
    &mut self,
    state: &mut SegmentWriteState<D>,
    sort_map: Option<&DM>,
    info: &mut SegmentInfo<D1>,
    dir: &D,
  ) -> Result<()>
  where
    DM: DocMap,
    D1: Directory,
  {
    match self.sub {
      Some(ref mut sub) => {
        let tmp_directory = &sub.tmp_directory;
        let writer = sub
          .writer
          .as_mut()
          .ok_or_else(|| LuceneError::illegal_state("sub writer must be initialized"))?;
        let max_doc_result = info.max_doc();
        let finish_result = match max_doc_result {
          Ok(max_doc) => writer.finish(max_doc, tmp_directory),
          Err(e) => Err(e),
        };
        let close_result = writer.close();
        close_result?;
        finish_result?;
        sub.flush(state, sort_map, info)?;
      },
      None => {
        let writer = self
          .writer
          .as_mut()
          .ok_or_else(|| LuceneError::illegal_state("writer must be initialized"))?;
        let max_doc_result = info.max_doc();
        let finish_result = match max_doc_result {
          Ok(max_doc) => writer.finish(max_doc, dir),
          Err(e) => Err(e),
        };
        let close_result = writer.close();
        close_result?;
        finish_result?;
      },
    }
    Ok(())
  }

  pub(crate) fn abort(&mut self) -> Result<()> {
    match self.sub {
      Some(ref mut sub) => sub.abort(),
      None => Ok(()),
    }
  }
}

impl<D> Accountable for StoredFieldsConsumer<D>
where
  D: Directory,
  DefaultStoredFieldsWriter<D::IndexOutput>: Accountable,
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

pub(crate) trait StoredFieldsConsumerBase {
  type Directory: Directory;
  fn init_stored_fields_writer<D1>(&mut self, info: &mut SegmentInfo<D1>) -> Result<()>
  where
    D1: Directory;
  fn flush<DM, D1>(
    &mut self,
    state: &SegmentWriteState<Self::Directory>,
    sort_map: Option<&DM>,
    info: &mut SegmentInfo<D1>,
  ) -> Result<()>
  where
    DM: DocMap,
    D1: Directory;
  fn abort(&mut self) -> Result<()>;
}

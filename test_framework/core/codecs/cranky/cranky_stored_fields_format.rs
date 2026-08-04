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
use crate::core::codecs::stored_fields_writer::{StoredFieldsWriter, StoredFieldsWriterDefaults};
use crate::core::index::BytesRef;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::merge_state::MergeState;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::store::directory::Directory;
use crate::core::store::{DataInput, IOContext, IndexInput};
use crate::core::util::accountable::Accountable;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use rand::RngExt;
use rand::prelude::StdRng;
use std::io::Error;
use std::sync::Arc;

pub struct CrankyStoredFieldsFormat<SFF> {
  delegate: SFF,
  random: Arc<Mutex<StdRng>>,
}

impl<SFF> CrankyStoredFieldsFormat<SFF> {
  pub fn new(delegate: SFF, random: Arc<Mutex<StdRng>>) -> Self {
    Self { delegate, random }
  }
}

impl<SFF> StoredFieldsFormat for CrankyStoredFieldsFormat<SFF>
where
  SFF: StoredFieldsFormat,
{
  type StoredFieldsReader<T: IndexInput> = SFF::StoredFieldsReader<T>;

  fn fields_reader<D1, D2>(
    &self,
    directory: &D1,
    segment_info: &SegmentInfo<D2>,
    field_infos: Arc<FieldInfos>,
    context: &IOContext,
  ) -> Result<Self::StoredFieldsReader<D1::IndexInput>>
  where
    D1: Directory,
    D2: Directory,
  {
    self
      .delegate
      .fields_reader(directory, segment_info, field_infos, context)
  }

  type StoredFieldsWriter<D: Directory> = CrankyStoredFieldsWriter<SFF::StoredFieldsWriter<D>>;

  fn fields_writer<D1, D2>(
    &self,
    directory: D1,
    segment_info: &mut SegmentInfo<D2>,
    context: &IOContext,
  ) -> Result<Self::StoredFieldsWriter<D1>>
  where
    D1: Directory,
    D2: Directory,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from StoredFieldsFormat.fieldsWriter()",
      )));
    }
    Ok(CrankyStoredFieldsWriter::new(
      self
        .delegate
        .fields_writer(directory, segment_info, context)?,
      Arc::clone(&self.random),
    ))
  }
}

pub struct CrankyStoredFieldsWriter<SFW> {
  delegate: SFW,
  random: Arc<Mutex<StdRng>>,
}

impl<SFW> CrankyStoredFieldsWriter<SFW> {
  fn new(delegate: SFW, random: Arc<Mutex<StdRng>>) -> Self {
    Self { delegate, random }
  }
}

impl<SFW> StoredFieldsWriter for CrankyStoredFieldsWriter<SFW>
where
  SFW: StoredFieldsWriter,
{
  fn start_document(&mut self) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from StoredFieldsWriter.startDocument()",
      )));
    }
    self.delegate.start_document()
  }

  fn finish_document(&mut self) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from StoredFieldsWriter.finishDocument()",
      )));
    }
    self.delegate.finish_document()
  }

  fn write_field_i32(&mut self, field_info: &FieldInfo, value: i32) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from StoredFieldsWriter.writeField()",
      )));
    }
    self.delegate.write_field_i32(field_info, value)
  }

  fn write_field_i64(&mut self, field_info: &FieldInfo, value: i64) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from StoredFieldsWriter.writeField()",
      )));
    }
    self.delegate.write_field_i64(field_info, value)
  }

  fn write_field_f32(&mut self, field_info: &FieldInfo, value: f32) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from StoredFieldsWriter.writeField()",
      )));
    }
    self.delegate.write_field_f32(field_info, value)
  }

  fn write_field_f64(&mut self, field_info: &FieldInfo, value: f64) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from StoredFieldsWriter.writeField()",
      )));
    }
    self.delegate.write_field_f64(field_info, value)
  }

  fn write_field_with_input(
    &mut self,
    field_info: &FieldInfo,
    input: &mut impl DataInput,
    length: i32,
  ) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from StoredFieldsWriter.writeField()",
      )));
    }
    self
      .delegate
      .write_field_with_input(field_info, input, length)
  }

  fn write_field_bytes(&mut self, field_info: &FieldInfo, value: &BytesRef<Vec<u8>>) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from StoredFieldsWriter.writeField()",
      )));
    }
    self.delegate.write_field_bytes(field_info, value)
  }

  fn write_field_str(&mut self, field_info: &FieldInfo, value: &str) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from StoredFieldsWriter.writeField()",
      )));
    }
    self.delegate.write_field_str(field_info, value)
  }

  fn finish<D>(&mut self, num_docs: i32, dir: &D) -> Result<()>
  where
    D: Directory,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from StoredFieldsWriter.finish()",
      )));
    }
    self.delegate.finish(num_docs, dir)
  }

  fn merge<D, D1, CR>(&mut self, merge_state: &mut MergeState<D, CR>, dir: &D1) -> Result<i32>
  where
    D: Directory,
    D1: Directory,
    CR: CodecReader,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from StoredFieldsWriter.merge()",
      )));
    }
    StoredFieldsWriterDefaults::merge(self, merge_state, dir)
  }
}

impl<SFW> Closeable for CrankyStoredFieldsWriter<SFW>
where
  SFW: StoredFieldsWriter,
{
  fn close(&mut self) -> Result<()> {
    self.delegate.close()?;
    if self.random.lock().random_range(0..1000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from StoredFieldsWriter.close()",
      )));
    }
    Ok(())
  }
}

impl<SFW> Accountable for CrankyStoredFieldsWriter<SFW>
where
  SFW: StoredFieldsWriter,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    self.delegate.ram_bytes_used()
  }
}

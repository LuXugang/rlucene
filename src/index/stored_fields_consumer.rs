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
use crate::codecs::stored_fields_format::StoredFieldsFormat;
use crate::codecs::stored_fields_writer::{StoredFieldsWriter, StoredFieldsWriterEnum};
use crate::codecs::Codec;
use crate::document::stored_value::{StoredValue, StoredValueType};
use crate::index::field_info::FieldInfo;
use crate::index::segment_info::SegmentInfo;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::sorter::DocMap;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct StoredFieldsConsumer<D>
where
    D: Directory,
{
    pub directory: Arc<Mutex<D>>,
    pub info: Rc<SegmentInfo<D>>,
    pub writer: Option<StoredFieldsWriterEnum<D>>,
    pub last_doc: i32,
}
impl<D> StoredFieldsConsumer<D>
where
    D: Directory,
{
    pub(crate) fn new(directory: Arc<Mutex<D>>, info: Rc<SegmentInfo<D>>) -> Self {
        Self {
            directory,
            info,
            writer: None,
            last_doc: -1,
        }
    }
}
impl<D> StoredFieldsConsumerBase for StoredFieldsConsumer<D>
where
    D: Directory,
{
    fn init_stored_fields_writer(&mut self, codec: &impl Codec) -> Result<()> {
        if self.writer.is_none() {
            let writer = codec.stored_fields_format().fields_writer(
                self.directory.clone(),
                self.info.clone(),
                &IOContext::default_io_context()?,
            )?;
            self.writer = Some(writer);
        }
        Ok(())
    }

    fn start_document(&mut self, codec: &impl Codec, doc_id: i32) -> Result<()> {
        debug_assert!(self.last_doc < doc_id);
        self.init_stored_fields_writer(codec)?;
        while self.last_doc + 1 < doc_id {
            self.last_doc += 1;
            if let Some(writer) = &mut self.writer {
                writer.start_document()?;
                writer.finish_document()?;
            }
        }
        self.last_doc += 1;
        if let Some(writer) = &mut self.writer {
            writer.start_document()?;
        }

        Ok(())
    }

    fn write_field(&mut self, info: &FieldInfo, value: &StoredValue) -> Result<()> {
        let writer = self.writer.as_mut().expect("writer must be initialized");

        match value.get_type() {
            StoredValueType::INTEGER => writer.write_field_i32(info, value.get_int_value()?),
            StoredValueType::LONG => writer.write_field_i64(info, value.get_long_value()?),
            StoredValueType::FLOAT => writer.write_field_f32(info, value.get_float_value()?),
            StoredValueType::DOUBLE => writer.write_field_f64(info, value.get_double_value()?),
            StoredValueType::BINARY => writer.write_field_bytes(info, value.get_binary_value()?),
            StoredValueType::STRING => writer.write_field_str(info, value.get_string_value()?),
        }
    }

    fn finish_document(&mut self) -> Result<()> {
        let writer = self.writer.as_mut().expect("writer must be initialized");
        writer.finish_document()
    }

    fn finish(&mut self, codec: &impl Codec, max_doc: i32) -> Result<()> {
        while self.last_doc < max_doc - 1 {
            self.start_document(codec, self.last_doc + 1)?;
            self.finish_document()?;
        }
        Ok(())
    }

    type Directory = D;

    fn flush(
        &mut self,
        state: &SegmentWriteState<Self::Directory>,
        _sort_map: &impl DocMap,
    ) -> Result<()> {
        self.writer
            .as_mut()
            .expect("writer must be initialized")
            .finish(state.segment_info.max_doc()?)
    }
}

pub(crate) trait StoredFieldsConsumerBase {
    fn init_stored_fields_writer(&mut self, codec: &impl Codec) -> Result<()>;
    fn start_document(&mut self, codec: &impl Codec, doc_id: i32) -> Result<()>;
    fn write_field(&mut self, info: &FieldInfo, value: &StoredValue) -> Result<()>;
    fn finish_document(&mut self) -> Result<()>;

    fn finish(&mut self, codec: &impl Codec, max_doc: i32) -> Result<()>;

    type Directory: Directory;
    fn flush(
        &mut self,
        state: &SegmentWriteState<Self::Directory>,
        _sort_map: &impl DocMap,
    ) -> Result<()>;
    fn abort(&mut self) {
        // not required in Rust Lucene
    }
}

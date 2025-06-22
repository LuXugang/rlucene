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
use crate::codecs::compressing::lucene90_compressing_stored_fields_format::Lucene90CompressingStoredFieldsFormat;
use crate::codecs::compression::compression_mode::{
    CompressionModeBase, CompressionModeEnum, CompressorEnum, DecompressorEnum,
};
use crate::codecs::compression::compressor::Compressor;
use crate::codecs::compression::decompressor::Decompressor;
use crate::codecs::stored_fields_format::StoredFieldsFormat;
use crate::codecs::stored_fields_reader::StoredFieldsReader;
use crate::codecs::stored_fields_writer::StoredFieldsWriter;
use crate::codecs::Codec;
use crate::document::stored_value::StoredValue;
use crate::index::field_info::FieldInfo;
use crate::index::segment_info::SegmentInfo;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::sorter::DocMap;
use crate::index::stored_field_visitor::{Status, StoredFieldVisitor};
use crate::index::stored_fields::StoredFields;
use crate::index::stored_fields_consumer::{StoredFieldsConsumer, StoredFieldsConsumerBase};
use crate::index::tracking_tmp_output_directory_wrapper::TrackingTmpOutputDirectoryWrapper;
use crate::index::BytesRef;
use crate::store::byte_buffers_data_input::ByteBuffersDataInput;
use crate::store::directory::Directory;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{DataInput, DataOutput, IOContext};
use crate::util::array_util::ArrayUtil;
use crate::util::clone::TryClone;
use crate::util::error::lucene_error::Result;
use crate::util::IOUtils;
use parking_lot::Mutex;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct SortingStoredFieldsConsumer<D>
where
    D: Directory,
{
    base: StoredFieldsConsumer<TrackingTmpOutputDirectoryWrapper<D>, D>,
    tmp_directory: Arc<Mutex<TrackingTmpOutputDirectoryWrapper<D>>>,
    stored_fields_format: Option<Lucene90CompressingStoredFieldsFormat>,
}
impl<D> SortingStoredFieldsConsumer<D>
where
    D: Directory,
{
    pub(crate) fn new(directory: Arc<Mutex<D>>, info: Rc<SegmentInfo<D>>) -> Self {
        let tmp_directory = Arc::new(Mutex::new(TrackingTmpOutputDirectoryWrapper::new(
            directory,
        )));
        Self {
            base: StoredFieldsConsumer::new(tmp_directory.clone(), info),
            tmp_directory,
            stored_fields_format: None,
        }
    }
}

impl<D> StoredFieldsConsumerBase for SortingStoredFieldsConsumer<D>
where
    D: Directory,
{
    fn init_stored_fields_writer(&mut self, _codec: &impl Codec) -> Result<()> {
        if self.base.writer.is_none() {
            let stored_fields_format = Lucene90CompressingStoredFieldsFormat::new(
                "TempStoredFields",
                CompressionModeEnum::Impl(NoCompression),
                128 * 1024,
                1,
                10,
            )?;
            self.base.writer = Option::from(stored_fields_format.fields_writer(
                self.tmp_directory.clone(),
                self.base.info.clone(),
                &IOContext::default_io_context()?,
            )?);
            self.stored_fields_format = Some(stored_fields_format);
        }
        Ok(())
    }

    fn start_document(&mut self, codec: &impl Codec, doc_id: i32) -> Result<()> {
        self.base.start_document(codec, doc_id)
    }

    fn write_field(&mut self, info: &FieldInfo, value: &StoredValue) -> Result<()> {
        self.base.write_field(info, value)
    }

    fn finish_document(&mut self) -> Result<()> {
        self.base.finish_document()
    }

    fn finish(&mut self, codec: &impl Codec, max_doc: i32) -> Result<()> {
        self.base.finish(codec, max_doc)
    }

    type Directory = D;

    fn flush<DM>(
        &mut self,
        state: &SegmentWriteState<Self::Directory>,
        sort_map: Option<Rc<DM>>,
        codec: &impl Codec,
    ) -> Result<()>
    where
        DM: DocMap,
    {
        self.base.flush(state, sort_map.clone(), codec)?;
        let mut dir = self.tmp_directory.lock();
        let mut reader = self.stored_fields_format.as_ref().unwrap().fields_reader(
            &mut *dir,
            state.segment_info.clone(),
            state.field_infos.clone(),
            &IOContext::default_io_context()?,
        )?;
        // Don't pull a merge instance, since merge instances optimize for
        // sequential access while we consume stored fields in random order here.
        let mut sort_writer = codec.stored_fields_format().fields_writer(
            state.directory.clone(),
            state.segment_info.clone(),
            &state.context,
        )?;

        reader.check_integrity()?;
        let mut visitor = CopyVisitor;
        let max_doc = state.segment_info.max_doc()?;
        for doc_id in 0..max_doc {
            sort_writer.start_document()?;
            let mapped_doc = if let Some(sort_map) = &sort_map {
                sort_map.new_to_old(doc_id)
            } else {
                doc_id
            };
            reader.document_with_visitor(mapped_doc, &mut visitor, &mut sort_writer)?;
            sort_writer.finish_document()?;
        }

        sort_writer.finish(max_doc)?;

        let values: Vec<String> = dir.get_temporary_files().values().cloned().collect();
        let name: Vec<&str> = values.iter().map(String::as_str).collect();
        IOUtils::delete_files(&mut *dir, name.as_slice())?;

        Ok(())
    }

    fn abort(&mut self) -> Result<()> {
        self.base.abort()?;
        let mut dir = self.tmp_directory.lock();
        let values: Vec<String> = dir.get_temporary_files().values().cloned().collect();
        let name: Vec<&str> = values.iter().map(String::as_str).collect();
        IOUtils::delete_files(&mut *dir, name.as_slice())?;
        Ok(())
    }
}

/// A visitor that copies every field it sees in the provided [`StoredFieldsWriter`]
#[derive(Default)]
pub(crate) struct CopyVisitor;
impl StoredFieldVisitor for CopyVisitor {
    fn binary_field_with_input(
        &mut self,
        field_info: Rc<FieldInfo>,
        input: &mut impl DataInput,
        length: i32,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        writer.write_field_with_input(&field_info, input, length)
    }

    fn binary_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: Vec<u8>,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        writer.write_field_bytes(&field_info, &BytesRef::from_bytes(value))
    }

    fn string_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: &str,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        writer.write_field_str(&field_info, value)
    }

    fn int_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: i32,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        writer.write_field_i32(&field_info, value)
    }

    fn long_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: i64,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        writer.write_field_i64(&field_info, value)
    }

    fn float_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: f32,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        writer.write_field_f32(&field_info, value)
    }

    fn double_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: f64,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        writer.write_field_f64(&field_info, value)
    }

    fn needs_field(
        &mut self,
        _field_info: Rc<FieldInfo>,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<Status> {
        Ok(Status::Yes)
    }
}

pub struct NoCompression;

impl Display for NoCompression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "CompressionModeImpl")
    }
}

impl Clone for NoCompression {
    fn clone(&self) -> Self {
        NoCompression
    }
}

impl CompressionModeBase for NoCompression {
    fn new_compressor(&self) -> CompressorEnum {
        CompressorEnum::Impl1(CompressorImpl)
    }

    fn new_decompressor(&self) -> DecompressorEnum {
        DecompressorEnum::Impl1(DecompressorImpl)
    }
}

pub struct CompressorImpl;
impl Compressor for CompressorImpl {
    fn compress(
        &mut self,
        buffers_input: &mut ByteBuffersDataInput<&[u8]>,
        out: &mut impl DataOutput,
    ) -> Result<()> {
        let len = buffers_input.length();
        out.copy_bytes(buffers_input, len)
    }
}
pub struct DecompressorImpl;

impl TryClone for DecompressorImpl {
    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(DecompressorImpl)
    }
}

impl Decompressor for DecompressorImpl {
    fn decompress(
        &mut self,
        input: &mut impl DataInput,
        _original_length: i32,
        offset: i32,
        length: i32,
        bytes: &mut BytesRef<Vec<u8>>,
    ) -> Result<()> {
        if let Some(new_array) = ArrayUtil::grow_no_copy(&bytes.bytes, length as usize) {
            bytes.bytes = new_array
        }
        input.skip_bytes(offset as i64)?;
        input.read_bytes(&mut bytes.bytes, 0, length)?;
        bytes.offset = 0;
        bytes.length = length as usize;
        Ok(())
    }
}

// StoredFieldsConsumer
pub enum StoredFieldsConsumerEnum<D1, D2>
where
    D1: Directory,
    D2: Directory,
{
    Sort(SortingStoredFieldsConsumer<D2>),
    UnSort(StoredFieldsConsumer<D1, D2>),
}
impl<D1, D2> StoredFieldsConsumerBase for StoredFieldsConsumerEnum<D1, D2>
where
    D1: Directory,
    D2: Directory,
{
    fn init_stored_fields_writer(&mut self, codec: &impl Codec) -> Result<()> {
        match self {
            StoredFieldsConsumerEnum::Sort(t) => t.init_stored_fields_writer(codec),
            StoredFieldsConsumerEnum::UnSort(s) => s.init_stored_fields_writer(codec),
        }
    }

    fn start_document(&mut self, codec: &impl Codec, doc_id: i32) -> Result<()> {
        match self {
            StoredFieldsConsumerEnum::Sort(t) => t.start_document(codec, doc_id),
            StoredFieldsConsumerEnum::UnSort(s) => s.start_document(codec, doc_id),
        }
    }

    fn write_field(&mut self, info: &FieldInfo, value: &StoredValue) -> Result<()> {
        match self {
            StoredFieldsConsumerEnum::Sort(t) => t.write_field(info, value),
            StoredFieldsConsumerEnum::UnSort(s) => s.write_field(info, value),
        }
    }

    fn finish_document(&mut self) -> Result<()> {
        match self {
            StoredFieldsConsumerEnum::Sort(t) => t.finish_document(),
            StoredFieldsConsumerEnum::UnSort(s) => s.finish_document(),
        }
    }

    fn finish(&mut self, codec: &impl Codec, max_doc: i32) -> Result<()> {
        match self {
            StoredFieldsConsumerEnum::Sort(t) => t.finish(codec, max_doc),
            StoredFieldsConsumerEnum::UnSort(s) => s.finish(codec, max_doc),
        }
    }

    type Directory = D2;

    fn flush<DM>(
        &mut self,
        state: &SegmentWriteState<Self::Directory>,
        sort_map: Option<Rc<DM>>,
        codec: &impl Codec,
    ) -> Result<()>
    where
        DM: DocMap,
    {
        match self {
            StoredFieldsConsumerEnum::Sort(t) => t.flush(state, sort_map, codec),
            StoredFieldsConsumerEnum::UnSort(s) => s.flush(state, sort_map, codec),
        }
    }

    fn abort(&mut self) -> Result<()> {
        match self {
            StoredFieldsConsumerEnum::Sort(t) => t.abort(),
            StoredFieldsConsumerEnum::UnSort(s) => s.abort(),
        }
    }
}

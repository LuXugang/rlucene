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
use crate::codecs::compression::compression_mode::{
    CompressionModeBase, CompressorEnum, DecompressorEnum,
};
use crate::codecs::compression::compressor::Compressor;
use crate::codecs::compression::decompressor::Decompressor;
use crate::codecs::stored_fields_writer::StoredFieldsWriter;
use crate::codecs::Codec;
use crate::document::stored_value::StoredValue;
use crate::index::field_info::FieldInfo;
use crate::index::segment_info::SegmentInfo;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::sorter::DocMap;
use crate::index::stored_field_visitor::{Status, StoredFieldVisitor};
use crate::index::stored_fields_consumer::{StoredFieldsConsumer, StoredFieldsConsumerBase};
use crate::index::tracking_tmp_output_directory_wrapper::TrackingTmpOutputDirectoryWrapper;
use crate::index::BytesRef;
use crate::store::byte_buffers_data_input::ByteBuffersDataInput;
use crate::store::directory::Directory;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{DataInput, DataOutput};
use crate::util::array_util::ArrayUtil;
use crate::util::clone::TryClone;
use crate::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct SortingStoredFieldsConsumer<D>
where
    D: Directory,
{
    base: StoredFieldsConsumer<D>,
    tmp_directory: TrackingTmpOutputDirectoryWrapper<D>,
}
impl<D> SortingStoredFieldsConsumer<D>
where
    D: Directory,
{
    pub(crate) fn new(directory: Arc<Mutex<D>>, info: Rc<SegmentInfo<D>>) -> Self {
        Self {
            base: StoredFieldsConsumer::new(directory.clone(), info),
            tmp_directory: TrackingTmpOutputDirectoryWrapper::new(directory),
        }
    }
}

impl<D> StoredFieldsConsumerBase for SortingStoredFieldsConsumer<D>
where
    D: Directory,
{
    fn init_stored_fields_writer(&mut self, codec: &impl Codec) -> Result<()> {
        self.base.writer.is_none();
        Ok(())
    }

    fn start_document(&mut self, codec: &impl Codec, doc_id: i32) -> Result<()> {
        todo!()
    }

    fn write_field(&mut self, info: &FieldInfo, value: &StoredValue) -> Result<()> {
        todo!()
    }

    fn finish_document(&mut self) -> Result<()> {
        todo!()
    }

    fn finish(&mut self, codec: &impl Codec, max_doc: i32) -> Result<()> {
        todo!()
    }

    type Directory = D;

    fn flush(
        &mut self,
        state: &SegmentWriteState<Self::Directory>,
        _sort_map: &impl DocMap,
    ) -> Result<()> {
        todo!()
    }
}

/// A visitor that copies every field it sees in the provided [`StoredFieldsWriter`]
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

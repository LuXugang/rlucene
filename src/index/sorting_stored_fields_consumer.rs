/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use crate::codecs::compressing::lucene90_compressing_stored_fields_format::Lucene90CompressingStoredFieldsFormat;
use crate::codecs::compression::compression_mode::{
    CompressionModeBase, CompressionModeEnum, CompressorEnum, DecompressorEnum,
};
use crate::codecs::compression::compressor::Compressor;
use crate::codecs::compression::decompressor::Decompressor;
use crate::codecs::stored_fields_format::StoredFieldsFormat;
use crate::codecs::stored_fields_reader::StoredFieldsReader;
use crate::codecs::stored_fields_writer::{StoredFieldsWriter, StoredFieldsWriterEnum};
use crate::codecs::Codec;
use crate::index::field_info::FieldInfo;
use crate::index::segment_info::SegmentInfo;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::sorter::DocMap;
use crate::index::stored_field_visitor::{Status, StoredFieldVisitor};
use crate::index::stored_fields::StoredFields;
use crate::index::stored_fields_consumer::StoredFieldsConsumerBase;
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
    pub(crate) writer: Option<StoredFieldsWriterEnum<TrackingTmpOutputDirectoryWrapper<D>>>,
    tmp_directory: Arc<Mutex<TrackingTmpOutputDirectoryWrapper<D>>>,
    stored_fields_format: Option<Lucene90CompressingStoredFieldsFormat>,
}
impl<D> SortingStoredFieldsConsumer<D>
where
    D: Directory,
{
    pub(crate) fn new(directory: Arc<Mutex<D>>) -> Self {
        let tmp_directory = Arc::new(Mutex::new(TrackingTmpOutputDirectoryWrapper::new(
            directory,
        )));
        Self {
            writer: None,
            tmp_directory,
            stored_fields_format: None,
        }
    }
}

impl<D> StoredFieldsConsumerBase for SortingStoredFieldsConsumer<D>
where
    D: Directory,
{
    type Directory = D;

    fn init_stored_fields_writer(&mut self, info: Rc<SegmentInfo<Self::Directory>>) -> Result<()> {
        let stored_fields_format = Lucene90CompressingStoredFieldsFormat::new(
            "TempStoredFields",
            CompressionModeEnum::Impl(NoCompression),
            128 * 1024,
            1,
            10,
        )?;
        self.writer = Some(stored_fields_format.fields_writer(
            self.tmp_directory.clone(),
            info.clone(),
            &IOContext::default_io_context()?,
        )?);
        self.stored_fields_format = Some(stored_fields_format);
        Ok(())
    }

    fn flush<DM>(
        &mut self,
        state: &SegmentWriteState<Self::Directory>,
        sort_map: Option<Rc<DM>>,
        codec: &impl Codec,
    ) -> Result<()>
    where
        DM: DocMap,
    {
        let mut tmp_dir = self.tmp_directory.lock();
        let mut reader = self.stored_fields_format.as_ref().unwrap().fields_reader(
            &mut *tmp_dir,
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

        let name_map: Vec<String> = tmp_dir.get_temporary_files().into_values().collect();
        let names: Vec<&str> = name_map.iter().map(String::as_str).collect();
        IOUtils::delete_files(&mut *tmp_dir, names.as_slice())?;

        Ok(())
    }

    fn abort(&mut self) -> Result<()> {
        let mut tmp_dir = self.tmp_directory.lock();
        let name_map: Vec<String> = tmp_dir.get_temporary_files().into_values().collect();
        let names: Vec<&str> = name_map.iter().map(String::as_str).collect();
        IOUtils::delete_files(&mut *tmp_dir, names.as_slice())?;
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

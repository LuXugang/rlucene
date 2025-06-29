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
use std::rc::Rc;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::codecs::compressing::lucene90_compressing_stored_fields_format::Lucene90CompressingStoredFieldsFormat;
use crate::codecs::stored_fields_reader::StoredFieldsReaderEnum;
use crate::codecs::stored_fields_writer::StoredFieldsWriterEnum;
use crate::index::field_infos::FieldInfos;
use crate::index::segment_info::SegmentInfo;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::error::lucene_error::Result;

/// Controls the format of stored fields.
pub trait StoredFieldsFormat {
    /// Returns a [`StoredFieldsReader`](crate::codecs::stored_fields_reader::StoredFieldsReader) to load stored fields.
    fn fields_reader<D1, D2>(
        &self,
        directory: &mut D1,
        segment_info: Rc<SegmentInfo<D2>>,
        field_infos: Rc<FieldInfos>,
        context: &IOContext,
    ) -> Result<StoredFieldsReaderEnum<D1::IndexInputType>>
    where
        D1: Directory,
        D2: Directory;

    /// Returns a [`StoredFieldsWriter`](crate::codecs::stored_fields_writer::StoredFieldsWriter) to write stored fields.
    fn fields_writer<D1, D2>(
        &self,
        directory: Arc<Mutex<D1>>,
        segment_info: Rc<SegmentInfo<D2>>,
        context: &IOContext,
    ) -> Result<StoredFieldsWriterEnum<D1>>
    where
        D1: Directory,
        D2: Directory;
}

pub enum StoredFieldsFormatEnum {
    Lucene90Compressing(Lucene90CompressingStoredFieldsFormat),
}
impl StoredFieldsFormat for StoredFieldsFormatEnum {
    fn fields_reader<D1, D2>(
        &self,
        directory: &mut D1,
        segment_info: Rc<SegmentInfo<D2>>,
        field_infos: Rc<FieldInfos>,
        context: &IOContext,
    ) -> Result<StoredFieldsReaderEnum<D1::IndexInputType>>
    where
        D1: Directory,
        D2: Directory,
    {
        match self {
            StoredFieldsFormatEnum::Lucene90Compressing(format) => {
                format.fields_reader(directory, segment_info, field_infos, context)
            },
        }
    }

    fn fields_writer<D1, D2>(
        &self,
        directory: Arc<Mutex<D1>>,
        segment_info: Rc<SegmentInfo<D2>>,
        context: &IOContext,
    ) -> Result<StoredFieldsWriterEnum<D1>>
    where
        D1: Directory,
        D2: Directory,
    {
        match self {
            StoredFieldsFormatEnum::Lucene90Compressing(format) => {
                format.fields_writer(directory, segment_info, context)
            },
        }
    }
}

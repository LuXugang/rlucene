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
use crate::codecs::stored_fields_reader::StoredFieldsReaderEnum;
use crate::codecs::stored_fields_writer::StoredFieldsWriterEnum;
use crate::index::field_infos::FieldInfos;
use crate::index::segment_info::SegmentInfo;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::error::lucene_error::Result;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// Controls the format of stored fields.
pub trait StoredFieldsFormat {
    /// Returns a [`StoredFieldsReader`] to load stored fields.
    fn fields_reader<D>(
        &self,
        directory: &mut D,
        segment_info: Rc<SegmentInfo<D>>,
        field_infos: Rc<FieldInfos>,
        context: &IOContext,
    ) -> Result<StoredFieldsReaderEnum<D::IndexInputType>>
    where
        D: Directory;

    /// Returns a [`StoredFieldsWriter`] to write stored fields.
    fn fields_writer<D>(
        &self,
        directory: Arc<Mutex<D>>,
        segment_info: Rc<SegmentInfo<D>>,
        context: &IOContext,
    ) -> Result<StoredFieldsWriterEnum<D>>
    where
        D: Directory;
}

pub enum StoredFieldsFormatEnum {
    Lucene90Compressing(Lucene90CompressingStoredFieldsFormat),
}
impl StoredFieldsFormat for StoredFieldsFormatEnum {
    fn fields_reader<D>(
        &self,
        directory: &mut D,
        segment_info: Rc<SegmentInfo<D>>,
        field_infos: Rc<FieldInfos>,
        context: &IOContext,
    ) -> Result<StoredFieldsReaderEnum<D::IndexInputType>>
    where
        D: Directory,
    {
        match self {
            StoredFieldsFormatEnum::Lucene90Compressing(format) => {
                format.fields_reader(directory, segment_info, field_infos, context)
            }
        }
    }

    fn fields_writer<D>(
        &self,
        directory: Arc<Mutex<D>>,
        segment_info: Rc<SegmentInfo<D>>,
        context: &IOContext,
    ) -> Result<StoredFieldsWriterEnum<D>>
    where
        D: Directory,
    {
        match self {
            StoredFieldsFormatEnum::Lucene90Compressing(format) => {
                format.fields_writer(directory, segment_info, context)
            }
        }
    }
}

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
use crate::codecs::lucene90_doc_values_format::Lucene90DocValuesFormat;
use crate::codecs::CodecUtil;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::IndexFileNames;
use crate::store::directory::Directory;
use crate::store::IndexOutput;
use crate::util::error::lucene_error::{LuceneError, Result};

/// writer for [`Lucene90DocValuesFormat`](Lucene90DocValuesFormat).
pub(crate) struct Lucene90DocValuesConsumer<O: IndexOutput> {
    data: O,
    meta: O,
    max_doc: i32,
    terms_dict_buffer: Vec<u8>,
    skip_index_interval_size: i32,
}
impl<O: IndexOutput> Lucene90DocValuesConsumer<O> {
    /// expert: Creates a new writer
    pub fn new<D>(
        state: &SegmentWriteState<D>,
        skip_index_interval_size: i32,
        data_codec: &str,
        data_extension: &str,
        meta_codec: &str,
        meta_extension: &str,
    ) -> Result<Self>
    where
        D: Directory<IndexOutputType = O>,
    {
        let terms_dict_buffer = vec![0u8; 1 << 14];

        let data_name = IndexFileNames::segment_file_name(
            &state.segment_info.name,
            &state.segment_suffix,
            data_extension,
        );
        let mut dir = state
            .directory
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire  lock.".to_string()))?;
        let mut data = dir.create_output(&data_name, &state.context)?;
        CodecUtil::write_index_header(
            &mut data,
            data_codec,
            Lucene90DocValuesFormat::VERSION_CURRENT,
            &state.segment_info.get_id(),
            &state.segment_suffix,
        )?;

        let meta_name = IndexFileNames::segment_file_name(
            &state.segment_info.name,
            &state.segment_suffix,
            meta_extension,
        );
        let mut meta = dir.create_output(&meta_name, &state.context)?;
        CodecUtil::write_index_header(
            &mut meta,
            meta_codec,
            Lucene90DocValuesFormat::VERSION_CURRENT,
            &state.segment_info.get_id(),
            &state.segment_suffix,
        )?;

        let max_doc = state.segment_info.max_doc()?;
        Ok(Lucene90DocValuesConsumer {
            data,
            meta,
            max_doc,
            terms_dict_buffer,
            skip_index_interval_size,
        })
    }
}

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
use crate::core::codecs::CodecUtil;
use crate::core::codecs::lucene90_points_format::Lucene90PointsFormat;
use crate::core::codecs::points_reader::PointsReader;
use crate::core::codecs::points_writer::PointsWriter;
use crate::core::index::IndexFileNames;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::IndexOutput;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

pub struct Lucene90PointWriter<O>
where
    O: IndexOutput,
{
    data_out: O,
    meta_out: O,
    index_out: O,
    max_points_in_leaf_node: i32,
    max_mb_sort_in_heap: f64,
}

impl<O> Lucene90PointWriter<O>
where
    O: IndexOutput,
{
    pub fn new<D1, D2>(
        write_state: &SegmentWriteState<D1>,
        max_points_in_leaf_node: i32,
        max_mb_sort_in_heap: f64,
        segment_info: &SegmentInfo<D2>,
    ) -> Result<Self>
    where
        D1: Directory<IndexOutput = O>,
        D2: Directory,
    {
        debug_assert!(write_state.field_infos.has_point_values());

        let data_file = IndexFileNames::segment_file_name(
            &segment_info.name,
            &write_state.segment_suffix,
            Lucene90PointsFormat::DATA_EXTENSION,
        );
        let mut data_out = write_state
            .directory
            .create_output(&data_file, write_state.context)?;

        CodecUtil::write_index_header(
            &mut data_out,
            Lucene90PointsFormat::DATA_CODEC_NAME,
            Lucene90PointsFormat::VERSION_CURRENT,
            segment_info.get_id(),
            &write_state.segment_suffix,
        )?;

        let meta_file = IndexFileNames::segment_file_name(
            &segment_info.name,
            &write_state.segment_suffix,
            Lucene90PointsFormat::META_EXTENSION,
        );
        let mut meta_out = write_state
            .directory
            .create_output(&meta_file, write_state.context)?;
        CodecUtil::write_index_header(
            &mut meta_out,
            Lucene90PointsFormat::META_CODEC_NAME,
            Lucene90PointsFormat::VERSION_CURRENT,
            segment_info.get_id(),
            &write_state.segment_suffix,
        )?;

        let index_file = IndexFileNames::segment_file_name(
            &segment_info.name,
            &write_state.segment_suffix,
            Lucene90PointsFormat::INDEX_EXTENSION,
        );
        let mut index_out = write_state
            .directory
            .create_output(&index_file, write_state.context)?;
        CodecUtil::write_index_header(
            &mut index_out,
            Lucene90PointsFormat::INDEX_CODEC_NAME,
            Lucene90PointsFormat::VERSION_CURRENT,
            segment_info.get_id(),
            &write_state.segment_suffix,
        )?;

        Ok(Self {
            data_out,
            meta_out,
            index_out,
            max_points_in_leaf_node,
            max_mb_sort_in_heap,
        })
    }
}

impl<O> PointsWriter for Lucene90PointWriter<O>
where
    O: IndexOutput,
{
    fn write_field<PR>(&mut self, _field_info: &Arc<FieldInfo>, _values: &mut PR) -> Result<()>
    where
        PR: PointsReader,
    {
        todo!()
    }

    fn finish(&mut self) -> Result<()> {
        todo!()
    }
}

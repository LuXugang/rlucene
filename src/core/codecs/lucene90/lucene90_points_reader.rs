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
use crate::core::index::IndexFileNames;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::store::directory::Directory;
use crate::core::store::{DataInput, IndexInput, ReadAdvice};
use crate::core::util::CoreHelper;
use crate::core::util::bkd::bkd_reader::BKDReader;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
/// Reads point values previously written with [`Lucene90PointsWriter`](crate::core::codecs::lucene90_points_writer::Lucene90PointsWriter)
pub struct Lucene90PointsReader<I>
where
    I: IndexInput,
{
    index_in: Arc<I>,
    data_in: Arc<Mutex<I>>,
    readers: HashMap<i32, Arc<BKDReader<I>>>,
    field_infos: Arc<FieldInfos>,
}

impl<I> Lucene90PointsReader<I>
where
    I: IndexInput,
{
    pub fn new<D1, D2>(
        read_state: &SegmentReadState<D1>,
        segment_info: &SegmentInfo<D2>,
    ) -> Result<Self>
    where
        D1: Directory<IndexInput = I>,
        D2: Directory,
    {
        let suffix = &read_state.segment_suffix;

        let meta_file_name = IndexFileNames::segment_file_name(
            &segment_info.name,
            suffix,
            Lucene90PointsFormat::META_EXTENSION,
        );
        let index_file_name = IndexFileNames::segment_file_name(
            &segment_info.name,
            suffix,
            Lucene90PointsFormat::INDEX_EXTENSION,
        );
        let data_file_name = IndexFileNames::segment_file_name(
            &segment_info.name,
            suffix,
            Lucene90PointsFormat::DATA_EXTENSION,
        );

        let mut index_in = read_state.directory.open_input(
            &index_file_name,
            &read_state
                .context
                .with_read_advice_self(ReadAdvice::RandomPreload)?,
        )?;
        CodecUtil::check_index_header(
            &mut index_in,
            Lucene90PointsFormat::INDEX_CODEC_NAME,
            Lucene90PointsFormat::VERSION_START,
            Lucene90PointsFormat::VERSION_CURRENT,
            segment_info.get_id(),
            suffix,
        )?;
        CodecUtil::retrieve_checksum(&mut index_in)?;
        // Points read whole ranges of bytes at once, so pass ReadAdvice.NORMAL to perform readahead.
        let mut data_in = read_state.directory.open_input(
            &data_file_name,
            &read_state
                .context
                .with_read_advice_self(ReadAdvice::Normal)?,
        )?;
        CodecUtil::check_index_header(
            &mut data_in,
            Lucene90PointsFormat::DATA_CODEC_NAME,
            Lucene90PointsFormat::VERSION_START,
            Lucene90PointsFormat::VERSION_CURRENT,
            segment_info.get_id(),
            suffix,
        )?;
        CodecUtil::retrieve_checksum(&mut data_in)?;

        let mut index_length: i64 = -1;
        let mut data_length: i64 = -1;
        let mut tmp_readers = HashMap::new();

        let data_in = Arc::new(Mutex::new(data_in));
        {
            let mut meta_in = read_state.directory.open_checksum_input(&meta_file_name)?;

            let result: Result<()> = (|| {
                CodecUtil::check_index_header(
                    &mut meta_in,
                    Lucene90PointsFormat::META_CODEC_NAME,
                    Lucene90PointsFormat::VERSION_START,
                    Lucene90PointsFormat::VERSION_CURRENT,
                    segment_info.get_id(),
                    suffix,
                )?;

                loop {
                    let field_number = meta_in.read_int()?;
                    if field_number == -1 {
                        break;
                    } else if field_number < 0 {
                        return Err(LuceneError::corrupt_index(format!(
                            "Illegal field number: {field_number}"
                        )));
                    }
                    let reader = BKDReader::new(&mut meta_in, &mut index_in, data_in.clone())?;
                    tmp_readers.insert(field_number, reader);
                }

                index_length = meta_in.read_long()?;
                data_length = meta_in.read_long()?;
                Ok(())
            })();

            match result {
                Ok(_) => {
                    CodecUtil::check_footer(&mut meta_in)?;
                },
                Err(e) => {
                    let e = CodecUtil::check_footer_with_error(&mut meta_in, e);
                    return Err(e);
                },
            }
        }

        CodecUtil::retrieve_checksum_with_expected(&mut index_in, index_length as usize)?;
        CodecUtil::retrieve_checksum_with_expected(&mut *data_in.lock(), data_length as usize)?;
        let index_in = Arc::new(index_in);
        let mut readers = HashMap::new();
        for mut value in tmp_readers.into_iter() {
            value.1.init_index_in(index_in.clone())?;
            readers.insert(value.0, Arc::new(value.1));
        }
        Ok(Self {
            index_in,
            data_in,
            readers,
            field_infos: read_state.field_infos.clone(),
        })
    }
}

impl<I> Clone for Lucene90PointsReader<I>
where
    I: IndexInput,
{
    fn clone(&self) -> Self {
        unreachable!(
            "{} {}",
            std::any::type_name::<Self>(),
            CoreHelper::CLONE_WARRING
        )
    }
}

impl<I> PointsReader for Lucene90PointsReader<I>
where
    I: IndexInput,
{
    fn check_integrity(&self) -> Result<()> {
        CodecUtil::checksum_entire_file(self.index_in.as_ref())?;
        CodecUtil::checksum_entire_file(&*self.data_in.lock())?;
        Ok(())
    }

    type PointValuesType = Arc<BKDReader<I>>;

    fn get_values(&self, field_name: &str) -> Result<Self::PointValuesType> {
        match self.field_infos.field_info_by_name(field_name) {
            Some(field_info) => {
                if field_info.get_point_dimension_count() == 0 {
                    return Err(LuceneError::illegal_state(format!(
                        "field=: {} does not index point values",
                        field_name
                    )));
                }
                match self.readers.get(&field_info.number) {
                    Some(reader) => Ok(reader.clone()),
                    None => Err(LuceneError::illegal_state(format!(
                        "No BKDReader found for field: {}",
                        field_name
                    ))),
                }
            },
            None => Err(LuceneError::illegal_state(format!(
                "field=: {} is unrecognized",
                field_name
            ))),
        }
    }
}

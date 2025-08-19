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
use crate::codecs::CodecUtil;
use crate::codecs::indexed_disi::indexed_disi_util;
use crate::codecs::lucene90_norms_format::Lucene90NormsFormat;
use crate::codecs::norms_consumer::NormsConsumer;
use crate::codecs::norms_producer::NormsProducer;
use crate::index::IndexFileNames;
use crate::index::field_info::FieldInfo;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::segment_info::SegmentInfo;
use crate::index::segment_write_state::SegmentWriteState;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::store::IndexOutput;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use std::sync::Arc;

/// Writer for
/// [`Lucene90NormsFormat`](crate::codecs::lucene90::lucene90_norms_format).
pub struct Lucene90NormsConsumer<O>
where
    O: IndexOutput,
{
    pub data: O,
    pub meta: O,
    pub max_doc: i32,
    closed: bool,
}
impl<O: IndexOutput> Lucene90NormsConsumer<O> {
    pub fn new<D, D1>(
        state: &mut SegmentWriteState<D>,
        data_codec: &str,
        data_extension: &str,
        meta_codec: &str,
        meta_extension: &str,
        segment_info: &SegmentInfo<D1>,
    ) -> Result<Self>
    where
        D: Directory<IndexOutput = O>,
        D1: Directory,
    {
        let data_name = IndexFileNames::segment_file_name(
            &segment_info.name,
            &state.segment_suffix,
            data_extension,
        );
        let mut data = state.directory.create_output(&data_name, state.context)?;
        CodecUtil::write_index_header(
            &mut data,
            data_codec,
            Lucene90NormsFormat::VERSION_CURRENT,
            segment_info.get_id(),
            &state.segment_suffix,
        )?;

        let meta_name = IndexFileNames::segment_file_name(
            &segment_info.name,
            &state.segment_suffix,
            meta_extension,
        );
        let mut meta = state.directory.create_output(&meta_name, state.context)?;
        CodecUtil::write_index_header(
            &mut meta,
            meta_codec,
            Lucene90NormsFormat::VERSION_CURRENT,
            segment_info.get_id(),
            &state.segment_suffix,
        )?;

        let max_doc = segment_info.max_doc()?;

        Ok(Self {
            data,
            meta,
            max_doc,
            closed: false,
        })
    }
    pub fn close(&mut self) -> Result<()> {
        if !self.closed {
            self.closed = true;
            self.meta.write_int(-1)?;
            CodecUtil::write_footer(&mut self.meta)?;
            CodecUtil::write_footer(&mut self.data)?;
        }
        Ok(())
    }
    fn num_bytes_per_value(&self, min: i64, max: i64) -> u8 {
        if min >= max {
            0
        } else if min >= i8::MIN as i64 && max <= i8::MAX as i64 {
            1
        } else if min >= i16::MIN as i64 && max <= i16::MAX as i64 {
            2
        } else if min >= i32::MIN as i64 && max <= i32::MAX as i64 {
            4
        } else {
            8
        }
    }
    fn write_values(
        values: &mut impl NumericDocValues,
        num_bytes_per_value: u8,
        out: &mut impl IndexOutput,
    ) -> Result<()> {
        while values.next_doc()? != NO_MORE_DOCS {
            let value = values.long_value()?;
            match num_bytes_per_value {
                1 => out.write_byte(value as u8)?,
                2 => out.write_short(value as i16)?,
                4 => out.write_int(value as i32)?,
                8 => out.write_long(value)?,
                _ => return Err(LuceneError::unreachable("invalid byte width")),
            }
        }
        Ok(())
    }
}
impl<O> Drop for Lucene90NormsConsumer<O>
where
    O: IndexOutput,
{
    fn drop(&mut self) {
        let result = self.close();
        match result {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Failed to close Lucene90NormsConsumer: {e:?}")
            },
        }
    }
}
impl<O> NormsConsumer for Lucene90NormsConsumer<O>
where
    O: IndexOutput,
{
    fn add_norms_field(
        &mut self,
        field: &Arc<FieldInfo>,
        norms_producer: &mut impl NormsProducer,
    ) -> Result<()> {
        let mut num_docs_with_value = 0;
        let mut min = i64::MAX;
        let mut max = i64::MIN;
        {
            let mut values = norms_producer.get_norms(field)?;

            while values.next_doc()? != NO_MORE_DOCS {
                num_docs_with_value += 1;
                let v = values.long_value()?;
                min = min.min(v);
                max = max.max(v);
            }
        }

        debug_assert!(num_docs_with_value <= self.max_doc);

        self.meta.write_int(field.number)?;

        if num_docs_with_value == 0 {
            self.meta.write_long(-2)?; // docsWithFieldOffset
            self.meta.write_long(0)?; // docsWithFieldLength
            self.meta.write_short(-1)?; // jumpTableEntryCount
            self.meta.write_byte(-1i8 as u8)?; // denseRankPower
        } else if num_docs_with_value == self.max_doc {
            self.meta.write_long(-1)?;
            self.meta.write_long(0)?;
            self.meta.write_short(-1)?;
            self.meta.write_byte(-1i8 as u8)?;
        } else {
            let offset = self.data.get_file_pointer();
            self.meta.write_long(offset)?; // docsWithFieldOffset

            let jump_table_entry_count;
            {
                let mut values = norms_producer.get_norms(field)?;
                jump_table_entry_count = indexed_disi_util::write_bitset_with_dense_rank_power(
                    &mut values,
                    &mut self.data,
                    indexed_disi_util::DEFAULT_DENSE_RANK_POWER,
                )?;
            }
            self.meta
                .write_long(self.data.get_file_pointer() - offset)?; // docsWithFieldLength
            self.meta.write_short(jump_table_entry_count)?;
            self.meta
                .write_byte(indexed_disi_util::DEFAULT_DENSE_RANK_POWER as u8)?;
        }

        self.meta.write_int(num_docs_with_value)?;
        let num_bytes_per_value = self.num_bytes_per_value(min, max);
        self.meta.write_byte(num_bytes_per_value)?;

        if num_bytes_per_value == 0 {
            self.meta.write_long(min)?;
        } else {
            self.meta.write_long(self.data.get_file_pointer())?;
            let mut values = norms_producer.get_norms(field)?;
            Self::write_values(&mut values, num_bytes_per_value, &mut self.data)?;
        }

        Ok(())
    }
}

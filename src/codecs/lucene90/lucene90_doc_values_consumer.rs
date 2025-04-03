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
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::index::{BytesRefBuilder, IndexFileNames};
use crate::store::directory::Directory;
use crate::store::{
    ByteArrayDataOutput, ByteBuffersDataOutput, ByteBuffersIndexOutput, DataOutput, IndexInput,
    IndexOutput,
};
use crate::util::array_util::ArrayUtil;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::compress::lz4::{FastCompressionHashTable, HashTableEnum, LZ4};
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::packed::direct_monotonic_writer::DirectMonotonicWriter;
use crate::util::{CommonUtil, StringHelper};

/// writer for [`Lucene90DocValuesFormat`](Lucene90DocValuesFormat).
pub(crate) struct Lucene90DocValuesConsumer<O: IndexOutput> {
    data: O,
    meta: O,
    max_doc: i32,
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
            skip_index_interval_size,
        })
    }
    fn add_terms_dict<I: IndexInput>(
        &mut self,
        values: &mut impl SortedSetDocValues<I>,
    ) -> Result<()> {
        let size = values.get_value_count()?;
        let meta = &mut self.meta;
        meta.write_vlong(size)?;
        let data = &mut self.data;
        let block_mask = Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_MASK as i64;
        let shift = Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SHIFT;
        meta.write_int(Lucene90DocValuesFormat::DIRECT_MONOTONIC_BLOCK_SHIFT)?;

        let mut address_buffer = ByteBuffersDataOutput::new();
        let mut address_output = ByteBuffersIndexOutput::new(&mut address_buffer, "temp", "temp");
        let num_blocks = (size + block_mask) >> shift;
        let mut writer = DirectMonotonicWriter::get_instance(
            meta,
            &mut address_output,
            num_blocks,
            Lucene90DocValuesFormat::DIRECT_MONOTONIC_BLOCK_SHIFT,
        )?;

        let mut previous = BytesRefBuilder::new();
        let mut ord: i64 = 0;
        let mut start = data.get_file_pointer();
        let mut max_length = 0;
        let mut max_block_length = 0;
        let mut iterator = values.terms_enum()?;

        let mut ht = HashTableEnum::Fast(FastCompressionHashTable::default());
        let terms_dict_buffer = vec![0u8; 1 << 14];
        let mut buffered_output = ByteArrayDataOutput::with_bytes(terms_dict_buffer);
        let mut dict_length = 0;

        while let Some(term) = iterator.next()? {
            if (ord & block_mask) == 0 {
                if ord != 0 {
                    let uncompressed_length = Self::compress_and_get_terms_dict_block_length(
                        &mut buffered_output,
                        dict_length,
                        &mut ht,
                        data,
                    )?;
                    max_block_length = max_block_length.max(uncompressed_length);
                    buffered_output.reset()?;
                }

                writer.add(data.get_file_pointer() - start)?;
                // Write the first term both to the index output, and to the buffer where we'll use it as a
                // dictionary for compression
                data.write_vint(term.length)?;
                data.write_bytes_range(&term.bytes, term.offset, term.length)?;
                Self::maybe_grow_buffer(&mut buffered_output, term.length)?;
                buffered_output.write_bytes_range(&term.bytes, term.offset, term.length)?;
                dict_length = term.length;
            } else {
                let prefix_length =
                    StringHelper::bytes_difference(previous.get_bytes_ref(), &term)?;
                let suffix_length = term.length - prefix_length;
                debug_assert!(suffix_length > 0);
                // Will write (suffixLength + 1 byte + 2 vint) bytes. Grow the buffer in need.
                Self::maybe_grow_buffer(&mut buffered_output, suffix_length + 11)?;
                buffered_output.write_byte(
                    ((prefix_length.min(15)) | ((suffix_length - 1).min(15) << 4)) as u8,
                )?;
                if prefix_length >= 15 {
                    buffered_output.write_vint(prefix_length - 15)?;
                }
                if suffix_length >= 16 {
                    buffered_output.write_vint(suffix_length - 16)?;
                }
                buffered_output.write_bytes_range(
                    &term.bytes,
                    term.offset + prefix_length,
                    suffix_length,
                )?;
            }

            max_length = max_length.max(term.length);
            previous.copy_bytes_with_ref(&term)?;
            ord += 1;
        }
        // Compress and write out the last block
        if buffered_output.get_position() > dict_length {
            let uncompressed_length = Self::compress_and_get_terms_dict_block_length(
                &mut buffered_output,
                dict_length,
                &mut ht,
                data,
            )?;
            max_block_length = max_block_length.max(uncompressed_length);
        }

        writer.finish()?;
        meta.write_int(max_length)?;
        // Write one more int for storing max block length.
        meta.write_int(max_block_length)?;
        meta.write_long(start)?;
        meta.write_long(data.get_file_pointer() - start)?;
        start = data.get_file_pointer();
        address_buffer.copy_to(data)?;
        meta.write_long(start)?;
        meta.write_long(data.get_file_pointer() - start)?;

        self.write_terms_index(values)?;
        Ok(())
    }

    fn compress_and_get_terms_dict_block_length(
        buffered_output: &mut ByteArrayDataOutput,
        dict_length: i32,
        ht: &mut HashTableEnum,
        data: &mut O,
    ) -> Result<i32> {
        let uncompressed_length = buffered_output.get_position() - dict_length;
        data.write_vint(uncompressed_length)?;
        LZ4::compress_with_dictionary(
            CommonUtil::take_and_reset(&mut buffered_output.bytes, |old| vec![0u8; old.len()]),
            0,
            dict_length,
            uncompressed_length,
            data,
            ht,
        )?;
        Ok(uncompressed_length)
    }

    fn maybe_grow_buffer(
        buffered_output: &mut ByteArrayDataOutput,
        term_length: i32,
    ) -> Result<()> {
        let pos = buffered_output.get_position();
        let terms_dict_buffer = &mut buffered_output.bytes;
        debug_assert!(terms_dict_buffer.len() <= i32::MAX as usize);
        let original_length = terms_dict_buffer.len() as i32;
        if pos + term_length >= original_length - 1 {
            ArrayUtil::grow_with_len(terms_dict_buffer, original_length + term_length)?;
            debug_assert!(terms_dict_buffer.len() <= i32::MAX as usize);
            let terms_dict_buffer_len = terms_dict_buffer.len() as i32;
            buffered_output.reset_with_range(pos, terms_dict_buffer_len - pos)?;
        }
        Ok(())
    }

    fn write_terms_index<I: IndexInput>(
        &mut self,
        values: &mut impl SortedSetDocValues<I>,
    ) -> Result<()> {
        let size = values.get_value_count()?;
        self.meta
            .write_int(Lucene90DocValuesFormat::TERMS_DICT_REVERSE_INDEX_SHIFT)?;
        let start = self.data.get_file_pointer();

        let num_blocks = 1
            + ((size + Lucene90DocValuesFormat::TERMS_DICT_REVERSE_INDEX_MASK as i64)
                >> Lucene90DocValuesFormat::TERMS_DICT_REVERSE_INDEX_SHIFT);

        let mut address_buffer = ByteBuffersDataOutput::new();
        let mut writer;

        {
            let mut address_output =
                ByteBuffersIndexOutput::new(&mut address_buffer, "temp", "temp");
            writer = DirectMonotonicWriter::get_instance(
                &mut self.meta,
                &mut address_output,
                num_blocks,
                Lucene90DocValuesFormat::DIRECT_MONOTONIC_BLOCK_SHIFT,
            )?;

            let mut iterator = values.terms_enum()?;
            let mut previous = BytesRefBuilder::new();
            let mut offset: i64 = 0;
            let mut ord: i64 = 0;

            while let Some(term) = iterator.next()? {
                if (ord & Lucene90DocValuesFormat::TERMS_DICT_REVERSE_INDEX_MASK as i64) == 0 {
                    writer.add(offset)?;
                    let sort_key_length = if ord == 0 {
                        0
                    } else {
                        StringHelper::sort_key_length(previous.get_bytes_ref(), &term)?
                    };
                    offset += sort_key_length as i64;
                    self.data
                        .write_bytes_range(&term.bytes, term.offset, sort_key_length)?;
                } else if (ord & Lucene90DocValuesFormat::TERMS_DICT_REVERSE_INDEX_MASK as i64)
                    == Lucene90DocValuesFormat::TERMS_DICT_REVERSE_INDEX_MASK as i64
                {
                    previous.copy_bytes_with_ref(&term)?;
                }
                ord += 1;
            }

            writer.add(offset)?;
            writer.finish()?;

            self.meta.write_long(start)?;
            self.meta.write_long(self.data.get_file_pointer() - start)?;

            let start = self.data.get_file_pointer();
            address_buffer.copy_to(&mut self.data)?;
            self.meta.write_long(start)?;
            self.meta.write_long(self.data.get_file_pointer() - start)?;
        }
        Ok(())
    }
}

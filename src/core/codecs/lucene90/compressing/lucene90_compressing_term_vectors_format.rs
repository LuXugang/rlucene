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
use crate::core::codecs::compressing::lucene90_compressing_term_vectors_reader::Lucene90CompressingTermVectorsReader;
use crate::core::codecs::compressing::lucene90_compressing_term_vectors_writer::Lucene90CompressingTermVectorsWriter;
use crate::core::codecs::compression::compression_mode::CompressionModeEnum;
use crate::core::codecs::term_vectors_format::TermVectorsFormat;

use crate::core::codecs::term_vectors_reader::TermVectorsReaderType;
use crate::core::codecs::term_vectors_writer::TermVectorsWriterEnum;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::fmt;
use std::sync::Arc;

/// A [`TermVectorsFormat`] that compresses chunks of documents together in order to improve the compression ratio.
pub struct Lucene90CompressingTermVectorsFormat {
    format_name: String,
    segment_suffix: String,
    compression_mode: CompressionModeEnum,
    chunk_size: i32,
    max_docs_per_chunk: i32,
    block_size: i32,
}

impl Lucene90CompressingTermVectorsFormat {
    /// Creates a new [`Lucene90CompressingTermVectorsFormat`].
    ///
    /// `format_name` is the name of the format. This name will be used in the file
    /// formats to perform [`CodecUtil::check_index_header`](crate::core::codecs::codec_util::CodecUtil::check_index_header) checks.
    ///
    /// The `compression_mode` parameter allows you to choose between compression
    /// algorithms that have various compression and decompression speeds so that you can
    /// pick the one that best fits your indexing and searching throughput.  
    /// You should **never** instantiate two [`Lucene90CompressingTermVectorsFormat`]s
    /// that have the same name but different [`CompressionMode`](crate::core::codecs::compression::compression_mode::CompressionMode)s.
    ///
    /// `chunk_size` is the minimum byte size of a chunk of documents.  
    /// Higher values of `chunk_size` should improve the compression ratio but will require
    /// more memory at indexing time and might make document loading a little slower (depending
    /// on the size of your OS cache compared to the size of your index).
    ///
    /// - `format_name`: The name of the [`StoredFieldsFormat`](crate::core::codecs::stored_fields_format::StoredFieldsFormat)
    /// - `segment_suffix`: A suffix to append to files created by this format
    /// - `compression_mode`: The [`CompressionMode`](crate::core::codecs::compression::compression_mode::CompressionMode) to use
    /// - `chunk_size`: The minimum number of bytes of a single chunk of stored documents
    /// - `max_docs_per_chunk`: The maximum number of documents in a single chunk
    /// - `block_size`: The number of chunks to store in an index block
    ///
    /// See also: [`CompressionMode`](crate::core::codecs::compression::compression_mode::CompressionMode)
    pub fn new(
        format_name: &str,
        segment_suffix: &str,
        compression_mode: CompressionModeEnum,
        chunk_size: i32,
        max_docs_per_chunk: i32,
        block_size: i32,
    ) -> Result<Self> {
        if chunk_size < 1 {
            return Err(LuceneError::illegal_argument(
                "chunk_size must be >= 1".to_string(),
            ));
        }
        if block_size < 1 {
            return Err(LuceneError::illegal_argument(
                "block_size must be >= 1".to_string(),
            ));
        }

        Ok(Self {
            format_name: format_name.to_string(),
            segment_suffix: segment_suffix.to_string(),
            compression_mode,
            chunk_size,
            max_docs_per_chunk,
            block_size,
        })
    }
}

impl TermVectorsFormat for Lucene90CompressingTermVectorsFormat {
    fn vectors_reader<D1, D2>(
        &self,
        directory: &D1,
        segment_info: &SegmentInfo<D2>,
        field_infos: Arc<FieldInfos>,
        context: &IOContext,
    ) -> Result<TermVectorsReaderType<D1::IndexInput>>
    where
        D1: Directory,
        D2: Directory,
    {
        Lucene90CompressingTermVectorsReader::new(
            directory,
            segment_info,
            &self.segment_suffix,
            field_infos,
            context,
            &self.format_name,
            self.compression_mode.clone(),
        )
    }

    fn vectors_writer<D1, D2>(
        &self,
        directory: &D1,
        segment_info: &SegmentInfo<D2>,
        context: &IOContext,
    ) -> Result<TermVectorsWriterEnum<D1>>
    where
        D1: Directory,
        D2: Directory,
    {
        Ok(TermVectorsWriterEnum::Lucene90(
            Lucene90CompressingTermVectorsWriter::new(
                directory,
                segment_info,
                &self.segment_suffix,
                context,
                &self.format_name,
                self.compression_mode.clone(),
                self.chunk_size,
                self.max_docs_per_chunk,
                self.block_size,
            )?,
        ))
    }
}

impl fmt::Display for Lucene90CompressingTermVectorsFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Lucene90CompressingTermVectorsFormat(compressionMode={}, chunkSize={}, maxDocsPerChunk={}, blockSize={})",
            self.compression_mode, self.chunk_size, self.max_docs_per_chunk, self.block_size
        )
    }
}

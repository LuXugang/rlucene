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
use crate::codecs::compressing::lucene90_compressing_term_vectors_reader::Lucene90CompressingTermVectorsReader;
use crate::codecs::compressing::lucene90_compressing_term_vectors_writer::Lucene90CompressingTermVectorsWriter;
use crate::codecs::compression::compression_mode::CompressionModeEnum;
use crate::codecs::term_vectors_format::TermVectorsFormat;
use crate::codecs::term_vectors_reader::TermVectorsReaderEnum;
use crate::codecs::term_vectors_writer::TermVectorsWriterEnum;
use crate::index::field_infos::FieldInfos;
use crate::index::segment_info::SegmentInfo;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::fmt;
use std::rc::Rc;
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
    /// formats to perform [`CodecUtil::check_index_header`](crate::codecs::codec_util::CodecUtil::check_index_header) checks.
    ///
    /// The `compression_mode` parameter allows you to choose between compression
    /// algorithms that have various compression and decompression speeds so that you can
    /// pick the one that best fits your indexing and searching throughput.  
    /// You should **never** instantiate two [`Lucene90CompressingTermVectorsFormat`]s
    /// that have the same name but different [`CompressionMode`](crate::codecs::compression::compression_mode::CompressionMode)s.
    ///
    /// `chunk_size` is the minimum byte size of a chunk of documents.  
    /// Higher values of `chunk_size` should improve the compression ratio but will require
    /// more memory at indexing time and might make document loading a little slower (depending
    /// on the size of your OS cache compared to the size of your index).
    ///
    /// - `format_name`: The name of the [`StoredFieldsFormat`](crate::codecs::stored_fields_format::StoredFieldsFormat)
    /// - `segment_suffix`: A suffix to append to files created by this format
    /// - `compression_mode`: The [`CompressionMode`](crate::codecs::compression::compression_mode::CompressionMode) to use
    /// - `chunk_size`: The minimum number of bytes of a single chunk of stored documents
    /// - `max_docs_per_chunk`: The maximum number of documents in a single chunk
    /// - `block_size`: The number of chunks to store in an index block
    ///
    /// See also: [`CompressionMode`](crate::codecs::compression::compression_mode::CompressionMode)
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
        directory: &mut D1,
        segment_info: Rc<SegmentInfo<D2>>,
        field_infos: Rc<FieldInfos>,
        context: &IOContext,
    ) -> Result<TermVectorsReaderEnum<D1::IndexInputType>>
    where
        D1: Directory,
        D2: Directory,
    {
        Ok(TermVectorsReaderEnum::Lucene90(
            Lucene90CompressingTermVectorsReader::new(
                directory,
                &*segment_info,
                &self.segment_suffix,
                field_infos,
                context,
                &self.format_name,
                self.compression_mode.clone(),
            )?,
        ))
    }

    fn vectors_writer<D1, D2>(
        &self,
        directory: Arc<Mutex<D1>>,
        segment_info: Rc<SegmentInfo<D2>>,
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
            self.compression_mode,
            self.chunk_size,
            self.max_docs_per_chunk,
            self.block_size
        )
    }
}

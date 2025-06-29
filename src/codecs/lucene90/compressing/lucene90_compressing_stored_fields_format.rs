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
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::codecs::compressing::lucene90_compressing_stored_fields_reader::Lucene90CompressingStoredFieldsReader;
use crate::codecs::compressing::lucene90_compressing_stored_fields_writer::Lucene90CompressingStoredFieldsWriter;
use crate::codecs::compression::compression_mode::CompressionModeEnum;
use crate::codecs::stored_fields_format::StoredFieldsFormat;
use crate::codecs::stored_fields_reader::StoredFieldsReaderEnum;
use crate::codecs::stored_fields_writer::StoredFieldsWriterEnum;
use crate::index::field_infos::FieldInfos;
use crate::index::segment_info::SegmentInfo;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use crate::util::packed::direct_monotonic_writer::direct_monotonic_writer_util;

/// A [`StoredFieldsFormat`] that compresses documents in chunks in order to
/// improve the compression ratio.
///
/// For a chunk size of *chunkSize* bytes, this [`StoredFieldsFormat`] does not
/// support documents larger than (`2^31 - chunkSize`) bytes.
///
/// For optimal performance, you should use a
/// [`MergePolicy`](crate::index::merge_policy::MergePolicy) that returns
/// segments that have the biggest byte size first.
pub struct Lucene90CompressingStoredFieldsFormat {
    format_name: String,
    segment_suffix: String,
    compression_mode: CompressionModeEnum,
    chunk_size: i32,
    max_docs_per_chunk: i32,
    block_shift: i32,
}
impl Lucene90CompressingStoredFieldsFormat {
    /// Create a new [`Lucene90CompressingStoredFieldsFormat`] with an empty
    /// segment suffix.
    pub fn new(
        format_name: &str,
        compression_mode: CompressionModeEnum,
        chunk_size: i32,
        max_docs_per_chunk: i32,
        block_shift: i32,
    ) -> Result<Self> {
        Self::new_with_suffix(
            format_name,
            "",
            compression_mode,
            chunk_size,
            max_docs_per_chunk,
            block_shift,
        )
    }
    /// Create a new [`Lucene90CompressingStoredFieldsFormat`].
    ///
    /// - `format_name` is the name of the format. This name will be used in the
    ///   file formats to perform
    ///   [`CodecUtil::check_index_header`](crate::codecs::codec_util::CodecUtil::check_index_header)
    ///   header checks.
    /// - `segment_suffix` is the segment suffix. This suffix is added to the
    ///   result file name only if it's not the empty string.
    /// - The `compression_mode` parameter allows you to choose between
    ///   compression algorithms that have various compression and decompression
    ///   speeds so that you can pick the one that best fits your indexing and
    ///   searching throughput. You should never instantiate two
    ///   [`Lucene90CompressingStoredFieldsFormat`]s that have the same name but
    ///   different [`CompressionMode`](crate::codecs::compression::compression_mode::CompressionMode)s.
    /// - `chunk_size` is the minimum byte size of a chunk of documents. A value
    ///   of `1` can make sense if there is redundancy across fields.
    /// - `max_docs_per_chunk` is an upper bound on how many docs may be stored
    ///   in a single chunk. This is to bound the CPU costs for highly
    ///   compressible data.
    ///
    /// Higher values of `chunk_size` should improve the compression ratio but
    /// will require more memory at indexing time and might make document
    /// loading a little slower (depending on the size of your OS cache
    /// compared to the size of your index).
    ///
    /// - `format_name`: the name of the [`StoredFieldsFormat`]
    /// - `compression_mode`: the
    ///   [`CompressionMode`](crate::codecs::compression::compression_mode::CompressionMode)
    ///   to use
    /// - `chunk_size`: the minimum number of bytes of a single chunk of stored
    ///   documents
    /// - `max_docs_per_chunk`: the maximum number of documents in a single
    ///   chunk
    /// - `block_shift`: the log in base 2 of number of chunks to store in an
    ///   index block
    ///
    /// See [`CompressionMode`](crate::codecs::compression::compression_mode::CompressionMode).
    pub fn new_with_suffix(
        format_name: &str,
        segment_suffix: &str,
        compression_mode: CompressionModeEnum,
        chunk_size: i32,
        max_docs_per_chunk: i32,
        block_shift: i32,
    ) -> Result<Self> {
        if chunk_size < 1 {
            return Err(LuceneError::illegal_argument(
                "chunk_size must be >= 1".to_string(),
            ));
        }
        if max_docs_per_chunk < 1 {
            return Err(LuceneError::illegal_argument(
                "max_docs_per_chunk must be >= 1".to_string(),
            ));
        }
        if !(direct_monotonic_writer_util::MIN_BLOCK_SHIFT
            ..=direct_monotonic_writer_util::MAX_BLOCK_SHIFT)
            .contains(&block_shift)
        {
            return Err(LuceneError::illegal_argument(format!(
                "block_shift must be in {}-{}, got {}",
                direct_monotonic_writer_util::MIN_BLOCK_SHIFT,
                direct_monotonic_writer_util::MAX_BLOCK_SHIFT,
                block_shift
            )));
        }

        Ok(Self {
            format_name: format_name.to_string(),
            segment_suffix: segment_suffix.to_string(),
            compression_mode,
            chunk_size,
            max_docs_per_chunk,
            block_shift,
        })
    }
}
impl StoredFieldsFormat for Lucene90CompressingStoredFieldsFormat {
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
        Ok(StoredFieldsReaderEnum::Lucene90(
            Lucene90CompressingStoredFieldsReader::new(
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
        Ok(StoredFieldsWriterEnum::Lucene90(
            Lucene90CompressingStoredFieldsWriter::new(
                directory,
                segment_info,
                &self.segment_suffix,
                context,
                &self.format_name,
                self.compression_mode.clone(),
                self.chunk_size,
                self.max_docs_per_chunk,
                self.block_shift,
            )?,
        ))
    }
}
impl fmt::Display for Lucene90CompressingStoredFieldsFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Lucene90CompressingStoredFieldsFormat(compressionMode={}, chunkSize={}, maxDocsPerChunk={}, blockShift={})",
            self.compression_mode,
            self.chunk_size,
            self.max_docs_per_chunk,
            self.block_shift
        )
    }
}

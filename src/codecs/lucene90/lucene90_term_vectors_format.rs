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
use crate::codecs::compressing::lucene90_compressing_term_vectors_format::Lucene90CompressingTermVectorsFormat;
use crate::codecs::compression::compression_mode::{CompressionModeEnum, LZ4FastCompressionMode};
use crate::codecs::term_vectors_format::TermVectorsFormat;
use crate::codecs::term_vectors_reader::TermVectorsReaderEnum;
use crate::codecs::term_vectors_writer::TermVectorsWriterEnum;
use crate::index::field_infos::FieldInfos;
use crate::index::segment_info::SegmentInfo;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::fmt::Display;
use std::rc::Rc;
use std::sync::Arc;
/// Lucene 9.0 TermVectorsFormat.
///
/// Very similarly to Lucene90StoredFieldsFormat, this format is based on compressed
/// chunks of data with document-level granularity so that a document can never span across
/// distinct chunks. Moreover, data is made as compact as possible:
/// - Textual data is compressed using the very light [LZ4](http://code.google.com/p/lz4/) algorithm
/// - Binary data is written using fixed-size blocks of [`PackedInts`](crate::util::packed::PackedInts)
///
/// Term vectors are stored using two files:
/// - A data file where terms, frequencies, positions, offsets and payloads are stored
/// - An index file, loaded into memory, used to locate specific documents in the data file
///
/// Looking up term vectors for any document requires at most one disk seek.
///
/// **File formats**
///
/// 1. **Vector metadata file** (`.tvm`):
///    - VectorMeta (.tvm) → <Header>, PackedIntsVersion, ChunkSize, ChunkIndexMetadata, ChunkCount, DirtyChunkCount, DirtyDocsCount, Footer
///    -  Header → [IndexHeader](crate::codecs::codec_util::CodecUtil::write_index_header)
///    -  PackedIntsVersion, ChunkSize → [`VInt`](crate::store::data_output::DataOutput::write_vint)
///    -  ChunkCount, DirtyChunkCount, DirtyDocsCount → [`Vlong`](crate::store::data_output::DataOutput::write_vlong)
///    -  ChunkIndexMetadata → [`FieldsIndexWriter`](crate::codecs::lucene90::fields_index_writer::FieldsIndexWriter)
///    -  Footer → [`CodecFooter`](crate::codecs::codec_util::CodecUtil::write_footer)
///    - **Notes:**
///      -  PackedIntsVersion is [`PackedInts::VERSION_CURRENT`](crate::util::packed::PackedInts::VERSION_CURRENT)
///      - ChunkSize is the number of bytes of terms to accumulate before flushing
///      - ChunkCount is not known in advance and is the number of chunks necessary for the segment
///      - DirtyChunkCount is the number of prematurely flushed chunks in the `.tvd` file
///
/// 2. **Vector data file** (`.tvd`):
///    - Stores terms, frequencies, positions, offsets and payloads for every document
///    - Accumulates data in memory until the buffer grows beyond 4 KB, then flushes using LZ4 for terms/payloads and `BlockPackedWriter` for positions
///    - **Detailed format**:
///      - VectorData (.tvd) → <Header>, <Chunk>^ChunkCount, Footer
///      - Header → [IndexHeader](crate::codecs::codec_util::CodecUtil::write_index_header)
///      - Chunk → DocBase, ChunkDocs, <NumFields>, <FieldNums>, <FieldNumOffs>, <Flags>, <NumTerms>, <TermLengths>, <TermFreqs>, <Positions>, <StartOffsets>, <Lengths>, <PayloadLengths>, <TermAndPayloads>
///      - NumFields → DocNumFields^ChunkDocs
///      - FieldNums → FieldNumDelta^TotalDistinctFields
///      - Flags → Bit <FieldFlags>
///      - FieldFlags → either Flag^TotalDistinctFields or Flag^TotalFields
///      - NumTerms → FieldNumTerms^TotalFields
///      - TermLengths → PrefixLength^TotalTerms, SuffixLength^TotalTerms
///      - TermFreqs → TermFreqMinus1^TotalTerms
///      - Positions → PositionDelta^TotalPositions
///      - StartOffsets → (AvgCharsPerTerm^TotalDistinctFields), StartOffsetDelta^TotalOffsets
///      - Lengths → LengthMinusTermLength^TotalOffsets
///      - PayloadLengths → PayloadLength^TotalPayloads
///      - TermAndPayloads → LZ4-compressed representation of <FieldTermsAndPayloads>^TotalFields
///      - <FieldTermsAndPayloads> → Terms (Payloads)
///      - DocBase, ChunkDocs, DocNumFields → [`VInt`](crate::store::data_output::DataOutput::write_vint)
///      - AvgCharsPerTerm → [`Int`](crate::store::data_output::DataOutput::write_int)
///      - DocNumFields (≥1), FieldNumOffs → PackedInts array
///      - FieldNumTerms, PrefixLength, SuffixLength, TermFreqMinus1, PositionDelta, StartOffsetDelta, LengthMinusTermLength, PayloadLength
///      - Footer → [`CodecFooter`](crate::codecs::codec_util::CodecUtil::write_footer)
///    - **Notes:**
///      - DocBase is the ID of the first doc in the chunk
///      - ChunkDocs is the number of documents in the chunk
///      - DocNumFields is the number of fields per document
///      - FieldNums is a delta-encoded list of sorted unique field numbers in the chunk
///      - FieldNumOffs is the array of offsets into FieldNums
///      - FieldNumTerms is the number of terms per field
///      - PrefixLength is 0 for the first term of a field, otherwise the shared prefix length with the previous term
///      - SuffixLength = term length – PrefixLength
///      - TermFreqMinus1 = frequency – 1
///      - PositionDelta = absolute for first position, delta thereafter
///      - StartOffsetDelta = startOffset – previousStartOffset – AvgCharsPerTerm × PositionDelta
///      - LengthMinusTermLength = endOffset – startOffset – termLength
///      - AvgCharsPerTerm is encoded as a 4-byte float (only if positions & offsets are enabled)
///      - PayloadLength encodes payload length
///      - TotalPayloads is sum of payload counts across fields
///
/// 3. **Vector index file** (`.tvx`):
///    -  VectorIndex (.tvx) → <Header>, <ChunkIndex>, Footer
///    -  Header → [IndexHeader](crate::codecs::codec_util::CodecUtil::write_index_header)
///    -  ChunkIndex → [`FieldsIndexWriter`](crate::codecs::lucene90::fields_index_writer::FieldsIndexWriter)
///    -  Footer → [`CodecFooter`](crate::codecs::codec_util::CodecUtil::write_footer)
pub struct Lucene90TermVectorsFormat {
    base: Lucene90CompressingTermVectorsFormat,
}
impl Default for Lucene90TermVectorsFormat {
    fn default() -> Self {
        Self::new()
    }
}

impl Lucene90TermVectorsFormat {
    pub fn new() -> Self {
        Self {
            base: Lucene90CompressingTermVectorsFormat::new(
                "Lucene90TermVectorsData",
                "",
                CompressionModeEnum::Fast(LZ4FastCompressionMode),
                1 << 12,
                128,
                10,
            )
            .unwrap(),
        }
    }
}
impl TermVectorsFormat for Lucene90TermVectorsFormat {
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
        self.base
            .vectors_reader(directory, segment_info, field_infos, context)
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
        self.base.vectors_writer(directory, segment_info, context)
    }
}
impl Display for Lucene90TermVectorsFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Lucene90TermVectorsFormat<{}>", self.base)
    }
}

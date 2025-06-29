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
use crate::codecs::lucene90::lucene90_norms_consumer::Lucene90NormsConsumer;
use crate::codecs::lucene90_norms_producer::Lucene90NormsProducer;
use crate::codecs::norms_consumer::NormsConsumerEnum;
use crate::codecs::norms_format::NormsFormat;
use crate::codecs::norms_producer::NormsProducerEnum;
use crate::index::segment_read_state::SegmentReadState;
use crate::index::segment_write_state::SegmentWriteState;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
/// Lucene 9.0 Score normalization format.
///
/// Encodes normalization values using the minimum number of bytes needed to
/// represent the range (which can be zero).
///
/// # Files
///
/// - `.nvd`: Norms data
/// - `.nvm`: Norms metadata
///
/// ## `.nvm` - Norms metadata file
///
/// For each norms field, stores metadata such as the offset into the norms data
/// (`.nvd`).
///
/// Format:
///
/// ```text
/// Norms metadata (.nvm) --> Header, <Entry> * NumFields, Footer
/// ```
///
/// - **Header** →
///   [`IndexHeader`](crate::codecs::codec_util::CodecUtil::write_header)
/// - **Entry** →
///     - FieldNumber
///       [`(Int32)`](crate::store::data_output::DataOutput::write_int)
///     - DocsWithFieldAddress
///       [`(Int64)`](crate::store::data_output::DataOutput::write_long)
///     - DocsWithFieldLength
///       [`(Int64)`](crate::store::data_output::DataOutput::write_long)
///     - NumDocsWithField
///       [`(Int32)`](crate::store::data_output::DataOutput::write_int)
///     - BytesPerNorm
///       [`(byte)`](crate::store::data_output::DataOutput::write_byte)
///     - NormsAddress
///       [`(Int64)`](crate::store::data_output::DataOutput::write_long)
/// - **Footer** →
///   [`CodecFooter`](crate::codecs::codec_util::CodecUtil::write_footer)
///
/// Notes:
///
/// - A `FieldNumber` of `-1` indicates the end of metadata.
/// - `NormsAddress` points to the start of the norm values in `.nvd`, or to the
///   SINGLETON value if `BytesPerNorm == 0`.   If `BytesPerNorm != 0`, there
///   are `NumDocsWithField` values to read at that offset.
/// - `DocsWithFieldAddress` points to the start of the bit set representing
///   documents with norms:
///     - `-2`: no documents have a norm
///     - `-1`: all documents have a norm
/// - `DocsWithFieldLength` is the byte length used to encode the set of
///   documents with a norm.
///
/// ## `.nvd` - Norms data file
///
/// For each norms field, this stores the actual per-document values.
///
/// Format:
///
/// ```text
/// Norms data (.nvd) --> Header, <Data> * NumFields, Footer
/// ```
///
/// - **Header** →
///   [`IndexHeader`](crate::codecs::codec_util::CodecUtil::write_header)
/// - **DocsWithFieldData** → [`BitSet of MaxDoc
///   documents`](crate::codecs::indexed_disi::IndexedDISI)
/// - **NormsData** →
///   [`byte`](crate::store::data_output::DataOutput::write_byte) *
///   (`NumDocsWithField` × `BytesPerValue`)
/// - **Footer** →
///   [`CodecFooter`](crate::codecs::codec_util::CodecUtil::write_footer)
pub struct Lucene90NormsFormat;
impl Lucene90NormsFormat {
    const DATA_CODEC: &'static str = "Lucene90NormsData";
    const DATA_EXTENSION: &'static str = "nvd";
    const METADATA_CODEC: &'static str = "Lucene90NormsMetadata";
    const METADATA_EXTENSION: &'static str = "nvm";
    pub(crate) const VERSION_START: i32 = 0;
    pub(crate) const VERSION_CURRENT: i32 = Self::VERSION_START;
}
impl NormsFormat for Lucene90NormsFormat {
    fn norms_consumer<D>(
        &self,
        state: &SegmentWriteState<D>,
    ) -> Result<NormsConsumerEnum<D::IndexOutputType>>
    where
        D: Directory,
    {
        let norms_consumer = Lucene90NormsConsumer::new(
            state,
            Self::DATA_CODEC,
            Self::DATA_EXTENSION,
            Self::METADATA_CODEC,
            Self::METADATA_EXTENSION,
        )?;
        Ok(NormsConsumerEnum::Lucene90(norms_consumer))
    }

    fn norms_producer<D>(
        &self,
        state: &SegmentReadState<D>,
    ) -> Result<NormsProducerEnum<D::IndexInputType>>
    where
        D: Directory,
    {
        let norms_producer = Lucene90NormsProducer::new(
            state,
            Self::DATA_CODEC,
            Self::DATA_EXTENSION,
            Self::METADATA_CODEC,
            Self::METADATA_EXTENSION,
        )?;
        Ok(NormsProducerEnum::Lucene90(norms_producer))
    }
}

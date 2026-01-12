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
use crate::core::codecs::lucene90::lucene90_norms_consumer::Lucene90NormsConsumer;
use crate::core::codecs::lucene90_norms_producer::Lucene90NormsProducer;
use crate::core::codecs::norms_format::NormsFormat;
use crate::core::codecs::norms_producer::NormsProducerType;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::error::lucene_error::Result;
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
///   [`IndexHeader`](crate::core::codecs::codec_util::CodecUtil::write_header)
/// - **Entry** →
///     - FieldNumber
///       [`(Int32)`](crate::core::store::data_output::DataOutput::write_int)
///     - DocsWithFieldAddress
///       [`(Int64)`](crate::core::store::data_output::DataOutput::write_long)
///     - DocsWithFieldLength
///       [`(Int64)`](crate::core::store::data_output::DataOutput::write_long)
///     - NumDocsWithField
///       [`(Int32)`](crate::core::store::data_output::DataOutput::write_int)
///     - BytesPerNorm
///       [`(byte)`](crate::core::store::data_output::DataOutput::write_byte)
///     - NormsAddress
///       [`(Int64)`](crate::core::store::data_output::DataOutput::write_long)
/// - **Footer** →
///   [`CodecFooter`](crate::core::codecs::codec_util::CodecUtil::write_footer)
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
///   [`IndexHeader`](crate::core::codecs::codec_util::CodecUtil::write_header)
/// - **DocsWithFieldData** → [`BitSet of MaxDoc
///   documents`](crate::core::codecs::indexed_disi::IndexedDISI)
/// - **NormsData** →
///   [`byte`](crate::core::store::data_output::DataOutput::write_byte) *
///   (`NumDocsWithField` × `BytesPerValue`)
/// - **Footer** →
///   [`CodecFooter`](crate::core::codecs::codec_util::CodecUtil::write_footer)
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
    type NormsConsumer<T: IndexOutput> = Lucene90NormsConsumer<T>;

    fn norms_consumer<D1, D2>(
        &self,
        state: &SegmentWriteState<D1>,
        segment_info: &SegmentInfo<D2>,
    ) -> Result<Self::NormsConsumer<D1::IndexOutput>>
    where
        D1: Directory,
        D2: Directory,
    {
        let norms_consumer = Lucene90NormsConsumer::new(
            state,
            Self::DATA_CODEC,
            Self::DATA_EXTENSION,
            Self::METADATA_CODEC,
            Self::METADATA_EXTENSION,
            segment_info,
        )?;
        Ok(norms_consumer)
    }

    type NormsProducer<T: IndexInput> = Lucene90NormsProducer<T>;

    fn norms_producer<D1, D2>(
        &self,
        state: &SegmentReadState<D1>,
        segment_info: &SegmentInfo<D2>,
    ) -> Result<NormsProducerType<D1::IndexInput>>
    where
        D1: Directory,
        D2: Directory,
    {
        let norms_producer = Lucene90NormsProducer::new(
            state,
            Self::DATA_CODEC,
            Self::DATA_EXTENSION,
            Self::METADATA_CODEC,
            Self::METADATA_EXTENSION,
            segment_info,
        )?;
        Ok(norms_producer)
    }
}

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
use once_cell::sync::Lazy;

use crate::codecs::compound_format::CompoundFormat;
use crate::codecs::doc_values_format::DocValuesFormat;
use crate::codecs::field_infos_format::FieldInfosFormat;
use crate::codecs::live_docs_format::LiveDocsFormat;
use crate::codecs::lucene101_codec::Lucene101Codec;
use crate::codecs::norms_format::NormsFormat;
use crate::codecs::points_format::PointsFormat;
use crate::codecs::postings_format::PostingsFormat;
use crate::codecs::segment_info_format::SegmentInfoFormat;
use crate::codecs::stored_fields_format::StoredFieldsFormat;
use crate::codecs::term_vectors_format::TermVectorsFormat;

pub static LATEST_CODEC: Lazy<Lucene101Codec> = Lazy::new(|| Lucene101Codec);
pub trait Codec {
    type PostingsFormat: PostingsFormat;
    type DocValuesFormat: DocValuesFormat;
    type StoredFieldsFormat: StoredFieldsFormat;
    type TermVectorsFormat: TermVectorsFormat;
    type FieldInfosFormat: FieldInfosFormat;
    type SegmentInfoFormat: SegmentInfoFormat;
    type NormsFormat: NormsFormat;
    type LiveDocsFormat: LiveDocsFormat;
    type CompoundFormat: CompoundFormat;
    type PointsFormat: PointsFormat;
    // type KnnVectorsFormat;
    /// Encodes/decodes postings
    fn postings_format(&self) -> Self::PostingsFormat;
    /// Encodes/decodes docvalues
    fn doc_values_format(&self) -> Self::DocValuesFormat;
    //
    /// Encodes/decodes stored fields
    fn stored_fields_format(&self) -> Self::StoredFieldsFormat;
    //
    /// Encodes/decodes term vectors
    fn term_vectors_format(&self) -> Self::TermVectorsFormat;

    /// Encodes/decodes field infos file
    fn field_infos_format(&self) -> Self::FieldInfosFormat;

    /// Encodes/decodes segment info file
    fn segment_info_format(&self) -> Self::SegmentInfoFormat;

    // /// Encodes/decodes document normalization values
    fn norms_format(&self) -> Self::NormsFormat;

    /// Encodes/decodes live docs
    fn live_docs_format(&self) -> Self::LiveDocsFormat;

    /// Encodes/decodes compound files
    fn compound_format(&self) -> Self::CompoundFormat;

    /// Encodes/decodes points index
    fn points_format(&self) -> Self::PointsFormat;

    // /// Encodes/decodes numeric vector fields
    // fn knn_vectors_format(&self) -> &Self::KnnVectorsFormat;

    fn get_name(&self) -> &str;
}
pub fn get_default_code() -> Lucene101Codec {
    let codec = Lucene101Codec;
    debug_assert!(LATEST_CODEC.get_name() == codec.get_name());
    codec
}

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
use crate::codecs::live_docs_format::LiveDocsFormat;
use crate::codecs::lucene101_codec::Lucene101Codec;
use crate::codecs::segment_info_format::SegmentInfoFormat;
use once_cell::sync::Lazy;
pub static LATEST_CODEC: Lazy<Lucene101Codec> = Lazy::new(|| Lucene101Codec);
pub trait Codec {
    // type PostingsFormat;
    // type DocValuesFormat;
    // type StoredFieldsFormat;
    // type TermVectorsFormat;
    // type FieldInfosFormat;
    type SegmentInfoFormat: SegmentInfoFormat;
    // type NormsFormat;
    type LiveDocsFormat: LiveDocsFormat;
    // type CompoundFormat;
    // type PointsFormat;
    // type KnnVectorsFormat;
    // /// Encodes/decodes postings
    // fn postings_format(&self) -> &Self::PostingsFormat;
    //
    // /// Encodes/decodes docvalues
    // fn doc_values_format(&self) -> &Self::DocValuesFormat;
    //
    // /// Encodes/decodes stored fields
    // fn stored_fields_format(&self) -> &Self::StoredFieldsFormat;
    //
    // /// Encodes/decodes term vectors
    // fn term_vectors_format(&self) -> &Self::TermVectorsFormat;
    //
    // /// Encodes/decodes field infos file
    // fn field_infos_format(&self) -> &Self::FieldInfosFormat;

    /// Encodes/decodes segment info file
    fn segment_info_format(&self) -> Self::SegmentInfoFormat;

    // /// Encodes/decodes document normalization values
    // fn norms_format(&self) -> &Self::NormsFormat;

    /// Encodes/decodes live docs
    fn live_docs_format(&self) -> &Self::LiveDocsFormat;

    // /// Encodes/decodes compound files
    // fn compound_format(&self) -> &Self::CompoundFormat;
    //
    // /// Encodes/decodes points index
    // fn points_format(&self) -> &Self::PointsFormat;
    //
    // /// Encodes/decodes numeric vector fields
    // fn knn_vectors_format(&self) -> &Self::KnnVectorsFormat;

    fn get_name(&self) -> &str;
}
pub fn get_default_code() -> Lucene101Codec {
    Lucene101Codec
}

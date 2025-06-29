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
use crate::codecs::lucene101::lucene101_postings_format::Lucene101PostingsFormat;
use crate::codecs::lucene90::lucene90_compound_format::Lucene90CompoundFormat;
use crate::codecs::lucene90_doc_values_format::Lucene90DocValuesFormat;
use crate::codecs::lucene90_live_docs_format::Lucene90LiveDocsFormat;
use crate::codecs::lucene90_norms_format::Lucene90NormsFormat;
use crate::codecs::lucene90_points_format::Lucene90PointsFormat;
use crate::codecs::lucene90_stored_fields_format::Lucene90StoredFieldsFormat;
use crate::codecs::lucene90_term_vectors_format::Lucene90TermVectorsFormat;
use crate::codecs::lucene94::lucene94_field_infos_format::Lucene94FieldInfosFormat;
use crate::codecs::lucene99_segment_info_format::Lucene99SegmentInfoFormat;
use crate::codecs::Codec;

#[derive(Clone)]
pub struct Lucene101Codec;

impl Codec for Lucene101Codec {
    type PostingsFormat = Lucene101PostingsFormat;
    type DocValuesFormat = Lucene90DocValuesFormat;
    type StoredFieldsFormat = Lucene90StoredFieldsFormat;
    type TermVectorsFormat = Lucene90TermVectorsFormat;
    type FieldInfosFormat = Lucene94FieldInfosFormat;
    type SegmentInfoFormat = Lucene99SegmentInfoFormat;
    type NormsFormat = Lucene90NormsFormat;
    type LiveDocsFormat = Lucene90LiveDocsFormat;
    type CompoundFormat = Lucene90CompoundFormat;
    type PointsFormat = Lucene90PointsFormat;

    fn postings_format(&self) -> Self::PostingsFormat {
        Lucene101PostingsFormat::new()
    }

    fn doc_values_format(&self) -> Self::DocValuesFormat {
        Lucene90DocValuesFormat::default()
    }

    fn stored_fields_format(&self) -> Self::StoredFieldsFormat {
        Lucene90StoredFieldsFormat::default()
    }

    fn term_vectors_format(&self) -> Self::TermVectorsFormat {
        Lucene90TermVectorsFormat::default()
    }

    fn field_infos_format(&self) -> Self::FieldInfosFormat {
        Lucene94FieldInfosFormat
    }

    fn segment_info_format(&self) -> Self::SegmentInfoFormat {
        Lucene99SegmentInfoFormat
    }

    fn norms_format(&self) -> Self::NormsFormat {
        Lucene90NormsFormat
    }

    fn live_docs_format(&self) -> Self::LiveDocsFormat {
        Lucene90LiveDocsFormat
    }

    fn compound_format(&self) -> Self::CompoundFormat {
        Lucene90CompoundFormat
    }

    fn points_format(&self) -> Self::PointsFormat {
        Lucene90PointsFormat
    }

    fn get_name(&self) -> &str {
        "Lucene101"
    }
}

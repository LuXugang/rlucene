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
use crate::codecs::lucene90::lucene90_compound_format::Lucene90CompoundFormat;
use crate::codecs::lucene90_doc_values_format::Lucene90DocValuesFormat;
use crate::codecs::lucene90_live_docs_format::Lucene90LiveDocsFormat;
use crate::codecs::lucene90_norms_format::Lucene90NormsFormat;
use crate::codecs::lucene90_stored_fields_format::Lucene90StoredFieldsFormat;
use crate::codecs::lucene94::lucene94_field_infos_format::Lucene94FieldInfosFormat;
use crate::codecs::lucene99_segment_info_format::Lucene99SegmentInfoFormat;
use crate::codecs::Codec;

#[derive(Clone)]
pub struct Lucene101Codec;

impl Codec for Lucene101Codec {
    type DocValuesFormat = Lucene90DocValuesFormat;
    type StoredFieldsFormat = Lucene90StoredFieldsFormat;
    type FieldInfosFormat = Lucene94FieldInfosFormat;
    type SegmentInfoFormat = Lucene99SegmentInfoFormat;
    type NormsFormat = Lucene90NormsFormat;
    type LiveDocsFormat = Lucene90LiveDocsFormat;
    type CompoundFormat = Lucene90CompoundFormat;

    fn doc_values_format(&self) -> Self::DocValuesFormat {
        Lucene90DocValuesFormat::default()
    }

    fn stored_fields_format(&self) -> Self::StoredFieldsFormat {
        Lucene90StoredFieldsFormat::new()
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

    fn get_name(&self) -> &str {
        "Lucene101"
    }
}

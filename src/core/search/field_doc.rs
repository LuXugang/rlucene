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
use crate::core::search::field_comparator::FieldComparatorValue;
use crate::core::search::score_doc::{ScoreDoc, ScoreDocLike};
use std::fmt;

pub type FieldsValue = FieldComparatorValue;
/// Expert: A [`ScoreDoc`] which also contains information about how to sort the referenced document.
/// In addition to the document number and score, this object contains an array of values for the
/// document from the field(s) used to sort. For example, if the sort criteria was to sort by fields
/// `"a"`, `"b"` then `"c"`, the `fields` object array will have three elements, corresponding
/// respectively to the term values for the document in fields `"a"`, `"b"` and `"c"`. The class of
/// each element in the array will be either `Integer`, `Float` or `String` depending on the type of
/// values in the terms of each field.
///
/// See also:
/// - [`ScoreDoc`]
/// - [`TopFieldDocs`](crate::core::search::top_field_docs::TopFieldDocs)
pub struct FieldDoc {
    pub base: ScoreDoc,
    /// Expert: The values which are used to sort the referenced document. The order of these will
    /// match the original sort criteria given by a [`Sort`] object. Each Object will have been returned
    /// from the `value` method corresponding `FieldComparator` used to sort this field.
    ///
    /// See also:
    /// - [`Sort`]
    /// - [`IndexSearcher::search`](`crate::core::search::IndexSearcher::search`)
    pub fields: Vec<FieldsValue>,
}

impl FieldDoc {
    /// Creates one of these objects with empty sort information.
    pub fn new(doc: i32, score: f32) -> Self {
        Self {
            base: ScoreDoc::new(doc, score),
            fields: Vec::new(),
        }
    }

    /// Creates one of these objects with the given sort information.
    pub fn with_fields(doc: i32, score: f32, fields: Vec<FieldsValue>) -> Self {
        Self {
            base: ScoreDoc::new(doc, score),
            fields,
        }
    }

    /// Creates one of these objects with the given sort information and shard_index.
    pub fn with_fields_and_shard_index(
        doc: i32,
        score: f32,
        fields: Vec<FieldsValue>,
        shard_index: i32,
    ) -> Self {
        Self {
            base: ScoreDoc::with_shard_index(doc, score, shard_index),
            fields,
        }
    }
}
impl ScoreDocLike for FieldDoc {
    fn doc(&self) -> i32 {
        self.base.doc
    }

    fn score(&self) -> f32 {
        self.base.score
    }

    fn shard_index(&self) -> i32 {
        self.base.shard_index
    }

    fn convert_score_doc(self) -> ScoreDoc {
        self.base
    }
}

impl fmt::Display for FieldDoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} fields={:?}", self.base, self.fields)
    }
}

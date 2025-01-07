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
use crate::index::index_sorter::{
    DoubleSorter, FloatSorter, IndexSortEnum, IndexSorter, IntSorter, LongSorter, StringSorter,
};
use crate::util::error::lucene_error::LuceneError;

pub struct SortField {
    fields: String,
    field_type: Type,
}

impl SortField {
    /// Creates a sort by terms in the given field with the type of term values explicitly given.
    ///
    /// # Arguments
    /// - `field`: Name of the field to sort by. Can be `None` if `field_type` is `SCORE` or `DOC`.
    /// - `field_type`: Type of values in the terms.
    ///
    /// # Errors
    /// Returns an error if the field is `None` and the type is not `SCORE` or `DOC`.
    pub fn new(field: Option<String>, field_type: Type) -> Result<Self, LuceneError> {
        SortField::init_field_type(field, field_type)
    }
    // Sets field & type, and ensures field is not NULL unless
    // type is SCORE or DOC
    pub fn init_field_type(field: Option<String>, field_type: Type) -> Result<Self, LuceneError> {
        if field.is_none() && field_type != Type::Score && field_type != Type::Doc {
            return Err(LuceneError::illegal_argument(
                "field can only be None when type is SCORE or DOC".to_string(),
            ));
        }
        Ok(Self {
            fields: field.unwrap(),
            field_type,
        })
    }
    pub fn get_index_sorter(&self) -> Option<IndexSortEnum> {
        match self.field_type {
            Type::Int => Some(IndexSortEnum::ISorter(IntSorter {
                provider_name: Provider::NAME.to_string(),
            })),
            Type::Float => Some(IndexSortEnum::FSorter(FloatSorter {
                provider_name: Provider::NAME.to_string(),
            })),
            Type::Long => Some(IndexSortEnum::LSorter(LongSorter {
                provider_name: Provider::NAME.to_string(),
            })),
            Type::Double => Some(IndexSortEnum::DSorter(DoubleSorter {
                provider_name: Provider::NAME.to_string(),
            })),
            Type::String => Some(IndexSortEnum::SSorter(StringSorter {
                provider_name: Provider::NAME.to_string(),
            })),
            _ => None,
        }
    }
}

pub struct Provider;
impl Provider {
    /// The name this Provider is registered under.
    pub const NAME: &'static str = "SortField";
}

/// Specifies the type of the terms to be sorted, or special types such as `CUSTOM`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Type {
    /// Sort by document score (relevance). Sort values are `f32` and higher values are at the front.
    Score,

    /// Sort by document number (index order). Sort values are `i32` and lower values are at the front.
    Doc,

    /// Sort using term values as `String`. Sort values are `String` and lower values are at the front.
    String,

    /// Sort using term values as encoded `i32`. Sort values are `i32` and lower values are at the front.
    /// Fields must either be not indexed, or indexed with `IntPoint`.
    Int,

    /// Sort using term values as encoded `f32`. Sort values are `f32` and lower values are at the front.
    /// Fields must either be not indexed, or indexed with `FloatPoint`.
    Float,

    /// Sort using term values as encoded `i64`. Sort values are `i64` and lower values are at the front.
    /// Fields must either be not indexed, or indexed with `LongPoint`.
    Long,

    /// Sort using term values as encoded `f64`. Sort values are `f64` and lower values are at the front.
    /// Fields must either be not indexed, or indexed with `DoublePoint`.
    Double,

    /// Sort using a custom comparator. Sort values are any `Comparable` and sorting is done according
    /// to natural order.
    Custom,

    /// Sort using term values as `String`, but comparing by value (using `String::cmp`) for all comparisons.
    /// This is typically slower than `STRING`, which uses ordinals to do the sorting.
    StringVal,

    /// Force rewriting of `SortField` using `SortField::rewrite` before it can be used for sorting.
    Rewriteable,
}

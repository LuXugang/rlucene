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
use crate::core::analysis::analyzer::Analyzer;
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_stream::{InnerTokenStreams, TokenStreamEnum2};
use crate::core::document::field::{Field, FieldBase, FieldDataEnum};
use crate::core::document::field_type::FieldType;
use crate::core::document::invertable_field::InvertableType;
use crate::core::index::BytesRef;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::search::point_range_query::{PointRangeBase, PointRangeQuery, check_args};
use crate::core::util::SliceCopyOps;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
use std::borrow::Cow;
use std::fmt;
use std::fmt::Formatter;

/// BinaryPoint field (Rust translation of Lucene's BinaryPoint)
pub struct BinaryPoint {
    parent_field: Field,
}

impl BinaryPoint {
    /// Create a new BinaryPoint with N-dimensional byte vectors
    pub fn new<T, P>(name: T, point: P) -> Result<BinaryPoint>
    where
        T: Into<String>,
        P: AsRef<[Vec<u8>]>,
    {
        let point = point.as_ref();
        let packed = Self::pack(point)?;
        let field_type = Self::get_type_from_dims(point)?;
        let parent_field = Field::with_bytes_ref(name, packed, field_type)?;
        Ok(BinaryPoint { parent_field })
    }

    /// Expert API: directly supply packed bytes + type
    pub fn with_type<T>(
        name: T,
        packed_point: Vec<u8>,
        field_type: FieldType,
    ) -> Result<BinaryPoint>
    where
        T: Into<String>,
    {
        let expected = field_type.point_dimension_count() * field_type.point_num_bytes();
        if packed_point.len() != expected {
            return Err(LuceneError::illegal_argument(format!(
                "packedPoint is length={} but type.pointDimensionCount()={} and type.pointNumBytes()={}",
                packed_point.len(),
                field_type.point_dimension_count(),
                field_type.point_num_bytes()
            )));
        }

        let parent_field = Field::with_binary(name, packed_point, field_type)?;
        Ok(BinaryPoint { parent_field })
    }

    fn get_type_from_dims(point: &[Vec<u8>]) -> Result<FieldType> {
        if point.is_empty() {
            return Err(LuceneError::illegal_argument(
                "point must not be 0 dimensions".to_string(),
            ));
        }

        let mut bytes_per_dim: Option<usize> = None;

        for one_dim in point {
            if one_dim.is_empty() {
                return Err(LuceneError::illegal_argument(
                    "point must not have 0-length values".to_string(),
                ));
            }
            match bytes_per_dim {
                None => bytes_per_dim = Some(one_dim.len()),
                Some(b) if b != one_dim.len() => {
                    return Err(LuceneError::illegal_argument(format!(
                        "all dimensions must have same bytes length; got {} and {}",
                        b,
                        one_dim.len()
                    )));
                },
                _ => {},
            }
        }

        Self::get_type(point.len(), bytes_per_dim.unwrap())
    }

    fn get_type(num_dims: usize, bytes_per_dim: usize) -> Result<FieldType> {
        let mut ty = FieldType::new();
        ty.set_dimensions(num_dims, bytes_per_dim)?;
        ty.freeze();
        Ok(ty)
    }

    pub fn pack(point: &[Vec<u8>]) -> Result<BytesRef<Vec<u8>>> {
        if point.is_empty() {
            return Err(LuceneError::illegal_argument(
                "point must not be 0 dimensions".to_string(),
            ));
        }

        if point.len() == 1 {
            return Ok(BytesRef::from_bytes(point[0].clone()));
        }

        let mut bytes_per_dim: Option<usize> = None;
        for d in point {
            if d.is_empty() {
                return Err(LuceneError::illegal_argument(
                    "point must not have 0-length values".to_string(),
                ));
            }
            match bytes_per_dim {
                None => bytes_per_dim = Some(d.len()),
                Some(b) if b != d.len() => {
                    return Err(LuceneError::illegal_argument(format!(
                        "all dimensions must have same bytes length; got {} and {}",
                        b,
                        d.len()
                    )));
                },
                _ => {},
            }
        }

        let bytes_per_dim = bytes_per_dim.unwrap();
        let mut packed = vec![0u8; bytes_per_dim * point.len()];
        for (i, dim) in point.iter().enumerate() {
            packed.copy_from(&dim[0..bytes_per_dim], i * bytes_per_dim);
        }

        Ok(BytesRef::from_bytes(packed))
    }

    pub fn new_exact_query<T>(field: T, value: Vec<u8>) -> Result<PointRangeQuery>
    where
        T: Into<String>,
    {
        Self::new_range_query(field, value.clone(), value)
    }

    pub fn new_range_query<T>(field: T, lower: Vec<u8>, upper: Vec<u8>) -> Result<PointRangeQuery>
    where
        T: Into<String>,
    {
        let field = field.into();
        check_args(&field, lower.as_ref(), upper.as_ref())?;
        Self::new_range_query_multi_dim(field, &[lower], &[upper])
    }

    pub fn new_range_query_multi_dim<T>(
        field: T,
        lower: &[Vec<u8>],
        upper: &[Vec<u8>],
    ) -> Result<PointRangeQuery>
    where
        T: Into<String>,
    {
        if lower.len() != upper.len() {
            return Err(LuceneError::illegal_argument(
                "lowerValue.length != upperValue.length".to_string(),
            ));
        }

        let field = field.into();
        let mut packed_lower = Self::pack(lower)?;
        let mut packed_upper = Self::pack(upper)?;

        check_args(&field, &packed_lower.bytes, &packed_upper.bytes)?;

        PointRangeQuery::new(
            field,
            packed_lower.take_bytes(),
            packed_upper.take_bytes(),
            lower.len(),
            BinaryPointRangeQuery,
        )
    }
}

impl FieldBase for BinaryPoint {
    fn set_bytes_value(&mut self, _value: BytesRef<Vec<u8>>) -> Result<()> {
        Err(LuceneError::illegal_argument(
            "cannot change value type from binary point to BytesRef".to_string(),
        ))
    }

    fn set_int_value(&mut self, _value: i32) -> Result<()> {
        Err(LuceneError::illegal_argument(
            "cannot set int value on BinaryPoint".to_string(),
        ))
    }
}
impl IndexableField for BinaryPoint {
    fn name(&self) -> &str {
        self.parent_field.name()
    }

    type FieldType = FieldType;

    fn field_type(&self) -> &Self::FieldType {
        self.parent_field.field_type()
    }

    type TokenStream = <Field as IndexableField>::TokenStream;

    fn token_stream<'a>(
        &'a mut self,
        token_stream: Option<&'a mut InnerTokenStreams>,
    ) -> Result<Option<TokenStreamEnum2<&'a mut InnerTokenStreams, &'a mut Self::TokenStream>>>
    {
        self.parent_field.token_stream(token_stream)
    }

    fn binary_value(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        self.parent_field.binary_value()
    }

    fn take_binary_value(&mut self) -> Result<Option<BytesRef<Vec<u8>>>> {
        self.parent_field.take_binary_value()
    }

    fn string_value(&self) -> Result<Option<Cow<'_, String>>> {
        self.parent_field.string_value()
    }

    fn take_string_value(&mut self) -> Result<Option<String>> {
        self.parent_field.take_string_value()
    }

    fn take_reader_value(&mut self) -> Result<Option<ReaderEnum>> {
        Ok(None)
    }

    fn numeric_value(&self) -> Result<Option<Number>> {
        Err(LuceneError::illegal_argument(
            "BinaryPoint has no numericValue".to_string(),
        ))
    }

    fn stored_value(&self) -> Option<&FieldDataEnum> {
        self.parent_field.stored_value()
    }

    fn take_stored_value(&mut self) -> Option<FieldDataEnum> {
        self.parent_field.take_stored_value()
    }

    fn invertable_type(&self) -> &InvertableType {
        self.parent_field.invertable_type()
    }

    fn init_token_stream<A>(&mut self, analyzer: &A) -> Result<()>
    where
        A: Analyzer,
    {
        self.parent_field.init_token_stream(analyzer)
    }
}

impl fmt::Display for BinaryPoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.parent_field.fmt(f)
    }
}

#[derive(Debug, Clone)]
pub struct BinaryPointRangeQuery;

impl PointRangeBase for BinaryPointRangeQuery {
    fn to_string(&self, _dimension: usize, value: &[u8]) -> String {
        let mut out = String::from("binary(");
        for (i, b) in value.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(&format!("{:x}", b));
        }
        out.push(')');
        out
    }
}

#[cfg(test)]
impl Clone for BinaryPoint {
    fn clone(&self) -> Self {
        Self {
            parent_field: self.parent_field.clone(),
        }
    }
}

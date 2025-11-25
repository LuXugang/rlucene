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
use crate::core::analysis::token_stream::{Either2TokenStream, InnerTokenStreams};
use crate::core::document::field::FieldDataEnum::Dummy;
use crate::core::document::field::{Field, FieldBase, FieldDataEnum};
use crate::core::document::field_type::FieldType;
use crate::core::document::invertable_field::InvertableType;
use crate::core::index::BytesRef;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::util::CoreHelper;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
use crate::core::util::numeric_utils::NumericUtils;
use std::borrow::Cow;
use std::fmt;

pub struct IntRange {
    parent_field: Field,
}

impl IntRange {
    pub fn new<T, P>(name: T, min: P, max: P) -> Result<Self>
    where
        T: Into<String>,
        P: AsRef<[i32]>,
    {
        let min = min.as_ref();
        let field_type = Self::get_type(min.len() as i32)?;
        let mut parent_field = Field::new(name, Dummy(()), field_type);
        Self::set_range_values_internal(&mut parent_field, min, max.as_ref())?;
        Ok(IntRange { parent_field })
    }

    fn get_type(dimensions: i32) -> Result<FieldType> {
        if dimensions > 4 {
            return Err(LuceneError::illegal_argument(
                "IntRange does not support greater than 4 dimensions".to_string(),
            ));
        }
        let mut ft = FieldType::new();
        ft.set_dimensions(dimensions * 2, BitUtil::INT_BYTES as i32)?;
        ft.freeze();
        Ok(ft)
    }

    pub fn set_range_values(&mut self, min: &[i32], max: &[i32]) -> Result<()> {
        Self::set_range_values_internal(&mut self.parent_field, min, max)
    }

    fn set_range_values_internal(parent_field: &mut Field, min: &[i32], max: &[i32]) -> Result<()> {
        Self::check_args(min, max)?;

        let dims = parent_field.field_type().point_dimension_count();
        if min.len() * 2 != dims as usize || max.len() * 2 != dims as usize {
            return Err(LuceneError::illegal_argument(format!(
                "field (name={}) uses {} dimensions; cannot change to (incoming) {} dimensions",
                parent_field.name(),
                dims / 2,
                min.len()
            )));
        }

        let bytes = match &mut parent_field.fields_data {
            FieldDataEnum::Binary(b) => &mut b.bytes,
            FieldDataEnum::Dummy(_) => {
                let new_bytes = vec![0u8; BitUtil::INT_BYTES * 2 * min.len()];
                parent_field.fields_data = BytesRef::from_bytes(new_bytes).into();
                match &mut parent_field.fields_data {
                    FieldDataEnum::Binary(b) => &mut b.bytes,
                    _ => return Err(LuceneError::illegal_state("should not be here")),
                }
            },
            _ => Err(LuceneError::illegal_state(
                "Unsupported FieldDataEnum variant",
            ))?,
        };

        Self::verify_and_encode(min, max, bytes)
    }

    fn check_args(min: &[i32], max: &[i32]) -> Result<()> {
        if min.is_empty() || max.is_empty() {
            return Err(LuceneError::illegal_argument(
                "min/max range values cannot be null or empty".to_string(),
            ));
        }
        if min.len() != max.len() {
            return Err(LuceneError::illegal_argument(
                "min/max ranges must agree".to_string(),
            ));
        }
        if min.len() > 4 {
            return Err(LuceneError::illegal_argument(
                "IntRange does not support greater than 4 dimensions".to_string(),
            ));
        }
        Ok(())
    }

    pub fn encode(min: &[i32], max: &[i32]) -> Result<Vec<u8>> {
        Self::check_args(min, max)?;
        let mut b = vec![0u8; BitUtil::INT_BYTES * 2 * min.len()];
        Self::verify_and_encode(min, max, &mut b)?;
        Ok(b)
    }

    fn verify_and_encode(min: &[i32], max: &[i32], bytes: &mut [u8]) -> Result<()> {
        let n = min.len();
        let mut i = 0;
        let mut j = min.len() * BitUtil::INT_BYTES;
        for d in 0..n {
            if (min[d] as f64).is_nan() {
                return Err(LuceneError::illegal_argument(format!(
                    "invalid min value ({}) in IntRange",
                    f64::NAN
                )));
            }
            if (max[d] as f64).is_nan() {
                return Err(LuceneError::illegal_argument(format!(
                    "invalid max value ({}) in IntRange",
                    f64::NAN
                )));
            }
            if min[d] > max[d] {
                return Err(LuceneError::illegal_argument(format!(
                    "min value ({}) is greater than max value ({})",
                    min[d], max[d]
                )));
            }
            Self::encode_dimension(min[d], bytes, i);
            Self::encode_dimension(max[d], bytes, j);
            i += BitUtil::INT_BYTES;
            j += BitUtil::INT_BYTES;
        }
        Ok(())
    }

    fn encode_dimension(value: i32, dest: &mut [u8], offset: usize) {
        NumericUtils::int_to_sortable_bytes(value, dest, offset)
    }

    fn decode_min(bytes: &[u8], dimension: i32) -> i32 {
        let offset = dimension as usize * BitUtil::INT_BYTES;
        NumericUtils::sortable_bytes_to_int(bytes, offset)
    }

    fn decode_max(bytes: &[u8], dimension: i32) -> i32 {
        let offset = bytes.len() / 2 + dimension as usize * BitUtil::INT_BYTES;
        NumericUtils::sortable_bytes_to_int(bytes, offset)
    }

    pub fn get_min(&self, dimension: i32) -> Result<i32> {
        CoreHelper::check_index(
            dimension,
            self.parent_field.field_type().point_dimension_count() / 2,
        )?;
        match &self.parent_field.fields_data {
            FieldDataEnum::Binary(b) => Ok(Self::decode_min(&b.bytes, dimension)),
            _ => Err(LuceneError::illegal_argument(
                "Unsupported FieldDataEnum variant",
            )),
        }
    }

    pub fn get_max(&self, dimension: i32) -> Result<i32> {
        CoreHelper::check_index(
            dimension,
            self.parent_field.field_type().point_dimension_count() / 2,
        )?;
        match &self.parent_field.fields_data {
            FieldDataEnum::Binary(b) => Ok(Self::decode_max(&b.bytes, dimension)),
            _ => Err(LuceneError::illegal_argument(
                "Unsupported FieldDataEnum variant".to_string(),
            )),
        }
    }
}

impl FieldBase for IntRange {}

impl IndexableField for IntRange {
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
    ) -> Result<Option<Either2TokenStream<&'a mut InnerTokenStreams, &'a mut Self::TokenStream>>>
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
        todo!()
    }

    fn numeric_value(&self) -> Result<Option<Number>> {
        Err(LuceneError::illegal_argument(
            "cannot convert IntRange to a single numeric value".to_string(),
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

impl fmt::Display for IntRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} <{}:",
            std::any::type_name::<Self>(),
            self.parent_field.name()
        )?;

        let dims = self.parent_field.field_type().point_dimension_count() / 2;

        match &self.parent_field.fields_data {
            FieldDataEnum::Binary(bytes) => {
                for dim in 0..dims {
                    if dim > 0 {
                        write!(f, ",")?;
                    }
                    let min = Self::decode_min(&bytes.bytes, dim);
                    let max = Self::decode_max(&bytes.bytes, dim);
                    write!(f, "[{},{}]", min, max)?;
                }
            },
            _ => {
                write!(f, "Unsupported FieldDataEnum variant")?;
            },
        }

        write!(f, ">")
    }
}

#[cfg(test)]
impl Clone for IntRange {
    fn clone(&self) -> Self {
        Self {
            parent_field: self.parent_field.clone(),
        }
    }
}

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
use crate::core::document::field::{Field, FieldBase, FieldDataEnum, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::invertable_field::InvertableType;
use crate::core::index::BytesRef;
use crate::core::index::indexable_field::IndexableField;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
use crate::core::util::numeric_utils::NumericUtils;
use std::borrow::Cow;
use std::fmt;
pub mod int_field_type {
    use crate::core::document::field_type::FieldType;
    use crate::core::index::doc_values_type::DocValuesType;
    use crate::core::util::bit_util::BitUtil;
    use once_cell::sync::Lazy;

    pub static FIELD_TYPE: Lazy<FieldType> = Lazy::new(|| {
        let mut ft = FieldType::new();
        ft.set_dimensions(1, BitUtil::INT_BYTES as i32)
            .expect("set_dimensions should not fail");
        ft.set_doc_values_type(DocValuesType::SortedNumeric)
            .expect("set_doc_values_type should not fail");
        ft.freeze();
        ft
    });

    /// Indexed as SortedNumeric DocValue, and stored.
    pub static FIELD_TYPE_STORED: Lazy<FieldType> = Lazy::new(|| {
        let mut ft = FieldType::from_ref(&FIELD_TYPE.clone()).expect("should not fail");
        ft.set_stored(true)
            .expect("set_stored(true) should not fail");
        ft.freeze();
        ft
    });
}
pub struct IntField {
    parent_field: Field,
    stored_value: Option<FieldDataEnum>,
}

impl IntField {
    /// Creates a new `IntField`, indexing the provided value,
    /// storing it as a DocValue, and optionally as a stored field.
    pub fn new<T>(name: T, value: i32, stored: Store) -> Result<IntField>
    where
        T: Into<String>,
    {
        let stored = stored.into();
        let (field_type, stored_value) = if stored {
            (
                int_field_type::FIELD_TYPE_STORED.clone(),
                Some(value.into()),
            )
        } else {
            (int_field_type::FIELD_TYPE.clone(), None)
        };
        let parent_field = Field::new(name, field_type, value);
        Ok(IntField {
            parent_field,
            stored_value,
        })
    }
}

impl FieldBase for IntField {
    fn set_int_value(&mut self, value: i32) -> Result<()> {
        self.parent_field.set_int_value(value)?;
        if self.stored_value.is_some() {
            self.stored_value = Some(value.into());
        }
        Ok(())
    }
}

impl IndexableField for IntField {
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
        match &self.parent_field.fields_data {
            FieldDataEnum::Number(Number::I32(v)) => {
                let mut bytes = vec![0u8; BitUtil::INT_BYTES];
                NumericUtils::int_to_sortable_bytes(*v, &mut bytes, 0);
                Ok(Some(Cow::Owned(BytesRef::from_bytes(bytes))))
            },
            _ => Err(LuceneError::illegal_state(
                "parent_field`s fields_data does not have an int value",
            )),
        }
    }

    fn take_binary_value(&mut self) -> Result<Option<BytesRef<Vec<u8>>>> {
        self.binary_value().map(|v| v.map(|c| c.into_owned()))
    }

    fn string_value(&self) -> Result<Option<Cow<'_, String>>> {
        self.parent_field.string_value()
    }

    fn take_string_value(&mut self) -> Result<Option<String>> {
        self.parent_field.take_string_value()
    }

    fn take_reader_value(&mut self) -> Result<Option<ReaderEnum>> {
        self.parent_field.take_reader_value()
    }

    fn numeric_value(&self) -> Result<Option<Number>> {
        self.parent_field.numeric_value()
    }

    fn stored_value(&self) -> Option<&FieldDataEnum> {
        self.stored_value.as_ref()
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

impl fmt::Display for IntField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} <{}:{}>",
            std::any::type_name::<Self>(),
            self.parent_field.name(),
            self.parent_field.fields_data
        )
    }
}

#[cfg(test)]
impl Clone for IntField {
    fn clone(&self) -> Self {
        Self {
            parent_field: self.parent_field.clone(),
            stored_value: self.stored_value.clone(),
        }
    }
}

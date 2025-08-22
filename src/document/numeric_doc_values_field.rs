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
use crate::analysis::analyzer::Analyzer;
use crate::document::field::{Field, FieldDataEnum};
use crate::document::field_type::FieldType;
use crate::document::fields::ReaderEnum;
use crate::document::invertable_field::InvertableType;
use crate::document::stored_value::StoredValue;
use crate::index::BytesRef;
use crate::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::index::doc_values_type::DocValuesType;
use crate::index::indexable_field::IndexableField;
use crate::util::error::lucene_error::Result;
use crate::util::number::Number;
use once_cell::sync::Lazy;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

static TYPE: Lazy<FieldType> = Lazy::new(|| {
    let mut ft = FieldType::new();
    ft.set_doc_values_type(DocValuesType::Numeric)
        .expect("set_doc_values_type should never fail in this context");
    ft.freeze();
    ft
});
static INDEXED_TYPE: Lazy<FieldType> = Lazy::new(|| {
    let mut ft =
        FieldType::from_ref(&*TYPE).expect("FieldType::from_ref should never fail in this context");
    ft.set_doc_values_skip_index_type(DocValuesSkipIndexType::Range)
        .expect("set_doc_values_skip_index_type should never fail in this context");
    ft
});
pub struct NumericDocValuesField {
    parent_field: Field,
}
impl NumericDocValuesField {
    pub fn new(name: &str, value: i64) -> Self {
        Self::new_with_type(name, value, TYPE.clone())
    }
    pub fn new_with_type(name: &str, value: i64, file_type: FieldType) -> Self {
        let mut parent_field = Field::new(name, file_type);
        parent_field.fields_data = Option::from(FieldDataEnum::Number(Number::I64(value)));
        Self { parent_field }
    }
}

impl Display for NumericDocValuesField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}(name: {}, value: {:?})",
            std::any::type_name::<Self>(),
            self.parent_field.name(),
            self.numeric_value()
        )
    }
}

impl IndexableField for NumericDocValuesField {
    fn name(&self) -> &str {
        self.parent_field.name()
    }

    type FieldType = FieldType;

    fn field_type(&self) -> &Self::FieldType {
        self.parent_field.field_type()
    }

    type TokenStream = <Field as IndexableField>::TokenStream;

    fn token_stream<A>(
        &self,
        analyzer: &A,
        reuse: Option<Self::TokenStream>,
    ) -> Result<Option<Self::TokenStream>>
    where
        A: Analyzer,
    {
        self.parent_field.token_stream(analyzer, reuse)
    }

    fn binary_value(&self) -> Result<Option<Rc<BytesRef<Vec<u8>>>>> {
        self.parent_field.binary_value()
    }

    fn string_value(&self) -> Result<Option<Rc<String>>> {
        self.parent_field.string_value()
    }

    fn reader_value(&self) -> Result<Option<ReaderEnum>> {
        self.parent_field.reader_value()
    }

    fn numeric_value(&self) -> Result<Option<Number>> {
        self.parent_field.numeric_value()
    }

    fn stored_value(&self) -> Result<Option<StoredValue>> {
        self.parent_field.stored_value()
    }

    fn invertable_type(&self) -> &InvertableType {
        self.parent_field.invertable_type()
    }
}

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
use crate::analysis::analyzer::Analyzer;
use crate::document::field::Field;
use crate::document::field_type::FieldType;
use crate::document::fields::ReaderEnum;
use crate::document::invertable_field::InvertableType;
use crate::document::stored_value::StoredValue;
use crate::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::index::doc_values_type::DocValuesType;
use crate::index::indexable_field::IndexableField;
use crate::index::BytesRef;
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

impl Display for NumericDocValuesField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NumericDocValuesField(name: {}, value: {:?})",
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
    ) -> Result<Self::TokenStream>
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

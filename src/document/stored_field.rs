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
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::sync::Arc;

use once_cell::sync::Lazy;

use crate::analysis::analyzer::Analyzer;
use crate::document::field::{Field, FieldBase, FieldDataEnum};
use crate::document::field_type::FieldType;
use crate::document::fields::ReaderEnum;
use crate::document::invertable_field::InvertableType;
use crate::document::stored_value::StoredValue;
use crate::index::indexable_field::IndexableField;
use crate::index::BytesRef;
use crate::util::error::lucene_error::Result;
use crate::util::number::Number;

/// Type for a stored-only field.
static TYPE: Lazy<Arc<FieldType>> = Lazy::new(|| {
    let mut ft = FieldType::new();
    ft.set_stored(true)
        .expect("set_stored(true) should never fail in this context");
    ft.freeze();
    Arc::new(ft)
});
/// A field whose value is stored so that
/// [`IndexSearcher::stored_fields`](crate::search::index_searcher::IndexSearcher::stored_fields)
/// and [`IndexReader::stored_fields`](crate::search::index_searcher::IndexSearcher::stored_fields)
/// will return the field and its value.
pub struct StoredField {
    parent_field: Field,
}
#[allow(unused)]
impl StoredField {
    /// Expert: allows you to customize the [`FieldType`].
    ///
    /// # Parameters
    /// - `name`: Field name.
    /// - `field_type`: Custom [`FieldType`] for this field.
    pub fn new(name: &str, file_type: FieldType) -> Self {
        let parent_field = Field::new(name, Arc::new(file_type));
        Self { parent_field }
    }
    /// Expert: allows you to customize the [`FieldType`].
    ///
    /// # Note
    /// The provided byte array is **not copied**, so ensure that it is not
    /// modified until you are done using this field.
    ///
    /// # Parameters
    /// - `name`: Field name.
    /// - `bytes`: Byte array pointing to binary content (**not copied**).
    /// - `field_type`: Custom [`FieldType`] for this field.
    pub fn with_bytes_ref_and_type(
        name: &str,
        bytes: Rc<BytesRef<Vec<u8>>>,
        file_type: FieldType,
    ) -> Result<Self> {
        let parent_field = Field::with_bytes_ref(name, bytes.clone(), Arc::new(file_type))?;
        Ok(Self { parent_field })
    }
    /// Creates a stored-only field with the given binary value.
    ///
    /// # Note
    /// The provided byte array is **not copied**, so ensure that it is not
    /// modified until you are done using this field.
    ///
    /// # Parameters
    /// - `name`: Field name.
    /// - `value`: Byte array pointing to binary content.
    pub fn with_binary(name: &str, value: Vec<u8>) -> Result<Self> {
        let len = value.len();
        debug_assert!(len <= i32::MAX as usize);
        let bytes_ref = Rc::new(BytesRef::from_slice(value, 0, len));
        let parent_field = Field::with_bytes_ref(name, bytes_ref.clone(), Arc::clone(&TYPE))?;
        Ok(Self { parent_field })
    }
    /// Creates a stored-only field with the given binary value.
    ///
    /// # Note
    /// The provided byte array is **not copied**, so ensure that it is not
    /// modified until you are done using this field.
    ///
    /// # Parameters
    /// - `name`: Field name.
    /// - `value`: Byte array pointing to binary content .
    /// - `offset`: Starting position in the byte array.
    /// - `length`: Valid length of the byte array.
    pub fn with_binary_range(name: &str, value: Vec<u8>, offset: i32, length: i32) -> Result<Self> {
        let bytes_ref = Rc::new(BytesRef::from_slice(
            value,
            offset as usize,
            length as usize,
        ));
        let parent_field = Field::with_bytes_ref(name, bytes_ref.clone(), Arc::clone(&TYPE))?;
        Ok(Self { parent_field })
    }
    /// Creates a stored-only field with the given binary value.
    ///
    /// # Note
    /// The provided [`BytesRef`] is **not copied**, so ensure that it is not
    /// modified until you are done using this field.
    ///
    /// # Parameters
    /// - `name`: Field name.
    /// - `value`: [`BytesRef`] pointing to binary content (**not copied**).
    pub fn with_bytes_ref(name: &str, value: Rc<BytesRef<Vec<u8>>>) -> Result<Self> {
        let parent_field = Field::with_bytes_ref(name, value.clone(), Arc::clone(&TYPE))?;
        Ok(Self { parent_field })
    }
    /// Creates a stored-only field with the given string value.
    ///
    /// # Parameters
    /// - `name`: Field name.
    /// - `value`: String value.
    pub fn with_string(name: &str, value: &str) -> Result<Self> {
        let value_str = Rc::new(value.to_string());
        let parent_field = Field::with_string(name, value_str, Arc::clone(&TYPE))?;
        Ok(Self { parent_field })
    }
    /// Expert: allows customization of the [`FieldType`].
    ///
    /// # Parameters
    /// - `name`: Field name.
    /// - `value`: String value.
    /// - `field_type`: Custom [`FieldType`] for this field.
    pub fn with_string_and_type(name: &str, value: &str, file_type: FieldType) -> Result<Self> {
        let value_str = Rc::new(value.to_string());
        let parent_field = Field::with_string(name, value_str, Arc::new(file_type))?;
        Ok(Self { parent_field })
    }
    /// Creates a stored-only field with the given i32 value.
    ///
    /// # Parameters
    /// - `name`: Field name.
    /// - `value`: i32 value.
    pub fn with_i32(name: &str, value: i32) -> Result<Self> {
        let fields_data = FieldDataEnum::Number(Number::I32(value));
        let mut parent_field = Field::new(name, Arc::clone(&TYPE));
        parent_field.fields_data = Option::from(fields_data.clone());
        Ok(Self { parent_field })
    }
    /// Creates a stored-only field with the given long value.
    ///
    /// # Parameters
    /// - `name`: Field name.
    /// - `value`: Long value.
    pub fn with_i64(name: &str, value: i64) -> Result<Self> {
        let fields_data = FieldDataEnum::Number(Number::I64(value));
        let mut parent_field = Field::new(name, Arc::clone(&TYPE));
        parent_field.fields_data = Option::from(fields_data.clone());
        Ok(Self { parent_field })
    }
    /// Creates a stored-only field with the given f32 value.
    ///
    /// # Parameters
    /// - `name`: Field name.
    /// - `value`: f32 value.
    pub fn with_f32(name: &str, value: f32) -> Result<Self> {
        let fields_data = FieldDataEnum::Number(Number::F32(value));
        let mut parent_field = Field::new(name, Arc::clone(&TYPE));
        parent_field.fields_data = Option::from(fields_data.clone());
        Ok(Self { parent_field })
    }
    /// Creates a stored-only field with the given f64 value.
    ///
    /// # Parameters
    /// - `name`: Field name.
    /// - `value`: f64 value.
    pub fn with_f64(name: &str, value: f64) -> Result<Self> {
        let fields_data = FieldDataEnum::Number(Number::F64(value));
        let mut parent_field = Field::new(name, Arc::clone(&TYPE));
        parent_field.fields_data = Option::from(fields_data.clone());
        Ok(Self { parent_field })
    }
}
impl FieldBase for StoredField {}

impl Display for StoredField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.parent_field.fmt(f)
    }
}

impl IndexableField for StoredField {
    fn name(&self) -> &str {
        self.parent_field.name()
    }

    type FieldType = FieldType;

    fn field_type(&self) -> Arc<Self::FieldType> {
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

    fn get_char_sequence_value(&self) -> Result<Option<Rc<String>>> {
        self.parent_field.get_char_sequence_value()
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

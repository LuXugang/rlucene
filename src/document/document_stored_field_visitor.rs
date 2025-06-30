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
use std::collections::HashSet;
use std::rc::Rc;

use crate::codecs::stored_fields_writer::StoredFieldsWriter;
use crate::document::document::Document;
use crate::document::field_type::FieldType;
use crate::document::stored_field::StoredField;
use crate::document::text_field::text;
use crate::index::field_info::FieldInfo;
use crate::index::stored_field_visitor::{Status, StoredFieldVisitor};
use crate::util::error::lucene_error::Result;

/// A [`StoredFieldVisitor`] that creates a [`Document`] from stored fields.
///
/// This visitor supports loading all stored fields, or only specific requested
/// fields provided from a `Set`.
///
/// This is used by
/// [`StoredFields::document`](crate::index::stored_fields::StoredFields::document)
/// to load a document.
pub struct DocumentStoredFieldVisitor<'a> {
    doc: Document,
    fields_to_add: Option<&'a HashSet<String>>,
}
impl Default for DocumentStoredFieldVisitor<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> DocumentStoredFieldVisitor<'a> {
    /// Load all fields
    pub fn new() -> Self {
        Self {
            doc: Document::default(),
            fields_to_add: None,
        }
    }

    /// Load only selected fields
    pub fn with_fields(fields: &'a HashSet<String>) -> Self {
        Self {
            doc: Document::default(),
            fields_to_add: Some(fields),
        }
    }

    #[allow(unused)]
    pub fn get_document_ref(&self) -> &Document {
        &self.doc
    }
    /// Retrieve the visited document.
    ///
    /// Returns a [`Document`] populated with stored fields.
    /// Note that only the stored information in the field instances is valid;
    /// data such as indexing options, term vector options, etc. is not set.
    pub fn get_document_owner(&mut self) -> Document {
        std::mem::take(&mut self.doc)
    }
}
impl StoredFieldVisitor for DocumentStoredFieldVisitor<'_> {
    fn binary_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: Vec<u8>,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        self.doc
            .add(StoredField::with_binary(&field_info.name, value)?);
        Ok(())
    }

    fn string_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: &str,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        let mut ft = FieldType::from_ref(&*text::TYPE_STORED)?;
        ft.set_store_term_vectors(field_info.has_term_vectors())?;
        ft.set_omit_norms(field_info.omits_norms())?;
        ft.set_index_options(*field_info.get_index_options())?;
        self.doc.add(StoredField::with_string_and_type(
            &field_info.name,
            value,
            ft,
        )?);
        Ok(())
    }

    fn int_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: i32,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        self.doc
            .add(StoredField::with_i32(&field_info.name, value)?);
        Ok(())
    }

    fn long_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: i64,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        self.doc
            .add(StoredField::with_i64(&field_info.name, value)?);
        Ok(())
    }

    fn float_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: f32,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        self.doc
            .add(StoredField::with_f32(&field_info.name, value)?);
        Ok(())
    }

    fn double_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: f64,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        self.doc
            .add(StoredField::with_f64(&field_info.name, value)?);
        Ok(())
    }

    fn needs_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<Status> {
        match self.fields_to_add {
            Some(fields) => {
                if fields.contains(&field_info.name) {
                    Ok(Status::Yes)
                } else {
                    Ok(Status::No)
                }
            },
            None => Ok(Status::Yes),
        }
    }
}

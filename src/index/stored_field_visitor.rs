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
use std::rc::Rc;

use crate::codecs::stored_fields_writer::StoredFieldsWriter;
use crate::index::field_info::FieldInfo;
use crate::store::DataInput;
use crate::util::error::lucene_error::Result;

/// Expert: provides a low-level means of accessing the stored field values in
/// an index.
///
/// # NOTE
/// a `StoredFieldVisitor` implementation should not try to load or visit other
/// stored documents in the same reader because the implementation of stored
/// fields for most codecs is not reentrant and you will see strange exceptions
/// as a result.
///
/// See [`DocumentStoredFieldVisitor`](crate::document::document_stored_field_visitor::DocumentStoredFieldVisitor), which is a `StoredFieldVisitor` that builds the [`Document`](crate::document::document::Document)
/// containing all stored fields.
pub trait StoredFieldVisitor {
    /// Expert: Process a binary field directly from the DataInput.
    /// Implementors of this method must read `length` bytes from the given
    /// `DataInput`. Default implementation reads into a byte array and
    /// delegates to `binary_field`.
    fn binary_field_with_input(
        &mut self,
        field_info: Rc<FieldInfo>,
        input: &mut impl DataInput,
        length: i32,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        let mut buffer = vec![0u8; length as usize];
        input.read_bytes(&mut buffer, 0, length)?;
        self.binary_field(field_info, buffer, writer)
    }

    /// Process a binary field.
    fn binary_field(
        &mut self,
        _field_info: Rc<FieldInfo>,
        _value: Vec<u8>,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        Ok(())
    }

    /// Process a string field.
    fn string_field(
        &mut self,
        _field_info: Rc<FieldInfo>,
        _value: &str,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        Ok(())
    }

    /// Process an int numeric field.
    fn int_field(
        &mut self,
        _field_info: Rc<FieldInfo>,
        _value: i32,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        Ok(())
    }

    /// Process a long numeric field.
    fn long_field(
        &mut self,
        _field_info: Rc<FieldInfo>,
        _value: i64,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        Ok(())
    }

    /// Process a float numeric field.
    fn float_field(
        &mut self,
        _field_info: Rc<FieldInfo>,
        _value: f32,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        Ok(())
    }

    /// Process a double numeric field.
    fn double_field(
        &mut self,
        _field_info: Rc<FieldInfo>,
        _value: f64,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        Ok(())
    }

    /// Hook before processing a field.
    /// Returns a [`Status`] representing whether to visit, skip, or stop.
    fn needs_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<Status>;
}

/// Enumeration of possible return values for `needs_field`.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Status {
    /// YES: the field should be visited.
    Yes,
    /// NO: don't visit this field, but continue processing fields for this
    /// document.
    No,
    /// STOP: don't visit this field and stop processing any other fields for
    /// this document.
    Stop,
}

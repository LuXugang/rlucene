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

use crate::codecs::compressing::lucene90_compressing_stored_fields_reader::Lucene90CompressingStoredFieldsReader;
use crate::codecs::stored_fields_writer::StoredFieldsWriter;
use crate::document::document::Document;
use crate::index::stored_field_visitor::StoredFieldVisitor;
use crate::index::stored_fields::StoredFields;
use crate::store::IndexInput;
use crate::util::clone::TryClone;
use crate::util::error::lucene_error::Result;

/// Codec API for reading stored fields.
///
/// You need to implement [`document(int,
/// StoredFieldVisitor)`](StoredFields::document_with_visitor) to read the
/// stored fields for a document, implement `clone()`(creating clones of any
/// IndexInputs used, etc)
pub trait StoredFieldsReader<I>: StoredFields + TryClone
where
    I: IndexInput,
{
    /// Checks consistency of this reader.
    ///
    /// Note that this may be costly in terms of I/O, e.g. may involve computing
    /// a checksum value against large data files.
    fn check_integrity(&mut self) -> Result<()>;
    /// Returns an instance optimized for merging. This instance may only be
    /// cloned # Note
    /// Returning None means returning itself.
    fn get_merge_instance(&self) -> Result<Option<StoredFieldsReaderEnum<I>>> {
        Ok(None)
    }
}

pub enum StoredFieldsReaderEnum<I>
where
    I: IndexInput,
{
    Lucene90(Lucene90CompressingStoredFieldsReader<I>),
}

impl<I> StoredFields for StoredFieldsReaderEnum<I>
where
    I: IndexInput,
{
    fn prefetch(&mut self, doc_id: i32) -> Result<()> {
        match self {
            StoredFieldsReaderEnum::Lucene90(reader) => reader.prefetch(doc_id),
        }
    }

    fn document(&mut self, doc_id: i32, writer: &mut impl StoredFieldsWriter) -> Result<Document> {
        match self {
            StoredFieldsReaderEnum::Lucene90(reader) => reader.document(doc_id, writer),
        }
    }

    fn document_with_visitor(
        &mut self,
        doc_id: i32,
        visitor: &mut impl StoredFieldVisitor,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        match self {
            StoredFieldsReaderEnum::Lucene90(reader) => {
                reader.document_with_visitor(doc_id, visitor, writer)
            },
        }
    }

    fn document_with_fields(
        &mut self,
        doc_id: i32,
        fields_to_load: &HashSet<String>,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<Document> {
        match self {
            StoredFieldsReaderEnum::Lucene90(reader) => {
                reader.document_with_fields(doc_id, fields_to_load, writer)
            },
        }
    }
}

impl<I> TryClone for StoredFieldsReaderEnum<I>
where
    I: IndexInput,
{
    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized,
    {
        match self {
            StoredFieldsReaderEnum::Lucene90(reader) => {
                Ok(StoredFieldsReaderEnum::Lucene90(reader.try_clone()?))
            },
        }
    }
}

impl<I> StoredFieldsReader<I> for StoredFieldsReaderEnum<I>
where
    I: IndexInput,
{
    fn check_integrity(&mut self) -> Result<()> {
        match self {
            StoredFieldsReaderEnum::Lucene90(reader) => reader.check_integrity(),
        }
    }

    fn get_merge_instance(&self) -> Result<Option<StoredFieldsReaderEnum<I>>> {
        match self {
            StoredFieldsReaderEnum::Lucene90(reader) => reader.get_merge_instance(),
        }
    }
}

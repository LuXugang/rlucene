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
use crate::codecs::compressing::lucene90_compressing_stored_fields_reader::Lucene90CompressingStoredFieldsReader;
use crate::codecs::stored_fields_writer::StoredFieldsWriter;
use crate::document::document::Document;
use crate::index::stored_field_visitor::StoredFieldVisitor;
use crate::index::stored_fields::StoredFields;
use crate::store::IndexInput;
use crate::util::clone::TryClone;
use crate::util::error::lucene_error::Result;
use std::collections::HashSet;

/// Codec API for reading stored fields.
///
/// You need to implement [`document(int, StoredFieldVisitor)`](StoredFields::document_with_visitor) to read the stored fields for
/// a document, implement `clone()`(creating clones of any IndexInputs used, etc)
pub trait StoredFieldsReader<I>: StoredFields + TryClone
where
    I: IndexInput,
{
    /// Checks consistency of this reader.
    ///
    /// Note that this may be costly in terms of I/O, e.g. may involve computing a checksum value
    /// against large data files.
    fn check_integrity(&mut self) -> Result<()>;
    /// Returns an instance optimized for merging. This instance may only be cloned
    /// # Note
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
            }
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
            }
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
            }
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

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
use crate::core::codecs::dummy::stored_fields_writer::DummyStoredFieldsWriter;
use crate::core::codecs::stored_fields_reader::DefaultStoredFieldsReader;
use crate::core::codecs::stored_fields_writer::StoredFieldsWriter;
use crate::core::document::document::Document;
use crate::core::document::document_stored_field_visitor::DocumentStoredFieldVisitor;
use crate::core::index::stored_field_visitor::StoredFieldVisitor;
use crate::core::store::dummy::dummy_index_input::DummyIndexInput;
use crate::core::store::index_input::IndexInput;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::collections::HashSet;

pub trait RawStoredFieldsReader {
  type IndexInput: IndexInput;

  fn raw_stored_fields_mut(&mut self) -> Result<&mut DefaultStoredFieldsReader<Self::IndexInput>> {
    Err(LuceneError::unsupported_operation(
      "raw stored fields are not available",
    ))
  }

  fn raw_stored_fields(&self) -> Result<&DefaultStoredFieldsReader<Self::IndexInput>> {
    Err(LuceneError::unsupported_operation(
      "raw stored fields are not available",
    ))
  }
}

/// API for reading stored fields.
///
/// **NOTE**: This struct is not thread-safe and should only be consumed in the
/// thread where it was acquired.
pub trait StoredFields: RawStoredFieldsReader {
  /// Optional method: Give a hint to this [`StoredFields`] instance that the
  /// given document will be read in the near future. This typically
  /// delegates to
  /// [`IndexInput::prefetch`]
  /// and is useful to parallelize I/O across multiple documents.
  ///
  /// NOTE: This API is expected to be called on a small enough set of doc IDs
  /// that they could all fit in the page cache. If you plan on retrieving
  /// a very large number of documents, it may be a good idea to perform
  /// calls to [`prefetch`](StoredFields::prefetch) and
  /// [`document`](Document) in batches instead of
  /// prefetching all documents up-front.
  fn prefetch(&mut self, _doc_id: i32) -> Result<()> {
    Ok(())
  }

  /// Returns the stored fields of the `n`th [`Document`] in this index. This is
  /// just sugar for using [`DocumentStoredFieldVisitor`].
  ///
  /// **NOTE:** for performance reasons, this method does not check if the
  /// requested document is deleted, and therefore asking for a deleted
  /// document may yield unspecified results. Usually this is not
  /// required, however you can test if the doc is deleted by checking the
  /// [`Bits`](crate::core::util::bits::Bits) returned from
  /// [`MultiBits`](crate::core::index::multi_bits::MultiBits).
  ///
  /// **NOTE:** only the content of a field is returned, if that field was
  /// stored during indexing. Metadata like boost, omitNorm, IndexOptions,
  /// tokenized, etc., are not preserved.
  ///
  /// # Errors
  ///
  /// - [`CorruptIndexError`](crate::core::util::error::CorruptIndexError) if the
  ///   index is corrupt
  /// - [`std::io::Error`] if there is a low-level IO error
  fn document(&mut self, doc_id: i32) -> Result<Document> {
    let mut visitor = DocumentStoredFieldVisitor::new();
    self.document_with_visitor(doc_id, &mut visitor, Some(&mut DummyStoredFieldsWriter))?;
    Ok(visitor.get_document_owner())
  }

  /// Expert: visits the fields of a stored document, for custom
  /// processing/loading of each field. If you simply want to load all
  /// fields, use [`document`](Document). If you want to load a subset,
  /// use [`DocumentStoredFieldVisitor`].
  fn document_with_visitor<S>(
    &mut self,
    doc_id: i32,
    visitor: &mut impl StoredFieldVisitor,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter;

  /// Like [`document`](Document) but only loads the specified fields. Note
  /// that this is simply sugar
  /// for [`DocumentStoredFieldVisitor::with_fields`].
  fn document_with_fields(
    &mut self,
    doc_id: i32,
    fields_to_load: &HashSet<String>,
  ) -> Result<Document> {
    let mut visitor = DocumentStoredFieldVisitor::with_fields(fields_to_load);
    self.document_with_visitor(doc_id, &mut visitor, Some(&mut DummyStoredFieldsWriter))?;
    Ok(visitor.get_document_owner())
  }
}
macro_rules! either_stored_fields {
    (
        $vis:vis $name:ident { $( $Variant:ident : $T:ident ),+ $(,)? }
    ) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> StoredFields for $name<$( $T ),+>
        where
            $( $T: StoredFields ),+
        {
            fn prefetch(&mut self, doc_id: i32) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.prefetch(doc_id), )+
                }
            }

            fn document(
                &mut self,
                doc_id: i32,
            ) -> Result<Document> {
                match self {
                    $( Self::$Variant(inner) => inner.document(doc_id), )+
                }
            }

            fn document_with_visitor<S>(
                &mut self,
                doc_id: i32,
                visitor: &mut impl StoredFieldVisitor,
                writer: Option<&mut S>,
            ) -> Result<()> where S: StoredFieldsWriter{
                match self {
                    $( Self::$Variant(inner) => inner.document_with_visitor(doc_id, visitor, writer), )+
                }
            }

            fn document_with_fields(
                &mut self,
                doc_id: i32,
                fields_to_load: &HashSet<String>,
            ) -> Result<Document> {
                match self {
                    $( Self::$Variant(inner) => inner.document_with_fields(doc_id, fields_to_load), )+
                }
            }
        }
    };
}
either_stored_fields!(
    pub StoredFieldsEnum2 { A: A, B: B}
);

impl<A, B> RawStoredFieldsReader for StoredFieldsEnum2<A, B> {
  type IndexInput = DummyIndexInput;
}

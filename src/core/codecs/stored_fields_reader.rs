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
use crate::core::codecs::DefaultStoredFieldsFormat;
use crate::core::codecs::stored_fields_format::StoredFieldsFormat;
use crate::core::codecs::stored_fields_writer::StoredFieldsWriter;
use crate::core::document::document::Document;
use crate::core::index::stored_field_visitor::StoredFieldVisitor;
use crate::core::index::stored_fields::StoredFields;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashSet;

/// Codec API for reading stored fields.
///
/// You need to implement [`document(int,
/// StoredFieldVisitor)`](StoredFields::document_with_visitor) to read the
/// stored fields for a document, implement `clone()`(creating clones of any
/// IndexInputs used, etc)
pub trait StoredFieldsReader: StoredFields + Clone {
    /// Checks consistency of this reader.
    ///
    /// Note that this may be costly in terms of I/O, e.g. may involve computing
    /// a checksum value against large data files.
    fn check_integrity(&self) -> Result<()>;
    /// Returns an instance optimized for merging. This instance may only be
    /// cloned # Note
    /// Returning None means returning itself.
    fn get_merge_instance(&self) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        Ok(None)
    }
}

pub type DefaultStoredFieldsReader<I> =
    <DefaultStoredFieldsFormat as StoredFieldsFormat>::StoredFieldsReader<I>;

macro_rules! either_stored_fields_reader {
    ($vis:vis $name:ident { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> StoredFields for $name<$( $T ),+>
        where
            $( $T: StoredFieldsReader ),+
        {
            fn prefetch(&mut self, doc_id: i32) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.prefetch(doc_id), )+
                }
            }

            fn document(&mut self, doc_id: i32) -> Result<Document> {
                match self {
                    $( Self::$Variant(inner) => inner.document(doc_id), )+
                }
            }

            fn document_with_visitor<S: StoredFieldsWriter>(
                &mut self,
                doc_id: i32,
                visitor: &mut impl StoredFieldVisitor,
                writer: Option<&mut S>,
            ) -> Result<()> {
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

        impl<$( $T ),+> Clone for $name<$( $T ),+>
        where
            $( $T: StoredFieldsReader ),+
        {
            fn clone(&self) -> Self {
                match self {
                    $( Self::$Variant(inner) => Self::$Variant(inner.clone()), )+
                }
            }
        }

        impl<$( $T ),+> StoredFieldsReader for $name<$( $T ),+>
        where
            $( $T: StoredFieldsReader ),+
        {
            fn check_integrity(&self) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.check_integrity(), )+
                }
            }

            fn get_merge_instance(&self) -> Result<Option<Self>>
            where
                Self: Sized,
            {
                match self {
                    $( Self::$Variant(inner) => match inner.get_merge_instance()? {
                        Some(value) => Ok(Some(Self::$Variant(value))),
                        None => Ok(None),
                    }, )+
                }
            }
        }
    };
}

either_stored_fields_reader!(pub StoredFieldsReaderEnum2 { A: A, B: B });

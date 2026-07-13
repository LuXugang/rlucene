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
use crate::core::index::stored_fields::{RawStoredFieldsReader, StoredFields};
use crate::core::util::clone::TryClone;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashSet;

/// Codec API for reading stored fields.
///
/// You need to implement [`document(int,
/// StoredFieldVisitor)`](StoredFields::document_with_visitor) to read the
/// stored fields for a document, implement `try_clone()`(creating clones of any
/// IndexInputs used, etc), and [`CloseableRef::close`].
///
/// This uses [`TryClone`] rather than the built-in [`Clone`] because cloning
/// underlying inputs can fail, and `Clone::clone` cannot return an error.
pub trait StoredFieldsReader: StoredFields + TryClone + CloseableRef {
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
    ($vis:vis $name:ident { $FirstVariant:ident : $First:ident, $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$First, $( $T ),+> {
            $FirstVariant($First),
            $( $Variant($T), )+
        }

        impl<$First, $( $T ),+> CloseableRef for $name<$First, $( $T ),+>
        where
            $First: StoredFieldsReader,
            $( $T: StoredFieldsReader ),+
        {
            fn close(&self) -> Result<()> {
                match self {
                    Self::$FirstVariant(inner) => inner.close(),
                    $( Self::$Variant(inner) => inner.close(), )+
                }
            }
        }

        impl<$First, $( $T ),+> StoredFields for $name<$First, $( $T ),+>
        where
            $First: StoredFieldsReader,
            $( $T: StoredFieldsReader + RawStoredFieldsReader<IndexInput = <$First as RawStoredFieldsReader>::IndexInput> ),+
        {
            fn prefetch(&mut self, doc_id: i32) -> Result<()> {
                match self {
                    Self::$FirstVariant(inner) => inner.prefetch(doc_id),
                    $( Self::$Variant(inner) => inner.prefetch(doc_id), )+
                }
            }

            fn document(&mut self, doc_id: i32) -> Result<Document> {
                match self {
                    Self::$FirstVariant(inner) => inner.document(doc_id),
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
                    Self::$FirstVariant(inner) => {
                        inner.document_with_visitor(doc_id, visitor, writer)
                    }
                    $( Self::$Variant(inner) => inner.document_with_visitor(doc_id, visitor, writer), )+
                }
            }

            fn document_with_fields(
                &mut self,
                doc_id: i32,
                fields_to_load: &HashSet<String>,
            ) -> Result<Document> {
                match self {
                    Self::$FirstVariant(inner) => {
                        inner.document_with_fields(doc_id, fields_to_load)
                    }
                    $( Self::$Variant(inner) => inner.document_with_fields(doc_id, fields_to_load), )+
                }
            }
        }

        impl<$First, $( $T ),+> TryClone for $name<$First, $( $T ),+>
        where
            $First: StoredFieldsReader,
            $( $T: StoredFieldsReader ),+
        {
            fn try_clone(&self) -> Result<Self>
            where
                Self: Sized,
            {
                match self {
                    Self::$FirstVariant(inner) => Ok(Self::$FirstVariant(inner.try_clone()?)),
                    $( Self::$Variant(inner) => Ok(Self::$Variant(inner.try_clone()?)), )+
                }
            }
        }

        impl<$First, $( $T ),+> StoredFieldsReader for $name<$First, $( $T ),+>
        where
            $First: StoredFieldsReader,
            $( $T: StoredFieldsReader + RawStoredFieldsReader<IndexInput = <$First as RawStoredFieldsReader>::IndexInput> ),+
        {
            fn check_integrity(&self) -> Result<()> {
                match self {
                    Self::$FirstVariant(inner) => inner.check_integrity(),
                    $( Self::$Variant(inner) => inner.check_integrity(), )+
                }
            }

            fn get_merge_instance(&self) -> Result<Option<Self>>
            where
                Self: Sized,
            {
                match self {
                    Self::$FirstVariant(inner) => match inner.get_merge_instance()? {
                        Some(value) => Ok(Some(Self::$FirstVariant(value))),
                        None => Ok(None),
                    },
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

impl<A, B> RawStoredFieldsReader for StoredFieldsReaderEnum2<A, B>
where
  A: RawStoredFieldsReader,
  B: RawStoredFieldsReader<IndexInput = A::IndexInput>,
{
  type IndexInput = A::IndexInput;

  fn raw_stored_fields_mut(&mut self) -> Result<&mut DefaultStoredFieldsReader<Self::IndexInput>> {
    match self {
      Self::A(inner) => inner.raw_stored_fields_mut(),
      Self::B(inner) => inner.raw_stored_fields_mut(),
    }
  }

  fn raw_stored_fields(&self) -> Result<&DefaultStoredFieldsReader<Self::IndexInput>> {
    match self {
      Self::A(inner) => inner.raw_stored_fields(),
      Self::B(inner) => inner.raw_stored_fields(),
    }
  }
}

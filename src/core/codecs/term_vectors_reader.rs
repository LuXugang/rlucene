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
use crate::core::codecs::DefaultTermVectorsFormat;
use crate::core::codecs::term_vectors_format::TermVectorsFormat;
use crate::core::index::fields::Fields;
use crate::core::index::term_vectors::{RawTermVectors, TermVectors};
use crate::core::util::clone::TryClone;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
/// Codec API for reading term vectors:
///
/// This uses [`TryClone`] rather than the built-in [`Clone`] because cloning
/// underlying inputs can fail, and `Clone::clone` cannot return an error.
/// Implementations must also implement [`CloseableRef::close`].
pub trait TermVectorsReader: TermVectors + TryClone + CloseableRef {
  /// Checks consistency of this reader.
  ///
  /// Note that this may be costly in terms of I/O, e.g. may involve computing
  /// a checksum value against large data files.
  fn check_integrity(&self) -> Result<()>;

  /// Returns an instance optimized for merging.
  ///
  /// This instance may only be used from the thread that acquires it.
  fn get_merge_instance(&self) -> Result<Option<Self>>
  where
    Self: Sized,
  {
    Ok(None)
  }
}
pub type DefaultTermVectorsReader<I> =
  <DefaultTermVectorsFormat as TermVectorsFormat>::TermVectorsReader<I>;

macro_rules! either_term_vectors_reader_with_same_fields {
    ($vis:vis $name:ident { $Variant1:ident : $T1:ident, $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$T1, $( $T ),+> {
            $Variant1($T1),
            $( $Variant($T), )+
        }

        impl<$T1, $( $T ),+> CloseableRef for $name<$T1, $( $T ),+>
        where
            $T1: CloseableRef,
            $( $T: CloseableRef ),+
        {
            fn close(&self) -> Result<()> {
                match self {
                    Self::$Variant1(inner) => inner.close(),
                    $( Self::$Variant(inner) => inner.close(), )+
                }
            }
        }

        impl<$T1, $( $T ),+> TermVectors for $name<$T1, $( $T ),+>
        where
            $T1: TermVectors,
            $( $T: TermVectors<Fields = <$T1 as TermVectors>::Fields>
                + RawTermVectors<IndexInput = <$T1 as RawTermVectors>::IndexInput> ),+
        {
            type Fields = <$T1 as TermVectors>::Fields;

            type Terms = <<$T1 as TermVectors>::Fields as Fields>::Terms;

            fn prefetch(&mut self, doc_id: i32) -> Result<()> {
                match self {
                    Self::$Variant1(inner) => inner.prefetch(doc_id),
                    $( Self::$Variant(inner) => inner.prefetch(doc_id), )+
                }
            }

            fn get(&mut self, doc: i32) -> Result<Option<Self::Fields>> {
                match self {
                    Self::$Variant1(inner) => {
                        inner.get(doc)
                    }
                    $(
                        Self::$Variant(inner) => {
                            inner.get(doc)
                        }
                    ),+
                }
            }

            fn get_field_terms(
                &mut self,
                doc: i32,
                field: &str,
            ) -> Result<Option<<Self::Fields as Fields>::Terms>> {
                match self {
                    Self::$Variant1(inner) => {
                        inner.get_field_terms(doc, field)
                    }
                    $(
                        Self::$Variant(inner) => {
                            inner.get_field_terms(doc, field)
                        }
                    ),+
                }
            }
        }

        impl<$T1, $( $T ),+> TryClone for $name<$T1, $( $T ),+>
        where
            $T1: TryClone,
            $( $T: TryClone ),+
        {
            fn try_clone(&self) -> Result<Self>
            where
                Self: Sized,
            {
                match self {
                    Self::$Variant1(inner) => Ok(Self::$Variant1(inner.try_clone()?)),
                    $( Self::$Variant(inner) => Ok(Self::$Variant(inner.try_clone()?)), )+
                }
            }
        }

        impl<$T1, $( $T ),+> TermVectorsReader for $name<$T1, $( $T ),+>
        where
            $T1: TermVectorsReader,
            $( $T: TermVectorsReader
                + TermVectors<Fields = <$T1 as TermVectors>::Fields>
                + RawTermVectors<IndexInput = <$T1 as RawTermVectors>::IndexInput> ),+
        {
            fn check_integrity(&self) -> Result<()> {
                match self {
                    Self::$Variant1(inner) => inner.check_integrity(),
                    $( Self::$Variant(inner) => inner.check_integrity(), )+
                }
            }

            fn get_merge_instance(&self) -> Result<Option<Self>>
            where
                Self: Sized,
            {
                match self {
                    Self::$Variant1(inner) => match inner.get_merge_instance()? {
                        Some(value) => Ok(Some(Self::$Variant1(value))),
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

either_term_vectors_reader_with_same_fields!(
    pub TermVectorsReaderEnum2 { A: A, B: B }
);

impl<A, B> RawTermVectors for TermVectorsReaderEnum2<A, B>
where
  A: RawTermVectors,
  B: RawTermVectors<IndexInput = A::IndexInput>,
{
  type IndexInput = A::IndexInput;

  fn raw_term_vectors_mut(&mut self) -> Result<&mut DefaultTermVectorsReader<Self::IndexInput>> {
    match self {
      Self::A(inner) => inner.raw_term_vectors_mut(),
      Self::B(inner) => inner.raw_term_vectors_mut(),
    }
  }

  fn raw_term_vectors(&self) -> Result<&DefaultTermVectorsReader<Self::IndexInput>> {
    match self {
      Self::A(inner) => inner.raw_term_vectors(),
      Self::B(inner) => inner.raw_term_vectors(),
    }
  }
}

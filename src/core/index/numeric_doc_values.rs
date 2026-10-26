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
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error::Result;
/// A per-document numeric value.
pub trait NumericDocValues: DocValuesIterator {
  /// Returns the numeric value for the current document ID.
  /// It is illegal to call this method after
  /// [`DocValuesIterator::advance_exact`] returned `false`.
  ///
  /// # Returns
  /// The numeric value for the current document ID.
  fn long_value(&mut self) -> Result<i64>;

  /// Visits a borrowed cursor for an operation that can retain its concrete type.
  /// The default preserves this cursor's behavior, including any wrapper logic.
  /// Overrides must preserve cursor state, values, errors and capabilities;
  /// mutations made by the visitor remain visible on this cursor.
  fn visit<V: NumericDocValuesVisitor>(&mut self, visitor: V) -> V::Output
  where
    Self: Sized,
  {
    visitor.visit(Some(self))
  }
}

/// An operation over a borrowed numeric cursor, or over an absent norms field.
/// The result cannot retain the cursor borrow. Implementations may keep the
/// concrete cursor type throughout their document-processing loop.
pub trait NumericDocValuesVisitor {
  type Output;

  fn visit<D: NumericDocValues>(self, values: Option<&mut D>) -> Self::Output;
}

macro_rules! either_numeric_docvalues {
    ($vis:vis $name:ident { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> DocValuesIterator for $name<$( $T ),+>
        where
            $( $T: DocValuesIterator ),+
        {
            fn advance_exact(&mut self, target: i32) -> Result<bool> {
                match self {
                    $( Self::$Variant(inner) => inner.advance_exact(target), )+
                }
            }
        }

        impl<$( $T ),+> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for $name<$( $T ),+>
        where
            $( $T: DocIdSetIterator ),+
        {}
        impl<$( $T ),+> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for $name<$( $T ),+>
        where
            $( $T: DocIdSetIterator ),+
        {}

        impl<$( $T ),+> DocIdSetIterator for $name<$( $T ),+>
        where
            $( $T: DocIdSetIterator ),+
        {
            fn doc_id(&self) -> i32 {
                match self {
                    $( Self::$Variant(inner) => inner.doc_id(), )+
                }
            }
            fn next_doc(&mut self) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.next_doc(), )+
                }
            }
            fn advance(&mut self, target: i32) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.advance(target), )+
                }
            }
            fn slow_advance(&mut self, target: i32) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.slow_advance(target), )+
                }
            }
            fn cost(&self) -> Result<i64> {
                match self {
                    $( Self::$Variant(inner) => inner.cost(), )+
                }
            }
        }

        impl<$( $T ),+> NumericDocValues for $name<$( $T ),+>
        where
            $( $T: NumericDocValues ),+
        {
            fn long_value(&mut self) -> Result<i64> {
                match self {
                    $( Self::$Variant(inner) => inner.long_value(), )+
                }
            }
        }
    };
}
either_numeric_docvalues!(pub NumericDocValuesEnum2 { A: F, B: B });

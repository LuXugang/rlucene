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
    /// [`advanceExact`](DocValuesIterator::advance_exact) returned `false`.
    ///
    /// # Returns
    /// The numeric value for the current document ID.
    fn long_value(&mut self) -> Result<i64>;
}

macro_rules! either_numeric_docvalues {
    ($vis:vis $name:ident { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> DocValuesIterator for $name<$( $T ),+>
        where
            $( $T: NumericDocValues ),+
        {
            fn advance_exact(&mut self, target: i32) -> Result<bool> {
                match self {
                    $( Self::$Variant(inner) => inner.advance_exact(target), )+
                }
            }
        }

        impl<$( $T ),+> DocIdSetIterator for $name<$( $T ),+>
        where
            $( $T: NumericDocValues ),+
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
either_numeric_docvalues!(pub NumericDocValuesEnum3 { A: F, B: B, C: C });
either_numeric_docvalues!(pub NumericDocValuesEnum4 { A: F, B: B, C: C, D: D });

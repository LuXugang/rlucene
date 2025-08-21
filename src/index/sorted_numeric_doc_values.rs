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
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::numeric_doc_values::{
    Either2NumericDocValues, Either4NumericDocValues, NumericDocValues,
};
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::Result;
/// A list of per-document numeric values, sorted according to i64's cmp.
pub trait SortedNumericDocValues: DocValuesIterator {
    /// Iterates to the next value in the current document. Do not call this
    /// more than
    /// [`doc_value_count`](SortedNumericDocValues::doc_value_count) times for
    /// the document.
    fn next_value(&mut self) -> Result<i64>;

    /// Retrieves the number of values for the current document. This must
    /// always be greater than zero. It is illegal to call this method after
    /// [`advance_exact(int)`](DocValuesIterator::advance_exact) returned
    /// `false`.
    fn doc_value_count(&mut self) -> Result<i32>;

    fn is_single_valued(&self) -> bool {
        false
    }
    type NumericDocValues: NumericDocValues;
    fn get_numeric_doc_values(&mut self) -> Result<Option<Self::NumericDocValues>> {
        Ok(None)
    }
}

macro_rules! either_sorted_numeric_docvalues {
    ($vis:vis $name:ident => $numdv:ident { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> { $( $Variant($T), )+ }

        // --- DocValuesIterator ---
        impl<$( $T ),+> DocValuesIterator for $name<$( $T ),+>
        where
            $( $T: SortedNumericDocValues ),+
        {
            #[inline]
            fn advance_exact(&mut self, target: i32) -> Result<bool> {
                match self {
                    $( Self::$Variant(inner) => inner.advance_exact(target), )+
                }
            }
        }

        impl<$( $T ),+> DocIdSetIterator for $name<$( $T ),+>
        where
            $( $T: SortedNumericDocValues ),+
        {
            #[inline]
            fn doc_id(&self) -> i32 {
                match self { $( Self::$Variant(inner) => inner.doc_id(), )+ }
            }
            #[inline]
            fn next_doc(&mut self) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.next_doc(), )+ }
            }
            #[inline]
            fn advance(&mut self, target: i32) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.advance(target), )+ }
            }
            #[inline]
            fn slow_advance(&mut self, target: i32) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.slow_advance(target), )+ }
            }
            #[inline]
            fn cost(&self) -> Result<i64> {
                match self { $( Self::$Variant(inner) => inner.cost(), )+ }
            }
        }

        impl<$( $T ),+> SortedNumericDocValues for $name<$( $T ),+>
        where
            $( $T: SortedNumericDocValues ),+
        {
            #[inline]
            fn next_value(&mut self) -> Result<i64> {
                match self { $( Self::$Variant(inner) => inner.next_value(), )+ }
            }
            #[inline]
            fn doc_value_count(&mut self) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.doc_value_count(), )+ }
            }
            #[inline]
            fn is_single_valued(&self) -> bool {
                match self { $( Self::$Variant(inner) => inner.is_single_valued(), )+ }
            }

            type NumericDocValues = $numdv<$( $T::NumericDocValues ),+>;

            #[inline]
            fn get_numeric_doc_values(&mut self) -> Result<Option<Self::NumericDocValues>> {
                match self {
                    $( Self::$Variant(inner) => {
                        let ndv = inner.get_numeric_doc_values()?;
                        Ok(ndv.map($numdv::$Variant))
                    } ),+
                }
            }
        }
    };
}
either_sorted_numeric_docvalues!(
    pub Either2SortedNumericDocValues
    => Either2NumericDocValues
    {
        F: F,
        S: S
    }
);

either_sorted_numeric_docvalues!(
    pub Either4SortedNumericDocValues
    => Either4NumericDocValues
    {
        F: F,
        S: S,
        T:T,
        U:U,
    }
);

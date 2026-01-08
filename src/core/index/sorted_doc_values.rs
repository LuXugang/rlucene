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
use std::borrow::Cow;

use crate::core::index::BytesRef;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::sorted_doc_values_terms_enum::SortedDocValuesTermsEnum;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::index::terms_enum::TermsEnumEnum2;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::ToInt;
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// A per-document `byte[]` with presorted values. This is fundamentally an
/// iterator over the `int` ord values per document, with random access APIs to
/// resolve an `int` ord to `BytesRef`.
///
/// Per-document values in a `SortedDocValues` are deduplicated, dereferenced,
/// and sorted into a dictionary of unique values. A pointer to the dictionary
/// value (ordinal) can be retrieved for each document. Ordinals are dense and
/// in increasing sorted order.
pub trait SortedDocValues: DocValuesIterator {
    /// Returns the ordinal for the current docID.
    ///
    /// This method must only be called after `advance_exact(doc_id)` returns
    /// `true`.
    ///
    /// # Returns
    /// A dense ordinal (starts at 0, then increments in sorted order).
    fn ord_value(&mut self) -> Result<i32>;

    /// Resolves the provided ordinal to the associated dictionary value.
    ///
    /// The returned `BytesRef` may be reused across calls,
    /// so if you want to keep it, make sure to deep-copy the value.
    ///
    /// # Arguments
    /// * `ord` - An ordinal in the range `[0, get_value_count())`
    ///
    /// # Returns
    /// The dictionary value corresponding to the ordinal.
    fn lookup_ord(&mut self, _ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        Err(LuceneError::need_implemented(
            "this method is not implemented",
        ))
    }

    /// Returns the number of unique sorted values in this doc values set.
    ///
    /// This is equivalent to one plus the maximum ordinal.
    fn get_value_count(&self) -> Result<i32> {
        Err(LuceneError::need_implemented(
            "this method is not implemented",
        ))
    }
    /// If `key` exists, returns its ordinal, else returns `-insertion_point -
    /// 1`, like `Arrays.binarySearch`.
    ///
    /// # Arguments
    /// * `key` - Key to look up
    ///
    /// # Returns
    /// * Ordinal of the key if found, otherwise `-insertion_point - 1`
    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        let mut low = 0;
        let mut high = self.get_value_count()? - 1;

        while low <= high {
            let mid = (low + high) >> 1;
            let term = self.lookup_ord(mid)?;
            let cmp = term.as_ref().cmp(key).to_int();
            if cmp < 0 {
                low = mid + 1;
            } else if cmp > 0 {
                high = mid - 1;
            } else {
                return Ok(mid); // key found
            }
        }
        Ok(-(low + 1)) // key not found
    }
    type TermsEnum<'a>: TermsEnum
    where
        Self: 'a;
    type TermsEnumRef: TermsEnum;
    /// Returns a [`TermsEnum`] over the
    /// values. The enum supports
    /// [`TermsEnum::ord`] and
    /// [`TermsEnum::seek_exact_with_ord`].
    fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>>;
    fn take_terms_enum(self) -> Result<Self::TermsEnumRef>;
    fn default_terms_enum(&mut self) -> Result<SortedDocValuesTermsEnum<&mut Self>>
    where
        Self: Sized,
    {
        Ok(SortedDocValuesTermsEnum::new(self))
    }
    fn default_take_terms_enum(self) -> Result<SortedDocValuesTermsEnum<Self>>
    where
        Self: Sized,
    {
        Ok(SortedDocValuesTermsEnum::new(self))
    }

    // TODO:
    // intersect not Implemented
}

macro_rules! either_sorted_docvalues {
    ($vis:vis $name:ident => $terms_enum:ident { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> { $( $Variant($T), )+ }

        // DocValuesIterator
        impl<$( $T ),+> DocValuesIterator for $name<$( $T ),+>
        where
            $( $T: SortedDocValues ),+
        {

            fn advance_exact(&mut self, target: i32) -> Result<bool> {
                match self {
                    $( Self::$Variant(inner) => inner.advance_exact(target), )+
                }
            }
        }

        // DocIdSetIterator
        impl<$( $T ),+> DocIdSetIterator for $name<$( $T ),+>
        where
            $( $T: SortedDocValues ),+
        {

            fn doc_id(&self) -> i32 {
                match self { $( Self::$Variant(inner) => inner.doc_id(), )+ }
            }

            fn next_doc(&mut self) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.next_doc(), )+ }
            }

            fn advance(&mut self, target: i32) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.advance(target), )+ }
            }

            fn slow_advance(&mut self, target: i32) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.slow_advance(target), )+ }
            }

            fn cost(&self) -> Result<i64> {
                match self { $( Self::$Variant(inner) => inner.cost(), )+ }
            }
        }

        // SortedDocValues
        impl<$( $T ),+> SortedDocValues for $name<$( $T ),+>
        where
            $( $T: SortedDocValues ),+
        {

            fn ord_value(&mut self) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.ord_value(), )+ }
            }

            fn lookup_ord(&mut self, _ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
                match self { $( Self::$Variant(inner) => inner.lookup_ord(_ord), )+ }
            }

            fn get_value_count(&self) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.get_value_count(), )+ }
            }

            fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.lookup_term(key), )+ }
            }

            type TermsEnum<'a> = $terms_enum<$( $T::TermsEnum<'a> ),+>
            where
                $( $T: 'a ),+;

            type TermsEnumRef = $terms_enum<$( $T::TermsEnumRef ),+>;

            fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
                match self {
                    $( Self::$Variant(inner) => {
                        let te = inner.terms_enum()?;
                        Ok($terms_enum::$Variant(te))
                    } ),+
                }
            }

            fn take_terms_enum(self) -> Result<Self::TermsEnumRef> {
                match self {
                    $( Self::$Variant(inner) => {
                        let te = inner.take_terms_enum()?;
                        Ok($terms_enum::$Variant(te))
                    } ),+
                }
            }
        }
    };
}
either_sorted_docvalues!(
    pub SortedDocValuesEnum2
    => TermsEnumEnum2
    { A: A, B: B }
);

impl<S> SortedDocValues for &mut S
where
    S: SortedDocValues,
{
    #[inline]
    fn ord_value(&mut self) -> Result<i32> {
        (**self).ord_value()
    }

    #[inline]
    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        (**self).lookup_ord(ord)
    }

    #[inline]
    fn get_value_count(&self) -> Result<i32> {
        (**self).get_value_count()
    }

    #[inline]
    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        (**self).lookup_term(key)
    }

    type TermsEnum<'a>
        = <S as SortedDocValues>::TermsEnum<'a>
    where
        Self: 'a;

    type TermsEnumRef = SortedDocValuesTermsEnum<Self>;

    #[inline]
    fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
        (**self).terms_enum()
    }

    #[inline]
    fn take_terms_enum(self) -> Result<Self::TermsEnumRef> {
        self.default_take_terms_enum()
    }
}

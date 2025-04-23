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

use crate::index::binary_doc_values::BinaryDocValues;
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::dummy::dummy_terms_enum::DummyTermsEnum;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
use crate::index::singleton_sorted_set_doc_values::SingletonSortedSetDocValues;
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::access::AccessVec;
use crate::util::error::lucene_error::Result;

pub struct DocValues;
impl DocValues {
    /// Returns a multi-valued view over the provided NumericDocValues.
    pub fn singleton_numeric<N>(dv: N) -> Result<SingletonSortedNumericDocValues<N>>
    where
        N: NumericDocValues,
    {
        SingletonSortedNumericDocValues::new(dv)
    }
    /// Returns a multi-valued view over the provided SortedDocValues.
    pub fn singleton_sorted<S, AV>(dv: S) -> Result<SingletonSortedSetDocValues<S, AV>>
    where
        S: SortedDocValues<AV>,
        AV: AccessVec<u8>,
    {
        SingletonSortedSetDocValues::new(dv)
    }
    /// An empty SortedNumericDocValues which returns zero values for every
    /// document.
    pub fn empty_sorted_numeric() -> Result<SingletonSortedNumericDocValues<EmptyNumeric>> {
        Self::singleton_numeric(EmptyNumeric::new())
    }
    /// An empty SortedDocValues which returns empty [`BytesRef`] for every
    /// document.
    pub fn empty_sorted_set() -> Result<SingletonSortedSetDocValues<EmptySorted, Vec<u8>>> {
        Self::singleton_sorted(EmptySorted::new())
    }

    /// Returns a single-valued view of the SortedSetDocValues, if it was
    /// previously wrapped with
    /// [`singleton_sorted`](DocValues::singleton_sorted), or null.
    pub fn unwrap_singleton_sorted_set_doc_values<I, AV, S>(
        dv: &impl SortedSetDocValues<AV>,
    ) -> Result<Option<S>>
    where
        AV: AccessVec<u8>,
        S: SortedDocValues<AV>,
    {
        todo!()
    }
    /// Returns a single-valued view of the SortedNumericDocValues, if it was
    /// previously wrapped with
    /// [`singleton_numeric`](DocValues::singleton_numeric), or null.
    pub fn unwrap_singleton_sorted_numeric_doc_values<N>(
        dv: &mut impl SortedNumericDocValues,
    ) -> Result<Option<N>>
    where
        N: NumericDocValues,
    {
        todo!()
        // match dv {
        //     SortedNumericDocValuesEnum::Singleton(singleton) => {
        //         let inner = singleton.get_numeric_doc_values()?;
        //         Ok(Some(inner))
        //     }
        //     _ => Ok(None),
        // }
    }
}
/// An empty [`BinaryDocValues`] which returns no documents */
pub struct EmptyBinary {
    doc: i32,
    bytes: BytesRef<Vec<u8>>,
}
impl Default for EmptyBinary {
    fn default() -> Self {
        Self::new()
    }
}
impl EmptyBinary {
    pub fn new() -> Self {
        Self {
            doc: -1,
            bytes: BytesRef::default(),
        }
    }
}

impl DocIdSetIterator for EmptyBinary {
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.doc = NO_MORE_DOCS;
        Ok(self.doc)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        self.doc = NO_MORE_DOCS;
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(0)
    }
}

impl DocValuesIterator for EmptyBinary {
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.doc = target;
        Ok(false)
    }
}
impl BinaryDocValues for EmptyBinary {
    fn binary_value(&mut self) -> Result<&BytesRef<Vec<u8>>> {
        debug_assert!(
            false,
            "EmptyBinary::binary_value() should not be called, as it is an empty iterator"
        );
        Ok(&self.bytes)
    }
}
/// An empty [`NumericDocValues`] which returns no documents */
pub struct EmptyNumeric {
    doc: i32,
}
impl Default for EmptyNumeric {
    fn default() -> Self {
        Self::new()
    }
}

impl EmptyNumeric {
    pub fn new() -> Self {
        Self { doc: -1 }
    }
}

impl DocValuesIterator for EmptyNumeric {}

impl DocIdSetIterator for EmptyNumeric {
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.doc = NO_MORE_DOCS;
        Ok(self.doc)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.doc = target;
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(0)
    }
}

impl NumericDocValues for EmptyNumeric {
    fn long_value(&mut self) -> Result<i64> {
        debug_assert!(false);
        Ok(0)
    }
}

/// An empty SortedDocValues which returns empty [`BytesRef`] for every
/// document.
pub struct EmptySorted {
    doc: i32,
    empty: BytesRef<Vec<u8>>,
}

impl Default for EmptySorted {
    fn default() -> Self {
        Self::new()
    }
}

impl EmptySorted {
    pub fn new() -> Self {
        Self {
            doc: -1,
            empty: BytesRef::default(),
        }
    }
}

impl DocValuesIterator for EmptySorted {
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.doc = target;
        Ok(false)
    }
}

impl DocIdSetIterator for EmptySorted {
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.doc = NO_MORE_DOCS;
        Ok(self.doc)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.doc = target;
        Ok(NO_MORE_DOCS)
    }

    fn cost(&self) -> Result<i64> {
        Ok(0)
    }
}

impl SortedDocValues<Vec<u8>> for EmptySorted {
    fn ord_value(&mut self) -> Result<i32> {
        debug_assert!(
            false,
            "EmptySorted should not be called, as it is an empty iterator"
        );
        Ok(-1)
    }

    fn lookup_ord(&mut self, _ord: i32) -> Result<Cow<BytesRef<Vec<u8>>>> {
        Ok(Cow::Owned(std::mem::take(&mut self.empty)))
    }

    fn get_value_count(&mut self) -> Result<i32> {
        Ok(0)
    }

    type TermsEnum = DummyTermsEnum;
}

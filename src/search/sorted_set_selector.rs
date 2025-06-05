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

use crate::codecs::doc_values_enum::doc_values::SortedSetDocValuesEnum;
use crate::codecs::lucene90_doc_values_enums::Lucene90SortedSetDocValuesEnum;
use crate::codecs::lucene90_doc_values_producer::{BaseSortedDocValues, BaseSortedSetDocValues};
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::dummy::dummy_terms_enum::DummyTermsEnum;
use crate::index::singleton_sorted_set_doc_values::SingletonSortedSetDocValues;
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::IndexInput;
use crate::util::error::lucene_error::{LuceneError, Result};

/// Selects a value from the document's set to use as the representative value.
pub struct SortedSetSelector;
impl SortedSetSelector {
    /// Wraps a multi-valued SortedSetDocValues as a single-valued view, using
    /// the specified selector.
    pub fn wrap<I>(
        mut sorted_set: SortedSetDocValuesEnum<I>,
        selector: SortedSetSelectorType,
    ) -> Result<SortedDocValuesWrapEnum<I>>
    where
        I: IndexInput,
    {
        if sorted_set.get_value_count()? >= i32::MAX as i64 {
            return Err(LuceneError::unsupported_operation(format!(
                "fields containing more than {} unique terms are unsupported",
                i32::MAX - 1
            )));
        }
        match sorted_set {
            SortedSetDocValuesEnum::Lucene90(inner) => match inner {
                Lucene90SortedSetDocValuesEnum::Singleton(single) => {
                    Ok(SortedDocValuesWrapEnum::Lucene90Singleton(single))
                },
                Lucene90SortedSetDocValuesEnum::Base(base) => {
                    let wrapped = match selector {
                        SortedSetSelectorType::Min => {
                            SortedDocValuesWrapEnum::Min(MinValue::new(base))
                        },
                        SortedSetSelectorType::Max => {
                            SortedDocValuesWrapEnum::Max(MaxValue::new(base))
                        },
                        SortedSetSelectorType::MiddleMin => {
                            SortedDocValuesWrapEnum::MiddleMin(MiddleMinValue::new(base))
                        },
                        SortedSetSelectorType::MiddleMax => {
                            SortedDocValuesWrapEnum::MiddleMax(MiddleMaxValue::new(base))
                        },
                    };
                    Ok(wrapped)
                },
            },
        }
    }
}
/// Type of selection to perform.
///
/// # Limitations
/// - Fields containing `i32::MAX` or more unique values are unsupported.
/// - Selectors other than [`SortedSetSelectorType::Min`] require optional codec
///   support. However, several codecs provided by Lucene, including the current
///   default codec, support this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortedSetSelectorType {
    /// Selects the minimum value in the set.
    Min,
    /// Selects the maximum value in the set.
    Max,
    /// Selects the middle value in the set.
    ///
    /// If the set has an even number of values, the lower of the middle two is
    /// chosen.
    MiddleMin,
    /// Selects the middle value in the set.
    ///
    /// If the set has an even number of values, the higher of the middle two
    /// is chosen.
    MiddleMax,
}
/// Wraps a SortedSetDocValues and returns the first ordinal (min)
pub struct MinValue<I>
where
    I: IndexInput,
{
    inner: BaseSortedSetDocValues<I>,
    ord: i32,
}

impl<I> MinValue<I>
where
    I: IndexInput,
{
    fn new(inner: BaseSortedSetDocValues<I>) -> Self {
        Self { inner, ord: 0 }
    }
    fn set_ord(&mut self) -> Result<()> {
        if self.doc_id() != NO_MORE_DOCS {
            self.ord = self.inner.next_ord()? as i32
        }
        Ok(())
    }
}

impl<I> DocValuesIterator for MinValue<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.advance_exact(target)? {
            self.set_ord()?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl<I> DocIdSetIterator for MinValue<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.inner.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.inner.next_doc()?;
        self.set_ord()?;
        Ok(self.doc_id())
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.inner.advance(target)?;
        self.set_ord()?;
        Ok(self.doc_id())
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.ord as i64)
    }
}

impl<I> SortedDocValues for MinValue<I>
where
    I: IndexInput,
{
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ord)
    }

    type AV = Vec<u8>;

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<BytesRef<Vec<u8>>>> {
        self.inner.lookup_ord(ord as i64)
    }

    fn get_value_count(&mut self) -> Result<i32> {
        Ok(self.inner.get_value_count()? as i32)
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        Ok(self.inner.lookup_term(key)? as i32)
    }
    type TermsEnum = DummyTermsEnum<Self::AV>;
}
/// Wraps a SortedSetDocValues and returns the last ordinal (max)
pub struct MaxValue<I>
where
    I: IndexInput,
{
    inner: BaseSortedSetDocValues<I>,
    ord: i32,
}

impl<I> MaxValue<I>
where
    I: IndexInput,
{
    fn new(inner: BaseSortedSetDocValues<I>) -> Self {
        Self { inner, ord: 0 }
    }

    fn set_ord(&mut self) -> Result<()> {
        if self.doc_id() != NO_MORE_DOCS {
            let doc_value_count = self.inner.doc_value_count()?;
            for _ in 0..(doc_value_count - 1) {
                self.inner.next_ord()?;
            }
            self.ord = self.inner.next_ord()? as i32;
        }
        Ok(())
    }
}

impl<I> DocValuesIterator for MaxValue<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.advance_exact(target)? {
            self.set_ord()?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl<I> DocIdSetIterator for MaxValue<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.inner.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.inner.next_doc()?;
        self.set_ord()?;
        Ok(self.doc_id())
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.inner.advance(target)?;
        self.set_ord()?;
        Ok(self.doc_id())
    }

    fn cost(&self) -> Result<i64> {
        self.inner.cost()
    }
}

impl<I> SortedDocValues for MaxValue<I>
where
    I: IndexInput,
{
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ord)
    }

    type AV = Vec<u8>;

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<BytesRef<Vec<u8>>>> {
        self.inner.lookup_ord(ord as i64)
    }

    fn get_value_count(&mut self) -> Result<i32> {
        Ok(self.inner.get_value_count()? as i32)
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        Ok(self.inner.lookup_term(key)? as i32)
    }

    type TermsEnum = DummyTermsEnum<Self::AV>;
}
/// Wraps a SortedSetDocValues and returns the middle ordinal (or min of the
/// two)
pub struct MiddleMinValue<I>
where
    I: IndexInput,
{
    inner: BaseSortedSetDocValues<I>,
    ord: i32,
}

impl<I> MiddleMinValue<I>
where
    I: IndexInput,
{
    fn new(inner: BaseSortedSetDocValues<I>) -> Self {
        Self { inner, ord: 0 }
    }

    fn set_ord(&mut self) -> Result<()> {
        if self.doc_id() != NO_MORE_DOCS {
            let doc_value_count = self.inner.doc_value_count()?;
            let target_idx = (doc_value_count - 1) >> 1;
            for _ in 0..target_idx {
                self.inner.next_ord()?;
            }
            self.ord = self.inner.next_ord()? as i32;
        }
        Ok(())
    }
}

impl<I> DocValuesIterator for MiddleMinValue<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.advance_exact(target)? {
            self.set_ord()?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl<I> DocIdSetIterator for MiddleMinValue<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.inner.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.inner.next_doc()?;
        self.set_ord()?;
        Ok(self.doc_id())
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.inner.advance(target)?;
        self.set_ord()?;
        Ok(self.doc_id())
    }

    fn cost(&self) -> Result<i64> {
        self.inner.cost()
    }
}

impl<I> SortedDocValues for MiddleMinValue<I>
where
    I: IndexInput,
{
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ord)
    }

    type AV = Vec<u8>;

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<BytesRef<Vec<u8>>>> {
        self.inner.lookup_ord(ord as i64)
    }

    fn get_value_count(&mut self) -> Result<i32> {
        Ok(self.inner.get_value_count()? as i32)
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        Ok(self.inner.lookup_term(key)? as i32)
    }
    type TermsEnum = DummyTermsEnum<Self::AV>;
}
/// Wraps a SortedSetDocValues and returns the middle ordinal (or max of the
/// two)
pub struct MiddleMaxValue<I>
where
    I: IndexInput,
{
    inner: BaseSortedSetDocValues<I>,
    ord: i32,
}

impl<I> MiddleMaxValue<I>
where
    I: IndexInput,
{
    fn new(inner: BaseSortedSetDocValues<I>) -> Self {
        Self { inner, ord: 0 }
    }

    fn set_ord(&mut self) -> Result<()> {
        if self.doc_id() != NO_MORE_DOCS {
            let count = self.inner.doc_value_count()?;
            let target_idx = ((count as u64) >> 1) as i32;
            for _ in 0..target_idx {
                self.inner.next_ord()?;
            }
            self.ord = self.inner.next_ord()? as i32;
        }
        Ok(())
    }
}

impl<I> DocValuesIterator for MiddleMaxValue<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.advance_exact(target)? {
            self.set_ord()?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl<I> DocIdSetIterator for MiddleMaxValue<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.inner.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.inner.next_doc()?;
        self.set_ord()?;
        Ok(self.doc_id())
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.inner.advance(target)?;
        self.set_ord()?;
        Ok(self.doc_id())
    }

    fn cost(&self) -> Result<i64> {
        self.inner.cost()
    }
}

impl<I> SortedDocValues for MiddleMaxValue<I>
where
    I: IndexInput,
{
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ord)
    }

    type AV = Vec<u8>;

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<BytesRef<Vec<u8>>>> {
        self.inner.lookup_ord(ord as i64)
    }

    fn get_value_count(&mut self) -> Result<i32> {
        Ok(self.inner.get_value_count()? as i32)
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        Ok(self.inner.lookup_term(key)? as i32)
    }
    type TermsEnum = DummyTermsEnum<Self::AV>;
}

pub enum SortedDocValuesWrapEnum<I>
where
    I: IndexInput,
{
    Lucene90Singleton(SingletonSortedSetDocValues<BaseSortedDocValues<I>>),
    Min(MinValue<I>),
    Max(MaxValue<I>),
    MiddleMin(MiddleMinValue<I>),
    MiddleMax(MiddleMaxValue<I>),
}

impl<I> DocValuesIterator for SortedDocValuesWrapEnum<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        match self {
            SortedDocValuesWrapEnum::Lucene90Singleton(inner) => {
                inner.inner.as_mut().unwrap().advance_exact(target)
            },
            SortedDocValuesWrapEnum::Min(inner) => inner.advance_exact(target),
            SortedDocValuesWrapEnum::Max(inner) => inner.advance_exact(target),
            SortedDocValuesWrapEnum::MiddleMin(inner) => inner.advance_exact(target),
            SortedDocValuesWrapEnum::MiddleMax(inner) => inner.advance_exact(target),
        }
    }
}

impl<I> DocIdSetIterator for SortedDocValuesWrapEnum<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        match self {
            SortedDocValuesWrapEnum::Lucene90Singleton(inner) => {
                inner.inner.as_ref().unwrap().doc_id()
            },
            SortedDocValuesWrapEnum::Min(inner) => inner.doc_id(),
            SortedDocValuesWrapEnum::Max(inner) => inner.doc_id(),
            SortedDocValuesWrapEnum::MiddleMin(inner) => inner.doc_id(),
            SortedDocValuesWrapEnum::MiddleMax(inner) => inner.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            SortedDocValuesWrapEnum::Lucene90Singleton(inner) => {
                inner.inner.as_mut().unwrap().next_doc()
            },
            SortedDocValuesWrapEnum::Min(inner) => inner.next_doc(),
            SortedDocValuesWrapEnum::Max(inner) => inner.next_doc(),
            SortedDocValuesWrapEnum::MiddleMin(inner) => inner.next_doc(),
            SortedDocValuesWrapEnum::MiddleMax(inner) => inner.next_doc(),
        }
    }
}

impl<I> SortedDocValues for SortedDocValuesWrapEnum<I>
where
    I: IndexInput,
{
    fn ord_value(&mut self) -> Result<i32> {
        match self {
            SortedDocValuesWrapEnum::Lucene90Singleton(inner) => {
                inner.inner.as_mut().unwrap().ord_value()
            },
            SortedDocValuesWrapEnum::Min(inner) => inner.ord_value(),
            SortedDocValuesWrapEnum::Max(inner) => inner.ord_value(),
            SortedDocValuesWrapEnum::MiddleMin(inner) => inner.ord_value(),
            SortedDocValuesWrapEnum::MiddleMax(inner) => inner.ord_value(),
        }
    }

    type AV = Vec<u8>;

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<BytesRef<Vec<u8>>>> {
        match self {
            SortedDocValuesWrapEnum::Lucene90Singleton(inner) => {
                inner.inner.as_mut().unwrap().lookup_ord(ord)
            },
            SortedDocValuesWrapEnum::Min(inner) => inner.lookup_ord(ord),
            SortedDocValuesWrapEnum::Max(inner) => inner.lookup_ord(ord),
            SortedDocValuesWrapEnum::MiddleMin(inner) => inner.lookup_ord(ord),
            SortedDocValuesWrapEnum::MiddleMax(inner) => inner.lookup_ord(ord),
        }
    }

    fn get_value_count(&mut self) -> Result<i32> {
        match self {
            SortedDocValuesWrapEnum::Lucene90Singleton(inner) => {
                inner.inner.as_mut().unwrap().get_value_count()
            },
            SortedDocValuesWrapEnum::Min(inner) => inner.get_value_count(),
            SortedDocValuesWrapEnum::Max(inner) => inner.get_value_count(),
            SortedDocValuesWrapEnum::MiddleMin(inner) => inner.get_value_count(),
            SortedDocValuesWrapEnum::MiddleMax(inner) => inner.get_value_count(),
        }
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        match self {
            SortedDocValuesWrapEnum::Lucene90Singleton(inner) => {
                inner.inner.as_mut().unwrap().lookup_term(key)
            },
            SortedDocValuesWrapEnum::Min(inner) => inner.lookup_term(key),
            SortedDocValuesWrapEnum::Max(inner) => inner.lookup_term(key),
            SortedDocValuesWrapEnum::MiddleMin(inner) => inner.lookup_term(key),
            SortedDocValuesWrapEnum::MiddleMax(inner) => inner.lookup_term(key),
        }
    }
    type TermsEnum = DummyTermsEnum<Self::AV>;
}

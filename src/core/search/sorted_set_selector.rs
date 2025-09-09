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
use crate::core::index::doc_values::DocValues;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::sorted_doc_values_terms_enum::SortedDocValuesTermsEnum;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Selects a value from the document's set to use as the representative value.
pub struct SortedSetSelector;
impl SortedSetSelector {
    /// Wraps a multi-valued SortedSetDocValues as a single-valued view, using
    /// the specified selector.
    pub fn wrap<S>(
        mut sorted_set: S,
        selector: SortedSetSelectorType,
    ) -> Result<SortedDocValuesWrapEnum<S>>
    where
        S: SortedSetDocValues,
    {
        if sorted_set.get_value_count()? >= i32::MAX as i64 {
            return Err(LuceneError::unsupported_operation(format!(
                "fields containing more than {} unique terms are unsupported",
                i32::MAX - 1
            )));
        }
        let singleton = DocValues::unwrap_singleton_sorted(&mut sorted_set)?;
        match singleton {
            Some(single) => Ok(SortedDocValuesWrapEnum::Singleton(single)),
            None => {
                let v = match selector {
                    SortedSetSelectorType::Min => {
                        SortedDocValuesWrapEnum::Min(MinValue::new(sorted_set))
                    },
                    SortedSetSelectorType::Max => {
                        SortedDocValuesWrapEnum::Max(MaxValue::new(sorted_set))
                    },
                    SortedSetSelectorType::MiddleMin => {
                        SortedDocValuesWrapEnum::MiddleMin(MiddleMinValue::new(sorted_set))
                    },
                    SortedSetSelectorType::MiddleMax => {
                        SortedDocValuesWrapEnum::MiddleMax(MiddleMaxValue::new(sorted_set))
                    },
                };
                Ok(v)
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
pub struct MinValue<S>
where
    S: SortedSetDocValues,
{
    inner: S,
    ord: i32,
}

impl<S> MinValue<S>
where
    S: SortedSetDocValues,
{
    fn new(inner: S) -> Self {
        Self { inner, ord: 0 }
    }
    fn set_ord(&mut self) -> Result<()> {
        if self.doc_id() != NO_MORE_DOCS {
            self.ord = self.inner.next_ord()? as i32
        }
        Ok(())
    }
}

impl<S> DocValuesIterator for MinValue<S>
where
    S: SortedSetDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.advance_exact(target)? {
            self.set_ord()?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl<S> DocIdSetIterator for MinValue<S>
where
    S: SortedSetDocValues,
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

impl<S> SortedDocValues for MinValue<S>
where
    S: SortedSetDocValues,
{
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ord)
    }

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        self.inner.lookup_ord(ord as i64)
    }

    fn get_value_count(&mut self) -> Result<i32> {
        Ok(self.inner.get_value_count()? as i32)
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        Ok(self.inner.lookup_term(key)? as i32)
    }
    type TermsEnum = SortedDocValuesTermsEnum;
}
/// Wraps a SortedSetDocValues and returns the last ordinal (max)
pub struct MaxValue<S>
where
    S: SortedSetDocValues,
{
    inner: S,
    ord: i32,
}

impl<S> MaxValue<S>
where
    S: SortedSetDocValues,
{
    fn new(inner: S) -> Self {
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

impl<S> DocValuesIterator for MaxValue<S>
where
    S: SortedSetDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.advance_exact(target)? {
            self.set_ord()?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl<S> DocIdSetIterator for MaxValue<S>
where
    S: SortedSetDocValues,
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

impl<S> SortedDocValues for MaxValue<S>
where
    S: SortedSetDocValues,
{
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ord)
    }

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        self.inner.lookup_ord(ord as i64)
    }

    fn get_value_count(&mut self) -> Result<i32> {
        Ok(self.inner.get_value_count()? as i32)
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        Ok(self.inner.lookup_term(key)? as i32)
    }

    type TermsEnum = SortedDocValuesTermsEnum;
}
/// Wraps a SortedSetDocValues and returns the middle ordinal (or min of the
/// two)
pub struct MiddleMinValue<S>
where
    S: SortedSetDocValues,
{
    inner: S,
    ord: i32,
}

impl<S> MiddleMinValue<S>
where
    S: SortedSetDocValues,
{
    fn new(inner: S) -> Self {
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

impl<S> DocValuesIterator for MiddleMinValue<S>
where
    S: SortedSetDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.advance_exact(target)? {
            self.set_ord()?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl<S> DocIdSetIterator for MiddleMinValue<S>
where
    S: SortedSetDocValues,
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

impl<S> SortedDocValues for MiddleMinValue<S>
where
    S: SortedSetDocValues,
{
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ord)
    }

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        self.inner.lookup_ord(ord as i64)
    }

    fn get_value_count(&mut self) -> Result<i32> {
        Ok(self.inner.get_value_count()? as i32)
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        Ok(self.inner.lookup_term(key)? as i32)
    }
    type TermsEnum = SortedDocValuesTermsEnum;
}
/// Wraps a SortedSetDocValues and returns the middle ordinal (or max of the
/// two)
pub struct MiddleMaxValue<S>
where
    S: SortedSetDocValues,
{
    inner: S,
    ord: i32,
}

impl<S> MiddleMaxValue<S>
where
    S: SortedSetDocValues,
{
    fn new(inner: S) -> Self {
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

impl<S> DocValuesIterator for MiddleMaxValue<S>
where
    S: SortedSetDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.advance_exact(target)? {
            self.set_ord()?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl<S> DocIdSetIterator for MiddleMaxValue<S>
where
    S: SortedSetDocValues,
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

impl<S> SortedDocValues for MiddleMaxValue<S>
where
    S: SortedSetDocValues,
{
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ord)
    }

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        self.inner.lookup_ord(ord as i64)
    }

    fn get_value_count(&mut self) -> Result<i32> {
        Ok(self.inner.get_value_count()? as i32)
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        Ok(self.inner.lookup_term(key)? as i32)
    }
    type TermsEnum = SortedDocValuesTermsEnum;
}

pub enum SortedDocValuesWrapEnum<S>
where
    S: SortedSetDocValues,
{
    Singleton(S::SortedDocValues),
    Min(MinValue<S>),
    Max(MaxValue<S>),
    MiddleMin(MiddleMinValue<S>),
    MiddleMax(MiddleMaxValue<S>),
}

impl<S> DocValuesIterator for SortedDocValuesWrapEnum<S>
where
    S: SortedSetDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        match self {
            SortedDocValuesWrapEnum::Singleton(single) => single.advance_exact(target),
            SortedDocValuesWrapEnum::Min(min) => min.advance_exact(target),
            SortedDocValuesWrapEnum::Max(max) => max.advance_exact(target),
            SortedDocValuesWrapEnum::MiddleMin(middle_min) => middle_min.advance_exact(target),
            SortedDocValuesWrapEnum::MiddleMax(middle_max) => middle_max.advance_exact(target),
        }
    }
}

impl<S> DocIdSetIterator for SortedDocValuesWrapEnum<S>
where
    S: SortedSetDocValues,
{
    fn doc_id(&self) -> i32 {
        match self {
            SortedDocValuesWrapEnum::Singleton(single) => single.doc_id(),
            SortedDocValuesWrapEnum::Min(min) => min.doc_id(),
            SortedDocValuesWrapEnum::Max(max) => max.doc_id(),
            SortedDocValuesWrapEnum::MiddleMin(middle_min) => middle_min.doc_id(),
            SortedDocValuesWrapEnum::MiddleMax(middle_max) => middle_max.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            SortedDocValuesWrapEnum::Singleton(single) => single.next_doc(),
            SortedDocValuesWrapEnum::Min(min) => min.next_doc(),
            SortedDocValuesWrapEnum::Max(max) => max.next_doc(),
            SortedDocValuesWrapEnum::MiddleMin(middle_min) => middle_min.next_doc(),
            SortedDocValuesWrapEnum::MiddleMax(middle_max) => middle_max.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            SortedDocValuesWrapEnum::Singleton(single) => single.advance(target),
            SortedDocValuesWrapEnum::Min(min) => min.advance(target),
            SortedDocValuesWrapEnum::Max(max) => max.advance(target),
            SortedDocValuesWrapEnum::MiddleMin(middle_min) => middle_min.advance(target),
            SortedDocValuesWrapEnum::MiddleMax(middle_max) => middle_max.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            SortedDocValuesWrapEnum::Singleton(single) => single.slow_advance(target),
            SortedDocValuesWrapEnum::Min(min) => min.slow_advance(target),
            SortedDocValuesWrapEnum::Max(max) => max.slow_advance(target),
            SortedDocValuesWrapEnum::MiddleMin(middle_min) => middle_min.slow_advance(target),
            SortedDocValuesWrapEnum::MiddleMax(middle_max) => middle_max.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            SortedDocValuesWrapEnum::Singleton(single) => single.cost(),
            SortedDocValuesWrapEnum::Min(min) => min.cost(),
            SortedDocValuesWrapEnum::Max(max) => max.cost(),
            SortedDocValuesWrapEnum::MiddleMin(middle_min) => middle_min.cost(),
            SortedDocValuesWrapEnum::MiddleMax(middle_max) => middle_max.cost(),
        }
    }
}

impl<S> SortedDocValues for SortedDocValuesWrapEnum<S>
where
    S: SortedSetDocValues,
{
    fn ord_value(&mut self) -> Result<i32> {
        match self {
            SortedDocValuesWrapEnum::Singleton(single) => single.ord_value(),
            SortedDocValuesWrapEnum::Min(min) => min.ord_value(),
            SortedDocValuesWrapEnum::Max(max) => max.ord_value(),
            SortedDocValuesWrapEnum::MiddleMin(middle_min) => middle_min.ord_value(),
            SortedDocValuesWrapEnum::MiddleMax(middle_max) => middle_max.ord_value(),
        }
    }

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        match self {
            SortedDocValuesWrapEnum::Singleton(single) => single.lookup_ord(ord),
            SortedDocValuesWrapEnum::Min(min) => min.lookup_ord(ord),
            SortedDocValuesWrapEnum::Max(max) => max.lookup_ord(ord),
            SortedDocValuesWrapEnum::MiddleMin(middle_min) => middle_min.lookup_ord(ord),
            SortedDocValuesWrapEnum::MiddleMax(middle_max) => middle_max.lookup_ord(ord),
        }
    }

    fn get_value_count(&mut self) -> Result<i32> {
        match self {
            SortedDocValuesWrapEnum::Singleton(single) => single.get_value_count(),
            SortedDocValuesWrapEnum::Min(min) => min.get_value_count(),
            SortedDocValuesWrapEnum::Max(max) => max.get_value_count(),
            SortedDocValuesWrapEnum::MiddleMin(middle_min) => middle_min.get_value_count(),
            SortedDocValuesWrapEnum::MiddleMax(middle_max) => middle_max.get_value_count(),
        }
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        match self {
            SortedDocValuesWrapEnum::Singleton(single) => single.lookup_term(key),
            SortedDocValuesWrapEnum::Min(min) => min.lookup_term(key),
            SortedDocValuesWrapEnum::Max(max) => max.lookup_term(key),
            SortedDocValuesWrapEnum::MiddleMin(middle_min) => middle_min.lookup_term(key),
            SortedDocValuesWrapEnum::MiddleMax(middle_max) => middle_max.lookup_term(key),
        }
    }

    type TermsEnum = SortedDocValuesTermsEnum;
}

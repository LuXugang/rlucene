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
use crate::codecs::doc_values_enum::doc_values::{
    SortedDocValuesEnum, SortedSetDocValuesEnum, SortedSetDocValuesWrapper,
};
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::index::terms_enums::TermsEnums;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::IndexInput;
use crate::util::error::lucene_error::{LuceneError, Result};
use std::cell::RefCell;
use std::rc::Rc;
/// Selects a value from the document's set to use as the representative value.
pub struct SortedSetSelector;
impl SortedSetSelector {
    /// Wraps a multi-valued SortedSetDocValues as a single-valued view, using the specified selector.
    pub fn wrap<I: IndexInput>(
        sorted_set: SortedSetDocValuesEnum<I>,
        selector: SortedSetSelectorType,
    ) -> Result<Rc<RefCell<SortedDocValuesEnum<I>>>> {
        if sorted_set.get_value_count()? >= i32::MAX as i64 {
            return Err(LuceneError::unsupported_operation(format!(
                "fields containing more than {} unique terms are unsupported",
                i32::MAX - 1
            )));
        }
        match sorted_set {
            SortedSetDocValuesEnum::Singleton(inner) => inner.get_numeric_doc_values(),
            SortedSetDocValuesEnum::Other(inner) => {
                let wrapped = match selector {
                    SortedSetSelectorType::Min => SortedDocValuesEnum1::Min(MinValue::new(inner)),
                    SortedSetSelectorType::Max => SortedDocValuesEnum1::Max(MaxValue::new(inner)),
                    SortedSetSelectorType::MiddleMin => {
                        SortedDocValuesEnum1::MiddleMin(MiddleMinValue::new(inner))
                    }
                    SortedSetSelectorType::MiddleMax => {
                        SortedDocValuesEnum1::MiddleMax(MiddleMaxValue::new(inner))
                    }
                };
                Ok(Rc::new(RefCell::new(SortedDocValuesEnum::Impl(wrapped))))
            }
        }
    }
}
/// Type of selection to perform.
///
/// # Limitations
/// - Fields containing `i32::MAX` or more unique values are unsupported.
/// - Selectors other than [`SortedSetSelectorType::Min`] require optional codec support. However, several
///   codecs provided by Lucene, including the current default codec, support this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortedSetSelectorType {
    /// Selects the minimum value in the set.
    Min,
    /// Selects the maximum value in the set.
    Max,
    /// Selects the middle value in the set.
    ///
    /// If the set has an even number of values, the lower of the middle two is chosen.
    MiddleMin,
    /// Selects the middle value in the set.
    ///
    /// If the set has an even number of values, the higher of the middle two is chosen.
    MiddleMax,
}
/// Wraps a SortedSetDocValues and returns the first ordinal (min)
struct MinValue<I>
where
    I: IndexInput,
{
    inner: Box<SortedSetDocValuesWrapper<I>>,
    ord: i32,
}

impl<I> MinValue<I>
where
    I: IndexInput,
{
    fn new(inner: Box<SortedSetDocValuesWrapper<I>>) -> Self {
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

impl<I> SortedDocValues<I> for MinValue<I>
where
    I: IndexInput,
{
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ord)
    }

    fn lookup_ord(&mut self, ord: i32) -> Result<BytesRef> {
        self.inner.lookup_ord(ord as i64)
    }

    fn get_value_count(&self) -> Result<i32> {
        Ok(self.inner.get_value_count()? as i32)
    }

    fn lookup_term(&mut self, key: &BytesRef) -> Result<i32> {
        Ok(self.inner.lookup_term(key)? as i32)
    }
}
/// Wraps a SortedSetDocValues and returns the last ordinal (max)
struct MaxValue<I: IndexInput> {
    inner: Box<SortedSetDocValuesWrapper<I>>,
    ord: i32,
}

impl<I: IndexInput> MaxValue<I> {
    fn new(inner: Box<SortedSetDocValuesWrapper<I>>) -> Self {
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

impl<I: IndexInput> DocValuesIterator for MaxValue<I> {
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.advance_exact(target)? {
            self.set_ord()?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl<I: IndexInput> DocIdSetIterator for MaxValue<I> {
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

impl<I: IndexInput> SortedDocValues<I> for MaxValue<I> {
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ord)
    }

    fn lookup_ord(&mut self, ord: i32) -> Result<BytesRef> {
        self.inner.lookup_ord(ord as i64)
    }

    fn get_value_count(&self) -> Result<i32> {
        Ok(self.inner.get_value_count()? as i32)
    }

    fn lookup_term(&mut self, key: &BytesRef) -> Result<i32> {
        Ok(self.inner.lookup_term(key)? as i32)
    }
}
/// Wraps a SortedSetDocValues and returns the middle ordinal (or min of the two)
struct MiddleMinValue<I: IndexInput> {
    inner: Box<SortedSetDocValuesWrapper<I>>,
    ord: i32,
}

impl<I: IndexInput> MiddleMinValue<I> {
    fn new(inner: Box<SortedSetDocValuesWrapper<I>>) -> Self {
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

impl<I: IndexInput> DocValuesIterator for MiddleMinValue<I> {
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.advance_exact(target)? {
            self.set_ord()?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl<I: IndexInput> DocIdSetIterator for MiddleMinValue<I> {
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

impl<I: IndexInput> SortedDocValues<I> for MiddleMinValue<I> {
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ord)
    }

    fn lookup_ord(&mut self, ord: i32) -> Result<BytesRef> {
        self.inner.lookup_ord(ord as i64)
    }

    fn get_value_count(&self) -> Result<i32> {
        Ok(self.inner.get_value_count()? as i32)
    }

    fn lookup_term(&mut self, key: &BytesRef) -> Result<i32> {
        Ok(self.inner.lookup_term(key)? as i32)
    }
}
/// Wraps a SortedSetDocValues and returns the middle ordinal (or max of the two)
struct MiddleMaxValue<I: IndexInput> {
    inner: Box<SortedSetDocValuesWrapper<I>>,
    ord: i32,
}

impl<I: IndexInput> MiddleMaxValue<I> {
    fn new(inner: Box<SortedSetDocValuesWrapper<I>>) -> Self {
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

impl<I: IndexInput> DocValuesIterator for MiddleMaxValue<I> {
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.advance_exact(target)? {
            self.set_ord()?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl<I: IndexInput> DocIdSetIterator for MiddleMaxValue<I> {
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

impl<I: IndexInput> SortedDocValues<I> for MiddleMaxValue<I> {
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ord)
    }

    fn lookup_ord(&mut self, ord: i32) -> Result<BytesRef> {
        self.inner.lookup_ord(ord as i64)
    }

    fn get_value_count(&self) -> Result<i32> {
        Ok(self.inner.get_value_count()? as i32)
    }

    fn lookup_term(&mut self, key: &BytesRef) -> Result<i32> {
        Ok(self.inner.lookup_term(key)? as i32)
    }
}

pub enum SortedDocValuesEnum1<I: IndexInput> {
    Min(MinValue<I>),
    Max(MaxValue<I>),
    MiddleMin(MiddleMinValue<I>),
    MiddleMax(MiddleMaxValue<I>),
}

impl<I> DocValuesIterator for SortedDocValuesEnum1<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        match self {
            SortedDocValuesEnum1::Min(inner) => inner.advance_exact(target),
            SortedDocValuesEnum1::Max(inner) => inner.advance_exact(target),
            SortedDocValuesEnum1::MiddleMin(inner) => inner.advance_exact(target),
            SortedDocValuesEnum1::MiddleMax(inner) => inner.advance_exact(target),
        }
    }
}

impl<I> DocIdSetIterator for SortedDocValuesEnum1<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        match self {
            SortedDocValuesEnum1::Min(inner) => inner.doc_id(),
            SortedDocValuesEnum1::Max(inner) => inner.doc_id(),
            SortedDocValuesEnum1::MiddleMin(inner) => inner.doc_id(),
            SortedDocValuesEnum1::MiddleMax(inner) => inner.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            SortedDocValuesEnum1::Min(inner) => inner.next_doc(),
            SortedDocValuesEnum1::Max(inner) => inner.next_doc(),
            SortedDocValuesEnum1::MiddleMin(inner) => inner.next_doc(),
            SortedDocValuesEnum1::MiddleMax(inner) => inner.next_doc(),
        }
    }
}

impl<I> SortedDocValues<I> for SortedDocValuesEnum1<I>
where
    I: IndexInput,
{
    fn ord_value(&mut self) -> Result<i32> {
        match self {
            SortedDocValuesEnum1::Min(inner) => inner.ord_value(),
            SortedDocValuesEnum1::Max(inner) => inner.ord_value(),
            SortedDocValuesEnum1::MiddleMin(inner) => inner.ord_value(),
            SortedDocValuesEnum1::MiddleMax(inner) => inner.ord_value(),
        }
    }

    fn lookup_ord(&mut self, ord: i32) -> Result<BytesRef> {
        match self {
            SortedDocValuesEnum1::Min(inner) => inner.lookup_ord(ord),
            SortedDocValuesEnum1::Max(inner) => inner.lookup_ord(ord),
            SortedDocValuesEnum1::MiddleMin(inner) => inner.lookup_ord(ord),
            SortedDocValuesEnum1::MiddleMax(inner) => inner.lookup_ord(ord),
        }
    }

    fn get_value_count(&self) -> Result<i32> {
        match self {
            SortedDocValuesEnum1::Min(inner) => inner.get_value_count(),
            SortedDocValuesEnum1::Max(inner) => inner.get_value_count(),
            SortedDocValuesEnum1::MiddleMin(inner) => inner.get_value_count(),
            SortedDocValuesEnum1::MiddleMax(inner) => inner.get_value_count(),
        }
    }

    fn lookup_term(&mut self, key: &BytesRef) -> Result<i32> {
        match self {
            SortedDocValuesEnum1::Min(inner) => inner.lookup_term(key),
            SortedDocValuesEnum1::Max(inner) => inner.lookup_term(key),
            SortedDocValuesEnum1::MiddleMin(inner) => inner.lookup_term(key),
            SortedDocValuesEnum1::MiddleMax(inner) => inner.lookup_term(key),
        }
    }

    fn terms_enum(&mut self) -> Result<TermsEnums<I>> {
        match self {
            SortedDocValuesEnum1::Min(inner) => inner.terms_enum(),
            SortedDocValuesEnum1::Max(inner) => inner.terms_enum(),
            SortedDocValuesEnum1::MiddleMin(inner) => inner.terms_enum(),
            SortedDocValuesEnum1::MiddleMax(inner) => inner.terms_enum(),
        }
    }
}

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
use crate::index::doc_values::DocValues;
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::filter_numeric_doc_values::FilterNumericDocValues;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::search::sort_field::SortFieldType;
use crate::util::either_enums::Either3NumericDocValues;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::numeric_utils::NumericUtils;

#[derive(Clone)]
/// Selects a value from the document’s list to use as the representative value.
///
/// This provides a [`NumericDocValues`] view over the `SortedNumeric`, for use with sorting,
/// expressions, function queries, etc.
pub struct SortedNumericSelector;
impl SortedNumericSelector {
    /// Wraps a multi-valued `SortedNumericDocValues` as a single-valued view,
    /// using the specified `selector` and `numeric_type`.
    pub fn wrap<S>(
        mut sorted_numeric: S,
        selector: SortedNumericSelectorType,
        numeric_type: SortFieldType,
    ) -> Result<NumericDocValuesImpl<S>>
    where
        S: SortedNumericDocValues,
    {
        match numeric_type {
            SortFieldType::Int
            | SortFieldType::Long
            | SortFieldType::Float
            | SortFieldType::Double => {},
            _ => {
                return Err(LuceneError::illegal_argument(
                    "numericType must be a numeric type",
                ))
            },
        }

        let view = match DocValues::unwrap_singleton_numeric(&mut sorted_numeric)? { Some(single) => {
            Either3NumericDocValues::F(single)
        } _ => {
            match selector {
                SortedNumericSelectorType::Min => {
                    Either3NumericDocValues::S(MinValue::new(sorted_numeric))
                },
                SortedNumericSelectorType::Max => {
                    Either3NumericDocValues::T(MaxValue::new(sorted_numeric))
                },
            }
        }};

        match numeric_type {
            SortFieldType::Float => Ok(Either3NumericDocValues::F(
                FilterNumericDocValuesImpl1::new(view),
            )),
            SortFieldType::Double => Ok(Either3NumericDocValues::S(
                FilterNumericDocValuesImpl2::new(view),
            )),
            _ => Ok(Either3NumericDocValues::T(view)),
        }
    }
}
/// Type of selection to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortedNumericSelectorType {
    /// Selects the minimum value in the set.
    Min,
    /// Selects the maximum value in the set.
    Max,
    // TODO: We could implement Median in constant time (at most 2 lookups).
}

pub struct MinValue<S> {
    inner: S,
    value: i64,
}
impl<S> MinValue<S> {
    pub fn new(inner: S) -> Self {
        MinValue { inner, value: 0 }
    }
}

impl<S> DocValuesIterator for MinValue<S>
where
    S: SortedNumericDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.advance_exact(target)? {
            self.value = self.inner.next_value()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
/// Wraps a SortedNumericDocValues and returns the first value (min)
impl<S> DocIdSetIterator for MinValue<S>
where
    S: SortedNumericDocValues,
{
    fn doc_id(&self) -> i32 {
        self.inner.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let doc = self.inner.next_doc()?;
        if doc != NO_MORE_DOCS {
            self.value = self.inner.next_value()?;
        }
        Ok(doc)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        let doc = self.inner.advance(target)?;
        if doc != NO_MORE_DOCS {
            self.value = self.inner.next_value()?;
        }
        Ok(doc)
    }

    fn cost(&self) -> Result<i64> {
        self.inner.cost()
    }
}

impl<S> NumericDocValues for MinValue<S>
where
    S: SortedNumericDocValues,
{
    fn long_value(&mut self) -> Result<i64> {
        Ok(self.value)
    }
}
/// Wraps a SortedNumericDocValues and returns the last value (max)
pub struct MaxValue<S>
where
    S: SortedNumericDocValues,
{
    inner: S,
    value: i64,
}

impl<S> MaxValue<S>
where
    S: SortedNumericDocValues,
{
    pub fn new(inner: S) -> Self {
        MaxValue { inner, value: 0 }
    }

    fn set_value(&mut self) -> Result<()> {
        let count = self.inner.doc_value_count()?;
        for _ in 0..count {
            self.value = self.inner.next_value()?;
        }
        Ok(())
    }
}

impl<S> DocIdSetIterator for MaxValue<S>
where
    S: SortedNumericDocValues,
{
    fn doc_id(&self) -> i32 {
        self.inner.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let doc = self.inner.next_doc()?;
        if doc != NO_MORE_DOCS {
            self.set_value()?;
        }
        Ok(doc)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        let doc = self.inner.advance(target)?;
        if doc != NO_MORE_DOCS {
            self.set_value()?;
        }
        Ok(doc)
    }

    fn cost(&self) -> Result<i64> {
        self.inner.cost()
    }
}

impl<S> DocValuesIterator for MaxValue<S>
where
    S: SortedNumericDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.advance_exact(target)? {
            self.set_value()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl<S> NumericDocValues for MaxValue<S>
where
    S: SortedNumericDocValues,
{
    fn long_value(&mut self) -> Result<i64> {
        Ok(self.value)
    }
}

pub struct FilterNumericDocValuesImpl1<N>
where
    N: NumericDocValues,
{
    base: FilterNumericDocValues<N>,
}
impl<N> FilterNumericDocValuesImpl1<N>
where
    N: NumericDocValues,
{
    pub fn new(inner: N) -> Self {
        FilterNumericDocValuesImpl1 {
            base: FilterNumericDocValues::new(inner),
        }
    }
}

impl<N> DocValuesIterator for FilterNumericDocValuesImpl1<N>
where
    N: NumericDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.base.advance_exact(target)
    }
}

impl<N> DocIdSetIterator for FilterNumericDocValuesImpl1<N>
where
    N: NumericDocValues,
{
    fn doc_id(&self) -> i32 {
        self.base.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.base.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.base.advance(target)
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        self.base.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.base.cost()
    }
}

impl<N> NumericDocValues for FilterNumericDocValuesImpl1<N>
where
    N: NumericDocValues,
{
    fn long_value(&mut self) -> Result<i64> {
        let v = self.base.long_value()? as i32;
        Ok(NumericUtils::sortable_float_bits(v) as i64)
    }
}
pub struct FilterNumericDocValuesImpl2<N>
where
    N: NumericDocValues,
{
    base: FilterNumericDocValues<N>,
}
impl<N> FilterNumericDocValuesImpl2<N>
where
    N: NumericDocValues,
{
    pub fn new(inner: N) -> Self {
        FilterNumericDocValuesImpl2 {
            base: FilterNumericDocValues::new(inner),
        }
    }
}

impl<N> DocValuesIterator for FilterNumericDocValuesImpl2<N>
where
    N: NumericDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.base.advance_exact(target)
    }
}

impl<N> DocIdSetIterator for FilterNumericDocValuesImpl2<N>
where
    N: NumericDocValues,
{
    fn doc_id(&self) -> i32 {
        self.base.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.base.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.base.advance(target)
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        self.base.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.base.cost()
    }
}

impl<N> NumericDocValues for FilterNumericDocValuesImpl2<N>
where
    N: NumericDocValues,
{
    fn long_value(&mut self) -> Result<i64> {
        let v = self.base.long_value()? as i32;
        Ok(NumericUtils::sortable_float_bits(v) as i64)
    }
}

pub type NumericDocValuesImpl<S: SortedNumericDocValues> = Either3NumericDocValues<
    FilterNumericDocValuesImpl1<
        Either3NumericDocValues<S::NumericDocValues, MinValue<S>, MaxValue<S>>,
    >,
    FilterNumericDocValuesImpl2<
        Either3NumericDocValues<S::NumericDocValues, MinValue<S>, MaxValue<S>>,
    >,
    Either3NumericDocValues<S::NumericDocValues, MinValue<S>, MaxValue<S>>,
>;

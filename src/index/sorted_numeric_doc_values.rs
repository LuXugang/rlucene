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
use crate::index::numeric_doc_values::{EitherNumericDocValues, NumericDocValues};
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

// SortedNumericDocValues
pub enum EitherSortedNumericDocValues<F, S> {
    F(F),
    S(S),
}

impl<F, S> DocValuesIterator for EitherSortedNumericDocValues<F, S>
where
    F: SortedNumericDocValues,
    S: SortedNumericDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        match self {
            EitherSortedNumericDocValues::F(t) => t.advance_exact(target),
            EitherSortedNumericDocValues::S(s) => s.advance_exact(target),
        }
    }
}

impl<F, S> DocIdSetIterator for EitherSortedNumericDocValues<F, S>
where
    F: SortedNumericDocValues,
    S: SortedNumericDocValues,
{
    fn doc_id(&self) -> i32 {
        match self {
            EitherSortedNumericDocValues::F(t) => t.doc_id(),
            EitherSortedNumericDocValues::S(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            EitherSortedNumericDocValues::F(t) => t.next_doc(),
            EitherSortedNumericDocValues::S(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherSortedNumericDocValues::F(t) => t.advance(target),
            EitherSortedNumericDocValues::S(s) => s.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherSortedNumericDocValues::F(t) => t.slow_advance(target),
            EitherSortedNumericDocValues::S(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            EitherSortedNumericDocValues::F(t) => t.cost(),
            EitherSortedNumericDocValues::S(s) => s.cost(),
        }
    }
}

impl<F, S> SortedNumericDocValues for EitherSortedNumericDocValues<F, S>
where
    F: SortedNumericDocValues,
    S: SortedNumericDocValues,
{
    fn next_value(&mut self) -> Result<i64> {
        match self {
            EitherSortedNumericDocValues::F(t) => t.next_value(),
            EitherSortedNumericDocValues::S(s) => s.next_value(),
        }
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        match self {
            EitherSortedNumericDocValues::F(t) => t.doc_value_count(),
            EitherSortedNumericDocValues::S(s) => s.doc_value_count(),
        }
    }

    fn is_single_valued(&self) -> bool {
        match self {
            EitherSortedNumericDocValues::F(t) => t.is_single_valued(),
            EitherSortedNumericDocValues::S(s) => s.is_single_valued(),
        }
    }

    type NumericDocValues = EitherNumericDocValues<F::NumericDocValues, S::NumericDocValues>;

    fn get_numeric_doc_values(&mut self) -> Result<Option<Self::NumericDocValues>> {
        match self {
            EitherSortedNumericDocValues::F(t) => {
                let sorted_doc_values = t.get_numeric_doc_values()?;
                Ok(sorted_doc_values.map(EitherNumericDocValues::F))
            },
            EitherSortedNumericDocValues::S(s) => {
                let sorted_doc_values = s.get_numeric_doc_values()?;
                Ok(sorted_doc_values.map(EitherNumericDocValues::S))
            },
        }
    }
}

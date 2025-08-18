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
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::Result;
/// A per-document numeric value.
pub trait NumericDocValues: DocValuesIterator {
    /// Returns the numeric value for the current document ID.
    /// /// It is illegal to call this method after
    /// [`advanceExact`](DocValuesIterator::advance_exact) returned `false`.
    ///
    /// # Returns
    /// The numeric value for the current document ID.
    fn long_value(&mut self) -> Result<i64>;
}

// NumericDocValues
pub enum EitherNumericDocValues<F, S> {
    F(F),
    S(S),
}

impl<F, S> DocValuesIterator for EitherNumericDocValues<F, S>
where
    F: NumericDocValues,
    S: NumericDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        match self {
            EitherNumericDocValues::F(t) => t.advance_exact(target),
            EitherNumericDocValues::S(s) => s.advance_exact(target),
        }
    }
}

impl<F, S> DocIdSetIterator for EitherNumericDocValues<F, S>
where
    F: NumericDocValues,
    S: NumericDocValues,
{
    fn doc_id(&self) -> i32 {
        match self {
            EitherNumericDocValues::F(t) => t.doc_id(),
            EitherNumericDocValues::S(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            EitherNumericDocValues::F(t) => t.next_doc(),
            EitherNumericDocValues::S(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherNumericDocValues::F(t) => t.advance(target),
            EitherNumericDocValues::S(s) => s.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherNumericDocValues::F(t) => t.slow_advance(target),
            EitherNumericDocValues::S(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            EitherNumericDocValues::F(t) => t.cost(),
            EitherNumericDocValues::S(s) => s.cost(),
        }
    }
}

impl<F, S> NumericDocValues for EitherNumericDocValues<F, S>
where
    F: NumericDocValues,
    S: NumericDocValues,
{
    fn long_value(&mut self) -> Result<i64> {
        match self {
            EitherNumericDocValues::F(t) => t.long_value(),
            EitherNumericDocValues::S(s) => s.long_value(),
        }
    }
}

// Either 3
// NumericDocValues
pub enum Either3NumericDocValues<F, S, T> {
    F(F),
    S(S),
    T(T),
}

impl<F, S, T> DocValuesIterator for Either3NumericDocValues<F, S, T>
where
    F: NumericDocValues,
    S: NumericDocValues,
    T: NumericDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        match self {
            Either3NumericDocValues::F(t) => t.advance_exact(target),
            Either3NumericDocValues::S(s) => s.advance_exact(target),
            Either3NumericDocValues::T(t) => t.advance_exact(target),
        }
    }
}

impl<F, S, T> DocIdSetIterator for Either3NumericDocValues<F, S, T>
where
    F: NumericDocValues,
    S: NumericDocValues,
    T: NumericDocValues,
{
    fn doc_id(&self) -> i32 {
        match self {
            Either3NumericDocValues::F(t) => t.doc_id(),
            Either3NumericDocValues::S(s) => s.doc_id(),
            Either3NumericDocValues::T(t) => t.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            Either3NumericDocValues::F(t) => t.next_doc(),
            Either3NumericDocValues::S(s) => s.next_doc(),
            Either3NumericDocValues::T(t) => t.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            Either3NumericDocValues::F(t) => t.advance(target),
            Either3NumericDocValues::S(s) => s.advance(target),
            Either3NumericDocValues::T(t) => t.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            Either3NumericDocValues::F(t) => t.slow_advance(target),
            Either3NumericDocValues::S(s) => s.slow_advance(target),
            Either3NumericDocValues::T(t) => t.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            Either3NumericDocValues::F(t) => t.cost(),
            Either3NumericDocValues::S(s) => s.cost(),
            Either3NumericDocValues::T(t) => t.cost(),
        }
    }
}

impl<F, S, T> NumericDocValues for Either3NumericDocValues<F, S, T>
where
    F: NumericDocValues,
    S: NumericDocValues,
    T: NumericDocValues,
{
    fn long_value(&mut self) -> Result<i64> {
        match self {
            Either3NumericDocValues::F(t) => t.long_value(),
            Either3NumericDocValues::S(s) => s.long_value(),
            Either3NumericDocValues::T(t) => t.long_value(),
        }
    }
}

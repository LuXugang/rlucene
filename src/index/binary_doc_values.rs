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
use crate::index::BytesRef;
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::{LuceneError, Result};

pub trait BinaryDocValues: DocValuesIterator {
    /// Returns the binary value for the current document ID.
    /// It is illegal to call this method after
    /// [`advanceExact`](DocValuesIterator::advance_exact) returned `false`.
    ///
    /// # Returns
    /// The binary value for the current document ID.
    fn binary_value(&mut self) -> Result<&BytesRef<Vec<u8>>> {
        Err(LuceneError::not_implemented("this method need implement"))
    }
}

// BinaryDocValues
pub enum EitherBinaryDocValues<F, S> {
    F(F),
    S(S),
}

impl<F, S> DocValuesIterator for EitherBinaryDocValues<F, S>
where
    F: BinaryDocValues,
    S: BinaryDocValues,
{
}

impl<F, S> DocIdSetIterator for EitherBinaryDocValues<F, S>
where
    F: BinaryDocValues,
    S: BinaryDocValues,
{
    fn doc_id(&self) -> i32 {
        match self {
            EitherBinaryDocValues::F(t) => t.doc_id(),
            EitherBinaryDocValues::S(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            EitherBinaryDocValues::F(t) => t.next_doc(),
            EitherBinaryDocValues::S(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherBinaryDocValues::F(t) => t.advance(target),
            EitherBinaryDocValues::S(s) => s.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherBinaryDocValues::F(t) => t.slow_advance(target),
            EitherBinaryDocValues::S(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            EitherBinaryDocValues::F(t) => t.cost(),
            EitherBinaryDocValues::S(s) => s.cost(),
        }
    }
}

impl<F, S> BinaryDocValues for EitherBinaryDocValues<F, S>
where
    F: BinaryDocValues,
    S: BinaryDocValues,
{
    fn binary_value(&mut self) -> Result<&BytesRef<Vec<u8>>> {
        match self {
            EitherBinaryDocValues::F(t) => t.binary_value(),
            EitherBinaryDocValues::S(s) => s.binary_value(),
        }
    }
}

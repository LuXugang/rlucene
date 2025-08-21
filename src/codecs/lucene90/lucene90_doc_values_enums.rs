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
use crate::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::codecs::lucene90_doc_values_producer::{
    DenseBinaryDocValues, DenseNumericDocValues, DenseSortedNumericDocValues,
    SpareSortedNumericDocValues, SparseBinaryDocValues, SparseNumericDocValues,
};
use crate::index::binary_doc_values::EitherBinaryDocValues3;
use crate::index::doc_values::{EmptyBinary, EmptyNumeric};
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::numeric_doc_values::{Either3NumericDocValues, EitherNumericDocValues};
use crate::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
use crate::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::IndexInput;
use crate::util::error::lucene_error::Result;

// 1. NumericDocValues
pub type Lucene90NumericDocValuesEnum<I> =
    Either3NumericDocValues<DenseNumericDocValues<I>, SparseNumericDocValues<I>, EmptyNumeric>;
// 2.SortedNumericDocValues
pub enum Lucene90SortedNumericDocValuesEnums<I>
where
    I: IndexInput,
{
    Dense(DenseSortedNumericDocValues<I>),
    Sparse(SpareSortedNumericDocValues<I>),
    Singleton(SingletonSortedNumericDocValues<Lucene90NumericDocValuesEnum<I>>),
    Empty(SingletonSortedNumericDocValues<EmptyNumeric>),
}

impl<I> DocValuesIterator for Lucene90SortedNumericDocValuesEnums<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        match self {
            Lucene90SortedNumericDocValuesEnums::Dense(d) => d.advance_exact(target),
            Lucene90SortedNumericDocValuesEnums::Sparse(s) => s.advance_exact(target),
            Lucene90SortedNumericDocValuesEnums::Singleton(s) => s.advance_exact(target),
            Lucene90SortedNumericDocValuesEnums::Empty(s) => s.advance_exact(target),
        }
    }
}
impl<I> DocIdSetIterator for Lucene90SortedNumericDocValuesEnums<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        match self {
            Lucene90SortedNumericDocValuesEnums::Dense(d) => d.doc_id(),
            Lucene90SortedNumericDocValuesEnums::Sparse(s) => s.doc_id(),
            Lucene90SortedNumericDocValuesEnums::Singleton(s) => s.doc_id(),
            Lucene90SortedNumericDocValuesEnums::Empty(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            Lucene90SortedNumericDocValuesEnums::Dense(d) => d.next_doc(),
            Lucene90SortedNumericDocValuesEnums::Sparse(s) => s.next_doc(),
            Lucene90SortedNumericDocValuesEnums::Singleton(s) => s.next_doc(),
            Lucene90SortedNumericDocValuesEnums::Empty(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            Lucene90SortedNumericDocValuesEnums::Dense(d) => d.advance(target),
            Lucene90SortedNumericDocValuesEnums::Sparse(s) => s.advance(target),
            Lucene90SortedNumericDocValuesEnums::Singleton(s) => s.advance(target),
            Lucene90SortedNumericDocValuesEnums::Empty(s) => s.advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            Lucene90SortedNumericDocValuesEnums::Dense(d) => d.cost(),
            Lucene90SortedNumericDocValuesEnums::Sparse(s) => s.cost(),
            Lucene90SortedNumericDocValuesEnums::Singleton(s) => s.cost(),
            Lucene90SortedNumericDocValuesEnums::Empty(s) => s.cost(),
        }
    }
}

impl<I> SortedNumericDocValues for Lucene90SortedNumericDocValuesEnums<I>
where
    I: IndexInput,
{
    fn next_value(&mut self) -> Result<i64> {
        match self {
            Lucene90SortedNumericDocValuesEnums::Dense(d) => d.next_value(),
            Lucene90SortedNumericDocValuesEnums::Sparse(s) => s.next_value(),
            Lucene90SortedNumericDocValuesEnums::Singleton(s) => s.next_value(),
            Lucene90SortedNumericDocValuesEnums::Empty(s) => s.next_value(),
        }
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        match self {
            Lucene90SortedNumericDocValuesEnums::Dense(d) => d.doc_value_count(),
            Lucene90SortedNumericDocValuesEnums::Sparse(s) => s.doc_value_count(),
            Lucene90SortedNumericDocValuesEnums::Singleton(s) => s.doc_value_count(),
            Lucene90SortedNumericDocValuesEnums::Empty(s) => s.doc_value_count(),
        }
    }

    fn is_single_valued(&self) -> bool {
        match self {
            Lucene90SortedNumericDocValuesEnums::Dense(_) => false,
            Lucene90SortedNumericDocValuesEnums::Sparse(_) => false,
            Lucene90SortedNumericDocValuesEnums::Singleton(s) => s.is_single_valued(),
            // for padding
            Lucene90SortedNumericDocValuesEnums::Empty(_) => false,
        }
    }

    type NumericDocValues =
        EitherNumericDocValues<Lucene90NumericDocValuesEnum<I>, DummyNumericDocValues>;

    fn get_numeric_doc_values(&mut self) -> Result<Option<Self::NumericDocValues>> {
        match self {
            Lucene90SortedNumericDocValuesEnums::Singleton(s) => {
                let v = s.get_numeric_doc_values()?.unwrap();
                Ok(Some(EitherNumericDocValues::F(v)))
            },
            _ => Ok(None),
        }
    }
}

// 3. BinaryDocValues
pub type Lucene90BinaryDocValuesEnum<I> =
    EitherBinaryDocValues3<DenseBinaryDocValues<I>, SparseBinaryDocValues<I>, EmptyBinary>;

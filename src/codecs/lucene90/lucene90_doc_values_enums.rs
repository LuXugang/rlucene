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
use crate::index::binary_doc_values::BinaryDocValues;
use crate::index::doc_values::{EmptyBinary, EmptyNumeric};
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
use crate::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::IndexInput;
use crate::util::either_enums::EitherNumericDocValues;
use crate::util::error::lucene_error::Result;

// 1. NumericDocValues
pub enum Lucene90NumericDocValuesEnums<I>
where
    I: IndexInput,
{
    Dense(DenseNumericDocValues<I>),
    Sparse(SparseNumericDocValues<I>),
    Empty(EmptyNumeric),
}

impl<I> DocValuesIterator for Lucene90NumericDocValuesEnums<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        match self {
            Lucene90NumericDocValuesEnums::Dense(d) => d.advance_exact(_target),
            Lucene90NumericDocValuesEnums::Sparse(s) => s.advance_exact(_target),
            Lucene90NumericDocValuesEnums::Empty(e) => e.advance_exact(_target),
        }
    }
}

impl<I> DocIdSetIterator for Lucene90NumericDocValuesEnums<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        match self {
            Lucene90NumericDocValuesEnums::Dense(d) => d.doc_id(),
            Lucene90NumericDocValuesEnums::Sparse(s) => s.doc_id(),
            Lucene90NumericDocValuesEnums::Empty(e) => e.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            Lucene90NumericDocValuesEnums::Dense(d) => d.next_doc(),
            Lucene90NumericDocValuesEnums::Sparse(s) => s.next_doc(),
            Lucene90NumericDocValuesEnums::Empty(e) => e.next_doc(),
        }
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        match self {
            Lucene90NumericDocValuesEnums::Dense(d) => d.advance(_target),
            Lucene90NumericDocValuesEnums::Sparse(s) => s.advance(_target),
            Lucene90NumericDocValuesEnums::Empty(e) => e.advance(_target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            Lucene90NumericDocValuesEnums::Dense(d) => d.cost(),
            Lucene90NumericDocValuesEnums::Sparse(s) => s.cost(),
            Lucene90NumericDocValuesEnums::Empty(e) => e.cost(),
        }
    }
}

impl<I> NumericDocValues for Lucene90NumericDocValuesEnums<I>
where
    I: IndexInput,
{
    fn long_value(&mut self) -> Result<i64> {
        match self {
            Lucene90NumericDocValuesEnums::Dense(d) => d.long_value(),
            Lucene90NumericDocValuesEnums::Sparse(s) => s.long_value(),
            Lucene90NumericDocValuesEnums::Empty(e) => e.long_value(),
        }
    }
}
// 2.SortedNumericDocValues
pub enum Lucene90SortedNumericDocValuesEnums<I>
where
    I: IndexInput,
{
    Dense(DenseSortedNumericDocValues<I>),
    Sparse(SpareSortedNumericDocValues<I>),
    Singleton(SingletonSortedNumericDocValues<Lucene90NumericDocValuesEnums<I>>),
    Empty(SingletonSortedNumericDocValues<EmptyNumeric>),
}

impl<I> DocValuesIterator for Lucene90SortedNumericDocValuesEnums<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        match self {
            Lucene90SortedNumericDocValuesEnums::Dense(d) => d.advance_exact(_target),
            Lucene90SortedNumericDocValuesEnums::Sparse(s) => s.advance_exact(_target),
            Lucene90SortedNumericDocValuesEnums::Singleton(s) => s.advance_exact(_target),
            Lucene90SortedNumericDocValuesEnums::Empty(s) => s.advance_exact(_target),
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

    fn advance(&mut self, _target: i32) -> Result<i32> {
        match self {
            Lucene90SortedNumericDocValuesEnums::Dense(d) => d.advance(_target),
            Lucene90SortedNumericDocValuesEnums::Sparse(s) => s.advance(_target),
            Lucene90SortedNumericDocValuesEnums::Singleton(s) => s.advance(_target),
            Lucene90SortedNumericDocValuesEnums::Empty(s) => s.advance(_target),
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
        EitherNumericDocValues<Lucene90NumericDocValuesEnums<I>, DummyNumericDocValues>;

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

// 5.BinaryDocValues
pub enum Lucene90BinaryDocValuesEnum<I>
where
    I: IndexInput,
{
    Dense(DenseBinaryDocValues<I>),
    Sparse(SparseBinaryDocValues<I>),
    Empty(EmptyBinary),
}

impl<I> DocValuesIterator for Lucene90BinaryDocValuesEnum<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        match self {
            Lucene90BinaryDocValuesEnum::Dense(d) => d.advance_exact(_target),
            Lucene90BinaryDocValuesEnum::Sparse(s) => s.advance_exact(_target),
            Lucene90BinaryDocValuesEnum::Empty(e) => e.advance_exact(_target),
        }
    }
}

impl<I> DocIdSetIterator for Lucene90BinaryDocValuesEnum<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        match self {
            Lucene90BinaryDocValuesEnum::Dense(d) => d.doc_id(),
            Lucene90BinaryDocValuesEnum::Sparse(s) => s.doc_id(),
            Lucene90BinaryDocValuesEnum::Empty(e) => e.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            Lucene90BinaryDocValuesEnum::Dense(d) => d.next_doc(),
            Lucene90BinaryDocValuesEnum::Sparse(s) => s.next_doc(),
            Lucene90BinaryDocValuesEnum::Empty(e) => e.next_doc(),
        }
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        match self {
            Lucene90BinaryDocValuesEnum::Dense(d) => d.advance(_target),
            Lucene90BinaryDocValuesEnum::Sparse(s) => s.advance(_target),
            Lucene90BinaryDocValuesEnum::Empty(e) => e.advance(_target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            Lucene90BinaryDocValuesEnum::Dense(d) => d.cost(),
            Lucene90BinaryDocValuesEnum::Sparse(s) => s.cost(),
            Lucene90BinaryDocValuesEnum::Empty(e) => e.cost(),
        }
    }
}

impl<I> BinaryDocValues for Lucene90BinaryDocValuesEnum<I>
where
    I: IndexInput,
{
    fn binary_value(&mut self) -> Result<&BytesRef<Vec<u8>>> {
        match self {
            Lucene90BinaryDocValuesEnum::Dense(d) => d.binary_value(),
            Lucene90BinaryDocValuesEnum::Sparse(s) => s.binary_value(),
            Lucene90BinaryDocValuesEnum::Empty(e) => e.binary_value(),
        }
    }
}

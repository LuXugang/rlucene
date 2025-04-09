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
pub mod doc_values {
    use crate::codecs::doc_values_consumer::{
        BinaryDocValuesMerge, NumericDocValuesMerge, SortedNumericDocValuesMerge,
    };
    use crate::codecs::lucene90_doc_values_consumer::{
        NumericDocValuesImpl, SortedNumericDocValuesImpl,
    };
    use crate::codecs::lucene90_doc_values_producer::{
        BaseSortedDocValues, BaseSortedSetDocValues, DenseBinaryDocValues, DenseNumericDocValues,
        DenseSortedNumericDocValues, DocValuesSkipperImpl, SpareSortedNumericDocValues,
        SparseBinaryDocValues, SparseNumericDocValues,
    };
    use crate::index::binary_doc_values::BinaryDocValues;
    use crate::index::doc_values::{EmptyBinary, EmptyNumeric, EmptySorted};
    use crate::index::doc_values_iterator::DocValuesIterator;
    use crate::index::numeric_doc_values::NumericDocValues;
    use crate::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
    use crate::index::singleton_sorted_set_doc_values::SingletonSortedSetDocValues;
    use crate::index::sorted_doc_values::SortedDocValues;
    use crate::index::sorted_numeric_doc_values::SortedNumericDocValues;
    use crate::index::sorted_set_doc_values::SortedSetDocValues;
    use crate::index::terms_enums::TermsEnums;
    use crate::index::BytesRef;
    use crate::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::search::sorted_set_selector::SortedDocValuesEnum1;
    use crate::store::IndexInput;
    use crate::util::error::lucene_error::Result;
    use std::cell::RefCell;
    use std::rc::Rc;

    pub enum DocValuesSkipperEnum<I>
    where
        I: IndexInput,
    {
        Impl(DocValuesSkipperImpl<I>),
    }

    pub enum SortedDocValuesEnum<I>
    where
        I: IndexInput,
    {
        Base(Box<BaseSortedDocValues<I>>),
        Impl(SortedDocValuesEnum1<I>),
        Empty(EmptySorted<I>),
    }

    impl<I> DocValuesIterator for SortedDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn advance_exact(&mut self, _target: i32) -> Result<bool> {
            todo!()
        }
    }

    impl<I> DocIdSetIterator for SortedDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn doc_id(&self) -> i32 {
            todo!()
        }

        fn next_doc(&mut self) -> Result<i32> {
            todo!()
        }

        fn advance(&mut self, _target: i32) -> Result<i32> {
            todo!()
        }

        fn slow_advance(&mut self, target: i32) -> Result<i32> {
            todo!()
        }

        fn cost(&self) -> Result<i64> {
            todo!()
        }
    }

    impl<I> SortedDocValues<I> for SortedDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn ord_value(&mut self) -> Result<i32> {
            todo!()
        }

        fn lookup_ord(&mut self, ord: i32) -> Result<BytesRef> {
            todo!()
        }

        fn get_value_count(&self) -> Result<i32> {
            todo!()
        }

        fn lookup_term(&mut self, key: &BytesRef) -> Result<i32> {
            todo!()
        }

        fn terms_enum(&mut self) -> Result<TermsEnums<I>> {
            todo!()
        }
    }
    pub enum BinaryDocValuesEnum<I>
    where
        I: IndexInput,
    {
        Dense(DenseBinaryDocValues<I>),
        Sparse(SparseBinaryDocValues<I>),
        Empty(EmptyBinary),
        Merge(BinaryDocValuesMerge<I>),
    }

    impl<I> DocValuesIterator for BinaryDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn advance_exact(&mut self, target: i32) -> Result<bool> {
            match self {
                BinaryDocValuesEnum::Dense(dense) => dense.advance_exact(target),
                BinaryDocValuesEnum::Sparse(sparse) => sparse.advance_exact(target),
                BinaryDocValuesEnum::Empty(empty) => empty.advance_exact(target),
                BinaryDocValuesEnum::Merge(merge) => merge.advance_exact(target),
            }
        }
    }

    impl<I> DocIdSetIterator for BinaryDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn doc_id(&self) -> i32 {
            match self {
                BinaryDocValuesEnum::Dense(dense) => dense.doc_id(),
                BinaryDocValuesEnum::Sparse(sparse) => sparse.doc_id(),
                BinaryDocValuesEnum::Empty(empty) => empty.doc_id(),
                BinaryDocValuesEnum::Merge(merge) => merge.doc_id(),
            }
        }

        fn next_doc(&mut self) -> Result<i32> {
            match self {
                BinaryDocValuesEnum::Dense(dense) => dense.next_doc(),
                BinaryDocValuesEnum::Sparse(sparse) => sparse.next_doc(),
                BinaryDocValuesEnum::Empty(empty) => empty.next_doc(),
                BinaryDocValuesEnum::Merge(merge) => merge.next_doc(),
            }
        }

        fn advance(&mut self, target: i32) -> Result<i32> {
            match self {
                BinaryDocValuesEnum::Dense(dense) => dense.advance(target),
                BinaryDocValuesEnum::Sparse(sparse) => sparse.advance(target),
                BinaryDocValuesEnum::Empty(empty) => empty.advance(target),
                BinaryDocValuesEnum::Merge(merge) => merge.advance(target),
            }
        }

        fn slow_advance(&mut self, target: i32) -> Result<i32> {
            match self {
                BinaryDocValuesEnum::Dense(dense) => dense.slow_advance(target),
                BinaryDocValuesEnum::Sparse(sparse) => sparse.slow_advance(target),
                BinaryDocValuesEnum::Empty(empty) => empty.slow_advance(target),
                BinaryDocValuesEnum::Merge(merge) => merge.slow_advance(target),
            }
        }

        fn cost(&self) -> Result<i64> {
            match self {
                BinaryDocValuesEnum::Dense(dense) => dense.cost(),
                BinaryDocValuesEnum::Sparse(sparse) => sparse.cost(),
                BinaryDocValuesEnum::Empty(empty) => empty.cost(),
                BinaryDocValuesEnum::Merge(merge) => merge.cost(),
            }
        }
    }

    impl<I> BinaryDocValues for BinaryDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn binary_value(&mut self) -> Result<&BytesRef> {
            match self {
                BinaryDocValuesEnum::Dense(dense) => dense.binary_value(),
                BinaryDocValuesEnum::Sparse(sparse) => sparse.binary_value(),
                BinaryDocValuesEnum::Empty(empty) => empty.binary_value(),
                BinaryDocValuesEnum::Merge(merge) => merge.binary_value(),
            }
        }
    }

    pub enum NumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        Dense(DenseNumericDocValues<I>),
        Sparse(SparseNumericDocValues<I>),
        Empty(EmptyNumeric),
        Impl(NumericDocValuesImpl<I>),
        Merge(NumericDocValuesMerge<I>),
    }

    impl<I> DocValuesIterator for NumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn advance_exact(&mut self, target: i32) -> Result<bool> {
            todo!()
        }
    }

    impl<I> DocIdSetIterator for NumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn doc_id(&self) -> i32 {
            todo!()
        }

        fn next_doc(&mut self) -> Result<i32> {
            todo!()
        }

        fn advance(&mut self, target: i32) -> Result<i32> {
            todo!()
        }

        fn slow_advance(&mut self, target: i32) -> Result<i32> {
            todo!()
        }

        fn cost(&self) -> Result<i64> {
            todo!()
        }
    }

    impl<I> NumericDocValues for NumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn long_value(&mut self) -> Result<i64> {
            todo!()
        }
    }
    pub enum SortedNumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        Dense(DenseSortedNumericDocValues<I>),
        Sparse(SpareSortedNumericDocValues<I>),
        Singleton(SingletonSortedNumericDocValues<I>),
        Impl(SortedNumericDocValuesImpl<I>),
        Merge(SortedNumericDocValuesMerge<I>),
    }
    impl<I> DocValuesIterator for SortedNumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn advance_exact(&mut self, _target: i32) -> Result<bool> {
            todo!()
        }
    }

    impl<I> DocIdSetIterator for SortedNumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn doc_id(&self) -> i32 {
            todo!()
        }

        fn next_doc(&mut self) -> Result<i32> {
            todo!()
        }

        fn advance(&mut self, _target: i32) -> Result<i32> {
            todo!()
        }

        fn slow_advance(&mut self, target: i32) -> Result<i32> {
            todo!()
        }

        fn cost(&self) -> Result<i64> {
            todo!()
        }
    }

    impl<I> SortedNumericDocValues<I> for SortedNumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn next_value(&mut self) -> Result<i64> {
            todo!()
        }

        fn doc_value_count(&mut self) -> Result<i32> {
            todo!()
        }

        fn unwrap_singleton(&self) -> Result<Option<Rc<RefCell<NumericDocValuesEnum<I>>>>> {
            match self {
                SortedNumericDocValuesEnum::Singleton(singleton) => singleton.unwrap_singleton(),
                SortedNumericDocValuesEnum::Dense(dense) => dense.unwrap_singleton(),
                SortedNumericDocValuesEnum::Sparse(sparse) => sparse.unwrap_singleton(),
                SortedNumericDocValuesEnum::Impl(impl_) => impl_.unwrap_singleton(),
                SortedNumericDocValuesEnum::Merge(merge) => merge.unwrap_singleton(),
            }
        }
    }

    pub enum SortedSetDocValuesEnum<I>
    where
        I: IndexInput,
    {
        // SingletonSortedSetDocValues wraps a SortedDocValuesEnum.
        // To prevent mutual inclusion between variants of SortedSetDocValuesEnum and SortedDocValuesEnum,
        // the other variants are encapsulated using SortedSetDocValuesWrapper.
        Singleton(SingletonSortedSetDocValues<I>),
        Other(Box<SortedSetDocValuesWrapper<I>>),
    }

    impl<I> DocValuesIterator for SortedSetDocValuesEnum<I> where I: IndexInput {}

    impl<I> DocIdSetIterator for SortedSetDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn doc_id(&self) -> i32 {
            todo!()
        }

        fn next_doc(&mut self) -> Result<i32> {
            todo!()
        }
    }

    impl<I> SortedSetDocValues<I> for SortedSetDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn next_ord(&mut self) -> Result<i64> {
            todo!()
        }

        fn doc_value_count(&mut self) -> Result<i32> {
            todo!()
        }

        fn lookup_ord(&mut self, _ord: i64) -> Result<BytesRef> {
            todo!()
        }

        fn get_value_count(&self) -> Result<i64> {
            todo!()
        }

        fn lookup_term(&mut self, key: &BytesRef) -> Result<i64> {
            todo!()
        }

        fn terms_enum(&mut self) -> Result<TermsEnums<I>> {
            todo!()
        }

        fn unwrap_singleton(&self) -> Result<Option<Rc<RefCell<SortedDocValuesEnum<I>>>>> {
            match self {
                SortedSetDocValuesEnum::Singleton(singleton) => singleton.unwrap_singleton(),
                SortedSetDocValuesEnum::Other(other) => other.unwrap_singleton(),
            }
        }
    }

    pub enum SortedSetDocValuesWrapper<I>
    where
        I: IndexInput,
    {
        Base(BaseSortedSetDocValues<I>),
    }

    impl<I> DocValuesIterator for SortedSetDocValuesWrapper<I>
    where
        I: IndexInput,
    {
        fn advance_exact(&mut self, _target: i32) -> Result<bool> {
            todo!()
        }
    }

    impl<I> DocIdSetIterator for SortedSetDocValuesWrapper<I>
    where
        I: IndexInput,
    {
        fn doc_id(&self) -> i32 {
            todo!()
        }

        fn next_doc(&mut self) -> Result<i32> {
            todo!()
        }

        fn advance(&mut self, _target: i32) -> Result<i32> {
            todo!()
        }

        fn slow_advance(&mut self, target: i32) -> Result<i32> {
            todo!()
        }

        fn cost(&self) -> Result<i64> {
            todo!()
        }
    }

    impl<I> SortedSetDocValues<I> for SortedSetDocValuesWrapper<I>
    where
        I: IndexInput,
    {
        fn next_ord(&mut self) -> Result<i64> {
            todo!()
        }

        fn doc_value_count(&mut self) -> Result<i32> {
            todo!()
        }

        fn lookup_ord(&mut self, _ord: i64) -> Result<BytesRef> {
            todo!()
        }

        fn get_value_count(&self) -> Result<i64> {
            todo!()
        }

        fn lookup_term(&mut self, key: &BytesRef) -> Result<i64> {
            todo!()
        }

        fn terms_enum(&mut self) -> Result<TermsEnums<I>> {
            todo!()
        }

        fn unwrap_singleton(&self) -> Result<Option<Rc<RefCell<SortedDocValuesEnum<I>>>>> {
            todo!()
        }
    }
}

pub mod norms {
    use crate::codecs::lucene90_norms_producer::{DenseNormsIterator, SparseNormsIterator};
    use crate::codecs::norms_consumer::NumericDocValuesMerge;
    use crate::index::doc_values::EmptyNumeric;
    use crate::index::doc_values_iterator::DocValuesIterator;
    use crate::index::numeric_doc_values::NumericDocValues;
    use crate::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::store::IndexInput;
    use crate::util::error::lucene_error::Result;
    pub enum NumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        Dense(DenseNormsIterator<I>),
        Sparse(SparseNormsIterator<I>),
        Empty(EmptyNumeric),
        Merge(NumericDocValuesMerge<I>),
    }

    impl<I> DocValuesIterator for NumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn advance_exact(&mut self, target: i32) -> Result<bool> {
            match self {
                NumericDocValuesEnum::Dense(dense) => dense.advance_exact(target),
                NumericDocValuesEnum::Sparse(sparse) => sparse.advance_exact(target),
                NumericDocValuesEnum::Empty(empty) => empty.advance_exact(target),
                NumericDocValuesEnum::Merge(merge) => merge.advance_exact(target),
            }
        }
    }

    impl<I> DocIdSetIterator for NumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn doc_id(&self) -> i32 {
            match self {
                NumericDocValuesEnum::Dense(dense) => dense.doc_id(),
                NumericDocValuesEnum::Sparse(sparse) => sparse.doc_id(),
                NumericDocValuesEnum::Empty(empty) => empty.doc_id(),
                NumericDocValuesEnum::Merge(merge) => merge.doc_id(),
            }
        }

        fn next_doc(&mut self) -> Result<i32> {
            match self {
                NumericDocValuesEnum::Dense(dense) => dense.next_doc(),
                NumericDocValuesEnum::Sparse(sparse) => sparse.next_doc(),
                NumericDocValuesEnum::Empty(empty) => empty.next_doc(),
                NumericDocValuesEnum::Merge(merge) => merge.next_doc(),
            }
        }

        fn advance(&mut self, target: i32) -> Result<i32> {
            match self {
                NumericDocValuesEnum::Dense(dense) => dense.advance(target),
                NumericDocValuesEnum::Sparse(sparse) => sparse.advance(target),
                NumericDocValuesEnum::Empty(empty) => empty.advance(target),
                NumericDocValuesEnum::Merge(merge) => merge.advance(target),
            }
        }

        fn slow_advance(&mut self, target: i32) -> Result<i32> {
            match self {
                NumericDocValuesEnum::Dense(dense) => dense.slow_advance(target),
                NumericDocValuesEnum::Sparse(sparse) => sparse.slow_advance(target),
                NumericDocValuesEnum::Empty(empty) => empty.slow_advance(target),
                NumericDocValuesEnum::Merge(merge) => merge.slow_advance(target),
            }
        }

        fn cost(&self) -> Result<i64> {
            match self {
                NumericDocValuesEnum::Dense(dense) => dense.cost(),
                NumericDocValuesEnum::Sparse(sparse) => sparse.cost(),
                NumericDocValuesEnum::Empty(empty) => empty.cost(),
                NumericDocValuesEnum::Merge(merge) => merge.cost(),
            }
        }
    }

    impl<I> NumericDocValues for NumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn long_value(&mut self) -> Result<i64> {
            match self {
                NumericDocValuesEnum::Dense(dense) => dense.long_value(),
                NumericDocValuesEnum::Sparse(sparse) => sparse.long_value(),
                NumericDocValuesEnum::Empty(empty) => empty.long_value(),
                NumericDocValuesEnum::Merge(merge) => merge.long_value(),
            }
        }
    }
}

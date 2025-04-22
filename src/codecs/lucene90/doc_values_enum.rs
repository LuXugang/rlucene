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
    use crate::util::access::AccessVec;
    use crate::util::error::lucene_error::Result;
    use std::borrow::Cow;
    use std::cell::RefCell;
    use std::rc::Rc;

    pub enum DocValuesSkipperEnum<I>
    where
        I: IndexInput,
    {
        Impl(DocValuesSkipperImpl<I>),
    }

    pub enum SortedDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        Base(Box<BaseSortedDocValues<I, AV>>),
        Impl(SortedDocValuesEnum1<I, AV>),
        Empty(EmptySorted<I, AV>),
    }

    impl<I, AV> DocValuesIterator for SortedDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        fn advance_exact(&mut self, _target: i32) -> Result<bool> {
            todo!()
        }
    }

    impl<I, AV> DocIdSetIterator for SortedDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
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

        fn slow_advance(&mut self, _target: i32) -> Result<i32> {
            todo!()
        }

        fn cost(&self) -> Result<i64> {
            todo!()
        }
    }

    impl<I, AV> SortedDocValues<I, AV> for SortedDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        fn ord_value(&mut self) -> Result<i32> {
            todo!()
        }

        fn lookup_ord(&mut self, _ord: i32) -> Result<Cow<BytesRef<AV>>> {
            todo!()
        }

        fn get_value_count(&self) -> Result<i32> {
            todo!()
        }

        fn lookup_term(&mut self, key: &BytesRef<AV>) -> Result<i32> {
            todo!()
        }

        fn terms_enum(&mut self) -> Result<TermsEnums<I, AV>> {
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
        fn binary_value(&mut self) -> Result<&BytesRef<Vec<u8>>> {
            match self {
                BinaryDocValuesEnum::Dense(dense) => dense.binary_value(),
                BinaryDocValuesEnum::Sparse(sparse) => sparse.binary_value(),
                BinaryDocValuesEnum::Empty(empty) => empty.binary_value(),
                BinaryDocValuesEnum::Merge(merge) => merge.binary_value(),
            }
        }
    }

    pub enum NumericDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        Dense(DenseNumericDocValues<I>),
        Sparse(SparseNumericDocValues<I>),
        Empty(EmptyNumeric),
        Impl(NumericDocValuesImpl<I, AV>),
        Merge(NumericDocValuesMerge<I, AV>),
    }

    impl<I, AV> DocValuesIterator for NumericDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        fn advance_exact(&mut self, _target: i32) -> Result<bool> {
            todo!()
        }
    }

    impl<I, AV> DocIdSetIterator for NumericDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
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

        fn slow_advance(&mut self, _target: i32) -> Result<i32> {
            todo!()
        }

        fn cost(&self) -> Result<i64> {
            todo!()
        }
    }

    impl<I, AV> NumericDocValues for NumericDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        fn long_value(&mut self) -> Result<i64> {
            todo!()
        }
    }
    pub enum SortedNumericDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        Dense(DenseSortedNumericDocValues<I>),
        Sparse(SpareSortedNumericDocValues<I>),
        Singleton(SingletonSortedNumericDocValues<I, AV>),
        Impl(SortedNumericDocValuesImpl<I, AV>),
        Merge(SortedNumericDocValuesMerge<I, AV>),
    }
    impl<I, AV> DocValuesIterator for SortedNumericDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        fn advance_exact(&mut self, _target: i32) -> Result<bool> {
            todo!()
        }
    }

    impl<I, AV> DocIdSetIterator for SortedNumericDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
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

        fn slow_advance(&mut self, _target: i32) -> Result<i32> {
            todo!()
        }

        fn cost(&self) -> Result<i64> {
            todo!()
        }
    }

    impl<I, AV> SortedNumericDocValues for SortedNumericDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        fn next_value(&mut self) -> Result<i64> {
            todo!()
        }

        fn doc_value_count(&mut self) -> Result<i32> {
            todo!()
        }
    }

    pub enum SortedSetDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        // SingletonSortedSetDocValues wraps a SortedDocValuesEnum.
        // To prevent mutual inclusion between variants of SortedSetDocValuesEnum and SortedDocValuesEnum,
        // the other variants are encapsulated using SortedSetDocValuesWrapper.
        Singleton(SingletonSortedSetDocValues<I, AV>),
        Other(Box<SortedSetDocValuesWrapper<I, AV>>),
    }

    impl<I, AV> DocValuesIterator for SortedSetDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
    }

    impl<I, AV> DocIdSetIterator for SortedSetDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        fn doc_id(&self) -> i32 {
            todo!()
        }

        fn next_doc(&mut self) -> Result<i32> {
            todo!()
        }
    }

    impl<I, AV> SortedSetDocValues<I, AV> for SortedSetDocValuesEnum<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        fn next_ord(&mut self) -> Result<i64> {
            todo!()
        }

        fn doc_value_count(&mut self) -> Result<i32> {
            todo!()
        }

        fn lookup_ord(&mut self, _ord: i64) -> Result<Cow<BytesRef<AV>>> {
            todo!()
        }

        fn get_value_count(&self) -> Result<i64> {
            todo!()
        }

        fn lookup_term(&mut self, key: &BytesRef<AV>) -> Result<i64> {
            todo!()
        }

        fn terms_enum(&mut self) -> Result<TermsEnums<I, AV>> {
            todo!()
        }

        fn unwrap_singleton(&self) -> Result<Option<Rc<RefCell<SortedDocValuesEnum<I, AV>>>>> {
            match self {
                SortedSetDocValuesEnum::Singleton(singleton) => singleton.unwrap_singleton(),
                SortedSetDocValuesEnum::Other(other) => other.unwrap_singleton(),
            }
        }
    }

    pub enum SortedSetDocValuesWrapper<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        Base(BaseSortedSetDocValues<I, AV>),
    }

    impl<I, AV> DocValuesIterator for SortedSetDocValuesWrapper<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        fn advance_exact(&mut self, _target: i32) -> Result<bool> {
            todo!()
        }
    }

    impl<I, AV> DocIdSetIterator for SortedSetDocValuesWrapper<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
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

        fn slow_advance(&mut self, _target: i32) -> Result<i32> {
            todo!()
        }

        fn cost(&self) -> Result<i64> {
            todo!()
        }
    }

    impl<I, AV> SortedSetDocValues<I, AV> for SortedSetDocValuesWrapper<I, AV>
    where
        I: IndexInput,
        AV: AccessVec<u8>,
    {
        fn next_ord(&mut self) -> Result<i64> {
            todo!()
        }

        fn doc_value_count(&mut self) -> Result<i32> {
            todo!()
        }

        fn lookup_ord(&mut self, _ord: i64) -> Result<Cow<BytesRef<AV>>> {
            todo!()
        }

        fn get_value_count(&self) -> Result<i64> {
            todo!()
        }

        fn lookup_term(&mut self, key: &BytesRef<AV>) -> Result<i64> {
            todo!()
        }

        fn terms_enum(&mut self) -> Result<TermsEnums<I, AV>> {
            todo!()
        }

        fn unwrap_singleton(&self) -> Result<Option<Rc<RefCell<SortedDocValuesEnum<I, AV>>>>> {
            todo!()
        }
    }
}

pub mod norms {
    use crate::codecs::lucene90_norms_producer::{DenseNormsIterator, SparseNormsIterator};
    use crate::index::doc_values::EmptyNumeric;
    use crate::index::doc_values_iterator::DocValuesIterator;
    use crate::index::numeric_doc_values::NumericDocValues;
    use crate::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::store::IndexInput;
    use crate::util::error::lucene_error::Result;
    pub enum Lucene90NormNumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        Dense(DenseNormsIterator<I>),
        Sparse(SparseNormsIterator<I>),
        Empty(EmptyNumeric),
    }

    impl<I> DocValuesIterator for Lucene90NormNumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn advance_exact(&mut self, target: i32) -> Result<bool> {
            match self {
                Lucene90NormNumericDocValuesEnum::Dense(dense) => dense.advance_exact(target),
                Lucene90NormNumericDocValuesEnum::Sparse(sparse) => sparse.advance_exact(target),
                Lucene90NormNumericDocValuesEnum::Empty(empty) => empty.advance_exact(target),
            }
        }
    }

    impl<I> DocIdSetIterator for Lucene90NormNumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn doc_id(&self) -> i32 {
            match self {
                Lucene90NormNumericDocValuesEnum::Dense(dense) => dense.doc_id(),
                Lucene90NormNumericDocValuesEnum::Sparse(sparse) => sparse.doc_id(),
                Lucene90NormNumericDocValuesEnum::Empty(empty) => empty.doc_id(),
            }
        }

        fn next_doc(&mut self) -> Result<i32> {
            match self {
                Lucene90NormNumericDocValuesEnum::Dense(dense) => dense.next_doc(),
                Lucene90NormNumericDocValuesEnum::Sparse(sparse) => sparse.next_doc(),
                Lucene90NormNumericDocValuesEnum::Empty(empty) => empty.next_doc(),
            }
        }

        fn advance(&mut self, target: i32) -> Result<i32> {
            match self {
                Lucene90NormNumericDocValuesEnum::Dense(dense) => dense.advance(target),
                Lucene90NormNumericDocValuesEnum::Sparse(sparse) => sparse.advance(target),
                Lucene90NormNumericDocValuesEnum::Empty(empty) => empty.advance(target),
            }
        }

        fn slow_advance(&mut self, target: i32) -> Result<i32> {
            match self {
                Lucene90NormNumericDocValuesEnum::Dense(dense) => dense.slow_advance(target),
                Lucene90NormNumericDocValuesEnum::Sparse(sparse) => sparse.slow_advance(target),
                Lucene90NormNumericDocValuesEnum::Empty(empty) => empty.slow_advance(target),
            }
        }

        fn cost(&self) -> Result<i64> {
            match self {
                Lucene90NormNumericDocValuesEnum::Dense(dense) => dense.cost(),
                Lucene90NormNumericDocValuesEnum::Sparse(sparse) => sparse.cost(),
                Lucene90NormNumericDocValuesEnum::Empty(empty) => empty.cost(),
            }
        }
    }

    impl<I> NumericDocValues for Lucene90NormNumericDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn long_value(&mut self) -> Result<i64> {
            match self {
                Lucene90NormNumericDocValuesEnum::Dense(dense) => dense.long_value(),
                Lucene90NormNumericDocValuesEnum::Sparse(sparse) => sparse.long_value(),
                Lucene90NormNumericDocValuesEnum::Empty(empty) => empty.long_value(),
            }
        }
    }
}

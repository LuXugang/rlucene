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
    use crate::codecs::lucene90_doc_values_producer::{
        DenseBinaryDocValues, DenseNumericDocValues, SparseBinaryDocValues, SparseNumericDocValues,
    };
    use crate::index::binary_doc_values::BinaryDocValues;
    use crate::index::doc_values::{EmptyBinary, EmptyNumeric};
    use crate::index::doc_values_iterator::DocValuesIterator;
    use crate::index::numeric_doc_values::NumericDocValues;
    use crate::index::BytesRef;
    use crate::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::store::IndexInput;
    use crate::util::error::lucene_error::Result;
    use std::cell::RefCell;
    use std::rc::Rc;

    pub enum BinaryDocValuesEnum<I>
    where
        I: IndexInput,
    {
        Dense(DenseBinaryDocValues<I>),
        Sparse(SparseBinaryDocValues<I>),
        Empty(EmptyBinary),
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
            }
        }

        fn next_doc(&mut self) -> Result<i32> {
            match self {
                BinaryDocValuesEnum::Dense(dense) => dense.next_doc(),
                BinaryDocValuesEnum::Sparse(sparse) => sparse.next_doc(),
                BinaryDocValuesEnum::Empty(empty) => empty.next_doc(),
            }
        }

        fn advance(&mut self, target: i32) -> Result<i32> {
            match self {
                BinaryDocValuesEnum::Dense(dense) => dense.advance(target),
                BinaryDocValuesEnum::Sparse(sparse) => sparse.advance(target),
                BinaryDocValuesEnum::Empty(empty) => empty.advance(target),
            }
        }

        fn slow_advance(&mut self, target: i32) -> Result<i32> {
            match self {
                BinaryDocValuesEnum::Dense(dense) => dense.slow_advance(target),
                BinaryDocValuesEnum::Sparse(sparse) => sparse.slow_advance(target),
                BinaryDocValuesEnum::Empty(empty) => empty.slow_advance(target),
            }
        }

        fn cost(&self) -> Result<i64> {
            match self {
                BinaryDocValuesEnum::Dense(dense) => dense.cost(),
                BinaryDocValuesEnum::Sparse(sparse) => sparse.cost(),
                BinaryDocValuesEnum::Empty(empty) => empty.cost(),
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
            }
        }

        fn binary_value_mut(&mut self) -> Result<Rc<RefCell<BytesRef>>> {
            match self {
                BinaryDocValuesEnum::Dense(dense) => dense.binary_value_mut(),
                BinaryDocValuesEnum::Sparse(sparse) => sparse.binary_value_mut(),
                BinaryDocValuesEnum::Empty(empty) => empty.binary_value_mut(),
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
            }
        }

        fn next_doc(&mut self) -> Result<i32> {
            match self {
                NumericDocValuesEnum::Dense(dense) => dense.next_doc(),
                NumericDocValuesEnum::Sparse(sparse) => sparse.next_doc(),
                NumericDocValuesEnum::Empty(empty) => empty.next_doc(),
            }
        }

        fn advance(&mut self, target: i32) -> Result<i32> {
            match self {
                NumericDocValuesEnum::Dense(dense) => dense.advance(target),
                NumericDocValuesEnum::Sparse(sparse) => sparse.advance(target),
                NumericDocValuesEnum::Empty(empty) => empty.advance(target),
            }
        }

        fn slow_advance(&mut self, target: i32) -> Result<i32> {
            match self {
                NumericDocValuesEnum::Dense(dense) => dense.slow_advance(target),
                NumericDocValuesEnum::Sparse(sparse) => sparse.slow_advance(target),
                NumericDocValuesEnum::Empty(empty) => empty.slow_advance(target),
            }
        }

        fn cost(&self) -> Result<i64> {
            match self {
                NumericDocValuesEnum::Dense(dense) => dense.cost(),
                NumericDocValuesEnum::Sparse(sparse) => sparse.cost(),
                NumericDocValuesEnum::Empty(empty) => empty.cost(),
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
            }
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

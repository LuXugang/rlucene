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

    use std::borrow::Cow;

    use crate::codecs::lucene90_doc_values_enums::Lucene90SortedSetDocValuesEnum;
    use crate::index::doc_values_iterator::DocValuesIterator;
    use crate::index::dummy::dummy_terms_enum::DummyTermsEnum;
    use crate::index::sorted_set_doc_values::SortedSetDocValues;
    use crate::index::BytesRef;
    use crate::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::store::IndexInput;
    use crate::util::error::lucene_error::Result;

    pub enum SortedSetDocValuesEnum<I>
    where
        I: IndexInput,
    {
        // SingletonSortedSetDocValues wraps a SortedDocValuesEnum.
        // To prevent mutual inclusion between variants of
        // SortedSetDocValuesEnum and SortedDocValuesEnum,
        // the other variants are encapsulated using
        // SortedSetDocValuesWrapper.
        Lucene90(Lucene90SortedSetDocValuesEnum<I>),
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

    impl<I> SortedSetDocValues for SortedSetDocValuesEnum<I>
    where
        I: IndexInput,
    {
        fn next_ord(&mut self) -> Result<i64> {
            todo!()
        }

        fn doc_value_count(&mut self) -> Result<i32> {
            todo!()
        }

        fn lookup_ord(&mut self, _ord: i64) -> Result<Cow<BytesRef<Vec<u8>>>> {
            todo!()
        }

        fn get_value_count(&mut self) -> Result<i64> {
            todo!()
        }

        fn lookup_term(&mut self, _key: &BytesRef<Vec<u8>>) -> Result<i64> {
            todo!()
        }

        type TermsEnum = DummyTermsEnum;
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

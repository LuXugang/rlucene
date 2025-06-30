/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
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

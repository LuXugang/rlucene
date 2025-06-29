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
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;

/// Exposes a multi-valued view over a single-valued instance.
///
/// This can be used if you want to have one multi-valued implementation that
/// works for both single-valued and multi-valued types.
pub struct SingletonSortedNumericDocValues<N>
where
    N: NumericDocValues,
{
    inner: Option<N>,
}

impl<N> SingletonSortedNumericDocValues<N>
where
    N: NumericDocValues,
{
    pub fn new(inner: N) -> Result<Self> {
        if inner.doc_id() != -1 {
            return Err(LuceneError::illegal_state(format!(
                "iterator has already been used: docID={}",
                inner.doc_id()
            )));
        }
        Ok(Self { inner: Some(inner) })
    }
}

impl<N> DocIdSetIterator for SingletonSortedNumericDocValues<N>
where
    N: NumericDocValues,
{
    fn doc_id(&self) -> i32 {
        self.inner.as_ref().unwrap().doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.inner.as_mut().unwrap().next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.inner.as_mut().unwrap().advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.inner.as_ref().unwrap().cost()
    }
}

impl<N> DocValuesIterator for SingletonSortedNumericDocValues<N>
where
    N: NumericDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.inner.as_mut().unwrap().advance_exact(target)
    }
}

impl<N> SortedNumericDocValues for SingletonSortedNumericDocValues<N>
where
    N: NumericDocValues,
{
    fn next_value(&mut self) -> Result<i64> {
        self.inner.as_mut().unwrap().long_value()
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        Ok(1)
    }

    fn is_single_valued(&self) -> bool {
        true
    }

    type NumericDocValues = N;

    fn get_numeric_doc_values(&mut self) -> Result<Option<Self::NumericDocValues>> {
        if self.inner.as_ref().unwrap().doc_id() != -1 {
            return Err(LuceneError::illegal_state(format!(
                "iterator has already been used: docID={}",
                self.inner.as_ref().unwrap().doc_id()
            )));
        }
        Ok(self.inner.take())
    }
}

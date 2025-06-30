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
use crate::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::Result;
pub struct DummySortedNumericDocValues;

impl DocValuesIterator for DummySortedNumericDocValues {
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl DocIdSetIterator for DummySortedNumericDocValues {
    fn doc_id(&self) -> i32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn next_doc(&mut self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn slow_advance(&mut self, _target: i32) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn cost(&self) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl SortedNumericDocValues for DummySortedNumericDocValues {
    fn next_value(&mut self) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn is_single_valued(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type NumericDocValues = DummyNumericDocValues;

    fn get_numeric_doc_values(&mut self) -> Result<Option<Self::NumericDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

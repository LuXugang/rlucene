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
use std::borrow::Cow;

use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::dummy::dummy_terms_enum::DummyTermsEnum;
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::Result;

pub struct DummySortedDocValues;

impl DocValuesIterator for DummySortedDocValues {}

impl DocIdSetIterator for DummySortedDocValues {
    fn doc_id(&self) -> i32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn next_doc(&mut self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl SortedDocValues for DummySortedDocValues {
    fn ord_value(&mut self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn lookup_ord(&mut self, _ord: i32) -> Result<Cow<BytesRef<Vec<u8>>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_value_count(&mut self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn lookup_term(&mut self, _key: &BytesRef<Vec<u8>>) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type TermsEnum = DummyTermsEnum;

    // fn terms_enum(&mut self) -> Result<TermsEnums<I, AV>> {
    //     unreachable!("Dummy implementation: this method should never be called in real usage")
    // }
}

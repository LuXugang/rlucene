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
use crate::codecs::dummy::dummy_binary_doc_values::DummyBinaryDocValues;
use crate::codecs::dummy::dummy_doc_values_skipper::DummyDocValuesSkipper;
use crate::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::codecs::dummy::dummy_sorted_doc_values::DummySortedDocValues;
use crate::codecs::dummy::dummy_sorted_numeric_doc_values::DummySortedNumericDocValues;
use crate::codecs::dummy::dummy_sorted_set_doc_values::DummySortedSetDocValues;
use crate::index::field_infos::FieldInfos;
use crate::index::leaf_reader::LeafReader;
use std::rc::Rc;

pub struct DummyLeafReader;
impl LeafReader for DummyLeafReader {
    type NumericDocValues = DummyNumericDocValues;

    fn get_numeric_doc_values(
        &mut self,
        _field: &str,
    ) -> crate::util::error::lucene_error::Result<Option<Self::NumericDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type BinaryDocValues = DummyBinaryDocValues;

    fn get_binary_doc_values(
        &mut self,
        _field: &str,
    ) -> crate::util::error::lucene_error::Result<Option<Self::BinaryDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type SortedDocValues = DummySortedDocValues;

    fn get_sorted_doc_values(
        &mut self,
        _field: &str,
    ) -> crate::util::error::lucene_error::Result<Option<Self::SortedDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type SortedNumericDocValues = DummySortedNumericDocValues;

    fn get_sorted_numeric_doc_values(
        &mut self,
        _field: &str,
    ) -> crate::util::error::lucene_error::Result<Option<Self::SortedNumericDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type SortedSetDocValues = DummySortedSetDocValues;

    fn get_sorted_set_doc_values(
        &mut self,
        _field: &str,
    ) -> crate::util::error::lucene_error::Result<Option<Self::SortedSetDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type NormNumericDocValues = DummyNumericDocValues;

    fn get_norm_values(
        &mut self,
        _field: &str,
    ) -> crate::util::error::lucene_error::Result<Option<Self::NormNumericDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type DocValuesSkipper = DummyDocValuesSkipper;

    fn get_doc_values_skipper(
        &mut self,
        _field: &str,
    ) -> crate::util::error::lucene_error::Result<Option<Self::DocValuesSkipper>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_field_infos(&self) -> crate::util::error::lucene_error::Result<&Rc<FieldInfos>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

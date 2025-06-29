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
use crate::index::binary_doc_values::BinaryDocValues;
use crate::index::doc_values_skipper::DocValuesSkipper;
use crate::index::field_infos::FieldInfos;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::util::error::lucene_error::Result;
use std::rc::Rc;

pub trait LeafReader {
    type NumericDocValues: NumericDocValues;
    fn get_numeric_doc_values(&mut self, field: &str) -> Result<Option<Self::NumericDocValues>>;

    type BinaryDocValues: BinaryDocValues;
    fn get_binary_doc_values(&mut self, field: &str) -> Result<Option<Self::BinaryDocValues>>;

    type SortedDocValues: SortedDocValues;
    fn get_sorted_doc_values(&mut self, field: &str) -> Result<Option<Self::SortedDocValues>>;

    type SortedNumericDocValues: SortedNumericDocValues;
    fn get_sorted_numeric_doc_values(
        &mut self,
        field: &str,
    ) -> Result<Option<Self::SortedNumericDocValues>>;

    type SortedSetDocValues: SortedSetDocValues;
    fn get_sorted_set_doc_values(
        &mut self,
        field: &str,
    ) -> Result<Option<Self::SortedSetDocValues>>;

    type NormNumericDocValues: NumericDocValues;
    fn get_norm_values(&mut self, field: &str) -> Result<Option<Self::NormNumericDocValues>>;

    type DocValuesSkipper: DocValuesSkipper;
    fn get_doc_values_skipper(&mut self, field: &str) -> Result<Option<Self::DocValuesSkipper>>;

    fn get_field_infos(&self) -> Result<&Rc<FieldInfos>>;
}

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
use crate::codecs::dummy::dummy_binary_doc_values::DummyBinaryDocValues;
use crate::codecs::dummy::dummy_doc_values_skipper::DummyDocValuesSkipper;
use crate::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::codecs::dummy::dummy_sorted_doc_values::DummySortedDocValues;
use crate::codecs::dummy::dummy_sorted_numeric_doc_values::DummySortedNumericDocValues;
use crate::codecs::dummy::dummy_sorted_set_doc_values::DummySortedSetDocValues;
use crate::index::field_infos::FieldInfos;
use crate::index::index_reader::IndexReader;
use crate::index::leaf_reader::LeafReader;
use crate::util::dummy::dummy_bits::DummyBits;
use crate::util::error::lucene_error::Result;
use std::rc::Rc;
use std::sync::Arc;

pub struct DummyLeafReader;

impl IndexReader for DummyLeafReader {
    fn max_doc(&self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn num_docs(&self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl LeafReader for DummyLeafReader {
    type NumericDocValues = DummyNumericDocValues;

    fn get_numeric_doc_values(&mut self, _field: &str) -> Result<Option<Self::NumericDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type BinaryDocValues = DummyBinaryDocValues;

    fn get_binary_doc_values(&mut self, _field: &str) -> Result<Option<Self::BinaryDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type SortedDocValues = DummySortedDocValues;

    fn get_sorted_doc_values(&mut self, _field: &str) -> Result<Option<Self::SortedDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type SortedNumericDocValues = DummySortedNumericDocValues;

    fn get_sorted_numeric_doc_values(
        &mut self,
        _field: &str,
    ) -> Result<Option<Self::SortedNumericDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type SortedSetDocValues = DummySortedSetDocValues;

    fn get_sorted_set_doc_values(
        &mut self,
        _field: &str,
    ) -> Result<Option<Self::SortedSetDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type NormNumericDocValues = DummyNumericDocValues;

    fn get_norm_values(&mut self, _field: &str) -> Result<Option<Self::NormNumericDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type DocValuesSkipper = DummyDocValuesSkipper;

    fn get_doc_values_skipper(&mut self, _field: &str) -> Result<Option<Self::DocValuesSkipper>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_field_infos(&self) -> Result<&Rc<FieldInfos>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type Bits = DummyBits;

    fn get_live_docs(&self) -> Result<Option<Arc<Self::Bits>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

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
use crate::index::binary_doc_values::BinaryDocValues;
use crate::index::doc_values_skipper::DocValuesSkipper;
use crate::index::field_infos::FieldInfos;
use crate::index::index_reader::IndexReader;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::util::bits::Bits;
use crate::util::error::lucene_error::Result;
use std::rc::Rc;

pub trait LeafReader: IndexReader {
    type NumericDocValues: NumericDocValues;
    fn get_numeric_doc_values(&self, field: &str) -> Result<Option<Self::NumericDocValues>>;

    type BinaryDocValues: BinaryDocValues;
    fn get_binary_doc_values(&self, field: &str) -> Result<Option<Self::BinaryDocValues>>;

    type SortedDocValues: SortedDocValues;
    fn get_sorted_doc_values(&self, field: &str) -> Result<Option<Self::SortedDocValues>>;

    type SortedNumericDocValues: SortedNumericDocValues;
    fn get_sorted_numeric_doc_values(
        &self,
        field: &str,
    ) -> Result<Option<Self::SortedNumericDocValues>>;

    type SortedSetDocValues: SortedSetDocValues;
    fn get_sorted_set_doc_values(&self, field: &str) -> Result<Option<Self::SortedSetDocValues>>;

    type NormNumericDocValues: NumericDocValues;
    fn get_norm_values(&self, field: &str) -> Result<Option<Self::NormNumericDocValues>>;

    type DocValuesSkipper: DocValuesSkipper;
    fn get_doc_values_skipper(&self, field: &str) -> Result<Option<Self::DocValuesSkipper>>;

    fn get_field_infos(&self) -> Result<Rc<FieldInfos>>;

    type Bits: Bits;
    fn get_live_docs(&self) -> Result<Option<Self::Bits>>;
}

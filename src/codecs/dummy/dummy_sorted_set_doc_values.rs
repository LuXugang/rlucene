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
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::dummy::dummy_terms_enum::DummyTermsEnum;
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::Result;

pub struct DummySortedSetDocValues;

impl DocValuesIterator for DummySortedSetDocValues {}

impl DocIdSetIterator for DummySortedSetDocValues {
    fn doc_id(&self) -> i32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn next_doc(&mut self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl SortedSetDocValues for DummySortedSetDocValues {
    fn next_ord(&mut self) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
    type TermsEnum = DummyTermsEnum;
}

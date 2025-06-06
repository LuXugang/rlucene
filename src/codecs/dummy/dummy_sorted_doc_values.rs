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
        todo!()
    }

    fn next_doc(&mut self) -> Result<i32> {
        todo!()
    }
}

impl SortedDocValues for DummySortedDocValues {
    fn ord_value(&mut self) -> Result<i32> {
        todo!()
    }

    fn lookup_ord(&mut self, _ord: i32) -> Result<Cow<BytesRef<Vec<u8>>>> {
        todo!()
    }

    fn get_value_count(&mut self) -> Result<i32> {
        todo!()
    }

    fn lookup_term(&mut self, _key: &BytesRef<Vec<u8>>) -> Result<i32> {
        todo!()
    }

    type TermsEnum = DummyTermsEnum;

    // fn terms_enum(&mut self) -> Result<TermsEnums<I, AV>> {
    //     todo!()
    // }
}

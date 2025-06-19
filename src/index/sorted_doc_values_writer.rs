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
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::sorted_doc_values_terms_enum::SortedDocValuesTermsEnum;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use std::borrow::Cow;

pub(crate) struct SortedDocValuesWriter;

pub(crate) struct SortingSortedDocValues<S>
where
    S: SortedDocValues,
{
    input: S,
    ords: Vec<i32>,
    doc_id: i32,
}

impl<S> SortingSortedDocValues<S>
where
    S: SortedDocValues,
{
    pub fn new(input: S, ords: Vec<i32>) -> Self {
        Self {
            input,
            ords,
            doc_id: -1,
        }
    }
}

impl<S> DocValuesIterator for SortingSortedDocValues<S>
where
    S: SortedDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        // needed in IndexSorter#StringSorter
        self.doc_id = target;
        Ok(self.ords[target as usize] != -1)
    }
}

impl<S> DocIdSetIterator for SortingSortedDocValues<S>
where
    S: SortedDocValues,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        loop {
            self.doc_id += 1;
            if self.doc_id as usize == self.ords.len() {
                self.doc_id = NO_MORE_DOCS;
                break;
            }
            if self.ords[self.doc_id as usize] != -1 {
                break;
            }
            // skip missing docs
        }
        Ok(self.doc_id)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation("use next_doc instead"))
    }

    fn cost(&self) -> Result<i64> {
        self.input.cost()
    }
}

impl<S> SortedDocValues for SortingSortedDocValues<S>
where
    S: SortedDocValues,
{
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ords[self.doc_id as usize])
    }

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<BytesRef<Vec<u8>>>> {
        self.input.lookup_ord(ord)
    }

    fn get_value_count(&mut self) -> Result<i32> {
        self.input.get_value_count()
    }

    type TermsEnum = SortedDocValuesTermsEnum;
}

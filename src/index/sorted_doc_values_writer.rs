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
use crate::util::bytes_ref_hash::STBytesRefHash;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use crate::util::packed::packed_long_values::{PackedLongValues, PackedLongValuesIterator};
use std::borrow::Cow;

pub(crate) struct SortedDocValuesWriter;

pub(crate) struct BufferedSortedDocValues<D>
where
    D: DocIdSetIterator,
{
    hash: STBytesRefHash,
    scratch: BytesRef<Vec<u8>>,
    sorted_values: Vec<i32>,
    ord_map: Vec<i32>,
    ord: i32,
    iter: PackedLongValuesIterator,
    docs_with_field: D,
}

impl<D> BufferedSortedDocValues<D>
where
    D: DocIdSetIterator,
{
    pub fn new(
        hash: STBytesRefHash,
        doc_to_ord: &PackedLongValues,
        sorted_values: Vec<i32>,
        ord_map: Vec<i32>,
        docs_with_field: D,
    ) -> Result<Self> {
        Ok(Self {
            hash,
            scratch: BytesRef::new(),
            sorted_values,
            ord_map,
            ord: -1,
            iter: doc_to_ord.iterator()?,
            docs_with_field,
        })
    }
}

impl<D> DocValuesIterator for BufferedSortedDocValues<D>
where
    D: DocIdSetIterator,
{
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl<D> DocIdSetIterator for BufferedSortedDocValues<D>
where
    D: DocIdSetIterator,
{
    fn doc_id(&self) -> i32 {
        self.docs_with_field.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let doc_id = self.docs_with_field.next_doc()?;
        if doc_id != NO_MORE_DOCS {
            let raw_ord: i32 = self.iter.next_value()?.try_into()?;
            let mapped = self.ord_map[raw_ord as usize];
            self.ord = mapped;
        }
        Ok(doc_id)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation("use next_doc instead"))
    }

    fn cost(&self) -> Result<i64> {
        self.docs_with_field.cost()
    }
}

impl<D> SortedDocValues for BufferedSortedDocValues<D>
where
    D: DocIdSetIterator,
{
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ord)
    }

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<BytesRef<Vec<u8>>>> {
        debug_assert!(ord >= 0 && (ord as usize) < self.sorted_values.len());
        let index = self.sorted_values[ord as usize];
        debug_assert!(
            index >= 0 && (index as usize) < self.sorted_values.len(),
            "sorted_values[ord] out of range"
        );
        self.hash.get(index, &mut self.scratch);
        Ok(Cow::Borrowed(&self.scratch))
    }

    fn get_value_count(&mut self) -> Result<i32> {
        Ok(self.hash.size())
    }

    type TermsEnum = SortedDocValuesTermsEnum;
}

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

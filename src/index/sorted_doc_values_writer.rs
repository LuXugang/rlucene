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
use crate::index::docs_with_field_set::DocsWithFieldSet;
use crate::index::field_info::FieldInfo;
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::sorted_doc_values_terms_enum::SortedDocValuesTermsEnum;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::accountable::Accountable;
use crate::util::bit_util::BitUtil;
use crate::util::bytes_ref_hash::{brh_util, BytesRefHash, DirectBytesStartArray, STBytesRefHash};
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use crate::util::packed::packed_long_values::{
    PackedLongValues, PackedLongValuesBuilder, PackedLongValuesIterator,
};
use crate::util::packed::PackedInts;
use crate::util::{byte_block_pool_util, ByteBlockPoolBorrow, Counter, CounterEnumBorrow};
use std::borrow::Cow;
use std::rc::Rc;

pub(crate) struct SortedDocValuesWriter {
    hash: STBytesRefHash,
    pending: PackedLongValuesBuilder,
    docs_with_field: DocsWithFieldSet,
    iw_bytes_used: CounterEnumBorrow,
    bytes_used: i64, // this currently only tracks differences in 'pending'
    field_info: Rc<FieldInfo>,
    last_doc_id: i32,

    final_ords: Option<PackedLongValues>,
    final_sorted_values: Option<Vec<i32>>,
    final_ord_map: Option<Vec<i32>>,
}

impl SortedDocValuesWriter {
    pub fn new(
        field_info: Rc<FieldInfo>,
        iw_bytes_used: CounterEnumBorrow,
        pool: ByteBlockPoolBorrow,
    ) -> Result<Self> {
        let bytes_start_array =
            DirectBytesStartArray::with_counter(brh_util::DEFAULT_CAPACITY, iw_bytes_used.clone());
        let hash = BytesRefHash::from_bytes_start_array(
            pool,
            brh_util::DEFAULT_CAPACITY,
            bytes_start_array,
        );
        let pending =
            PackedLongValues::delta_packed_long_values_builder_default(PackedInts::COMPACT)?;
        let docs_with_field = DocsWithFieldSet::new();

        let bytes_used = pending.ram_bytes_used()? + docs_with_field.ram_bytes_used()?;
        iw_bytes_used.borrow_mut().add_and_get(bytes_used);

        Ok(Self {
            hash,
            pending,
            docs_with_field,
            iw_bytes_used,
            bytes_used,
            field_info,
            last_doc_id: -1,
            final_ords: None,
            final_sorted_values: None,
            final_ord_map: None,
        })
    }

    pub fn add_value(&mut self, doc_id: i32, value: &BytesRef<Vec<u8>>) -> Result<()> {
        if doc_id <= self.last_doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "DocValuesField \"{}\" appears more than once in this document (only one value is allowed per field)",
                self.field_info.name
            )));
        }

        if value.length > (byte_block_pool_util::BYTE_BLOCK_SIZE as usize - 2) {
            return Err(LuceneError::illegal_argument(format!(
                "DocValuesField \"{}\" is too large, must be <= {}",
                self.field_info.name,
                byte_block_pool_util::BYTE_BLOCK_SIZE - 2
            )));
        }

        self.add_one_value(value)?;
        self.docs_with_field.add(doc_id)?;
        self.last_doc_id = doc_id;
        Ok(())
    }

    fn add_one_value(&mut self, value: &BytesRef<Vec<u8>>) -> Result<()> {
        let mut term_id = self.hash.add(value)?;
        if term_id < 0 {
            term_id = -term_id - 1;
        } else {
            // reserve additional space for each unique value:
            // 1. when indexing, when hash is 50% full, rehash() suddenly needs 2*size ints.
            //    TODO: can this same OOM happen in THPF?
            // 2. when flushing, we need 1 int per value (slot in the ordMap).
            self.iw_bytes_used
                .borrow_mut()
                .add_and_get((2 * BitUtil::INT_BYTES) as i64);
        }

        self.pending.add(term_id as i64)?;
        self.update_bytes_used()
    }

    fn update_bytes_used(&mut self) -> Result<()> {
        let new_bytes_used =
            self.pending.ram_bytes_used()? + self.docs_with_field.ram_bytes_used()?;
        let delta = new_bytes_used - self.bytes_used;
        self.iw_bytes_used.borrow_mut().add_and_get(delta);
        self.bytes_used = new_bytes_used;
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        if self.final_sorted_values.is_none() {
            let value_count = self.hash.size();
            self.update_bytes_used()?;
            debug_assert!(self.final_ord_map.is_none() && self.final_ords.is_none());

            self.hash.sort()?;
            let ords = self.pending.build()?;

            let mut ord_map = vec![0i32; value_count as usize];
            for (ord, &idx) in self.hash.ids.iter().enumerate() {
                ord_map[idx as usize] = ord as i32;
            }

            self.final_sorted_values = Some(std::mem::take(&mut self.hash.ids));
            self.final_ords = Some(ords);
            self.final_ord_map = Some(ord_map);
        }
        Ok(())
    }
}

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

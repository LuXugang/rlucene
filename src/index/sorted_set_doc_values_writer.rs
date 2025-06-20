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
use crate::codecs::dummy::dummy_sorted_doc_values::DummySortedDocValues;
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::sorted_doc_values_terms_enum::SortedDocValuesTermsEnum;
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::index::sorter::DocMap;
use crate::index::{docs_with_field_set::DocsWithFieldSet, field_info::FieldInfo, BytesRef};
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::accountable::Accountable;
use crate::util::array_util::ArrayUtil;
use crate::util::bit_util::BitUtil;
use crate::util::bytes_ref_hash::{brh_util, BytesRefHash, DirectBytesStartArray, STBytesRefHash};
use crate::util::counter::CounterEnumBorrow;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::long_values::LongValues;
use crate::util::packed::growable_writer::GrowableWriter;
use crate::util::packed::packed_long_values::{
    PackedLongValues, PackedLongValuesBuilder, PackedLongValuesIterator,
};
use crate::util::packed::{Mutable, PackedInts, Reader};
use crate::util::{byte_block_pool_util, ByteBlockPoolBorrow, Counter};
use std::borrow::Cow;
use std::rc::Rc;

/// Buffers up pending `[u8]`s per doc, deref and sorting via int ord, then flushes when segment flushes.
pub(crate) struct SortedSetDocValuesWriter {
    hash: STBytesRefHash,
    pending: PackedLongValuesBuilder, // stream of all termIDs
    pending_counts: Option<PackedLongValuesBuilder>, // termIDs per doc
    docs_with_field: DocsWithFieldSet,
    iw_bytes_used: CounterEnumBorrow,
    bytes_used: i64, // this only tracks differences in 'pending' and 'pendingCounts'
    field_info: Rc<FieldInfo>,

    current_doc: i32,
    current_values: Vec<i32>,
    current_upto: usize,
    max_count: i32,

    final_ords: Option<PackedLongValues>,
    final_ord_counts: Option<PackedLongValues>,
    // In Java Lucene, `finalSortedValues` corresponds to the `ids` array inside BytesRefHash.
    // Due to language limitations, we do not need to explicitly define finalSortedValues in Rust.
    // Instead of storing the sorted array,
    // we can simply define an `is_sorted` field to indicate whether the BytesRefHash::sort method has been called.
    is_sorted: bool,
    final_ord_map: Option<Rc<Vec<i32>>>,
}

impl SortedSetDocValuesWriter {
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
        // TODO: memory calculation not implemented
        let bytes_used = pending.ram_bytes_used()? + docs_with_field.ram_bytes_used()?;
        iw_bytes_used.borrow_mut().add_and_get(bytes_used);
        Ok(Self {
            hash,
            pending,
            pending_counts: None,
            docs_with_field,
            iw_bytes_used,
            bytes_used: 0,
            field_info,
            current_doc: -1,
            current_values: Vec::with_capacity(8),
            current_upto: 0,
            max_count: 0,
            final_ords: None,
            final_ord_counts: None,
            is_sorted: false,
            final_ord_map: None,
        })
    }

    pub fn add_value(&mut self, doc_id: i32, value: &BytesRef<Vec<u8>>) -> Result<()> {
        debug_assert!(doc_id >= self.current_doc);
        if value.length > (byte_block_pool_util::BYTE_BLOCK_SIZE as usize - 2) {
            return Err(LuceneError::illegal_argument(format!(
                "DocValuesField \"{}\" is too large, must be <= {}",
                self.field_info.name,
                byte_block_pool_util::BYTE_BLOCK_SIZE - 2
            )));
        }
        if doc_id != self.current_doc {
            self.finish_current_doc()?;
            self.current_doc = doc_id;
        }
        self.add_one_value(value)?;
        self.update_bytes_used()
    }
    // finalize currentDoc: this deduplicates the current term ids
    fn finish_current_doc(&mut self) -> Result<()> {
        if self.current_doc == -1 {
            return Ok(());
        }
        if self.current_upto > 1 {
            self.current_values[..self.current_upto].sort_unstable();
        }
        let mut last_value = -1;
        let mut count = 0;
        for &term_id in &self.current_values[..self.current_upto] {
            // if it's not a duplicate
            if term_id != last_value {
                self.pending.add(term_id as i64)?;
                count += 1;
            }
            last_value = term_id;
        }
        // record the number of unique term ids for this doc
        if let Some(ref mut pc) = self.pending_counts {
            pc.add(count as i64)?;
        } else if count != 1 {
            let mut pc =
                PackedLongValues::delta_packed_long_values_builder_default(PackedInts::COMPACT)?;
            for _ in 0..self.docs_with_field.cardinality() {
                pc.add(1)?;
            }
            pc.add(count as i64)?;
            self.pending_counts = Some(pc);
        }
        self.max_count = self.max_count.max(count);
        self.current_upto = 0;
        self.docs_with_field.add(self.current_doc)?;
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
        if self.current_upto == self.current_values.len() {
            let old_cap = self.current_values.len();
            ArrayUtil::grow_with_len(&mut self.current_values, old_cap + 1);
            self.iw_bytes_used.borrow_mut().add_and_get(
                ((self.current_values.len() - self.current_upto) * BitUtil::INT_BYTES) as i64,
            );
        }
        self.current_values[self.current_upto] = term_id;
        self.current_upto += 1;
        Ok(())
    }

    fn update_bytes_used(&mut self) -> Result<()> {
        let pc_used = if let Some(ref pc) = self.pending_counts {
            pc.ram_bytes_used()?
        } else {
            0
        };
        // TODO: memory calculation not implemented
        let new_used =
            self.pending.ram_bytes_used()? + pc_used + self.docs_with_field.ram_bytes_used()?;
        self.iw_bytes_used
            .borrow_mut()
            .add_and_get(new_used - self.bytes_used);
        self.bytes_used = new_used;
        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        self.docs_with_field.finish();
        if self.final_ords.is_none() {
            debug_assert!(
                self.final_ord_counts.is_none() && !self.is_sorted && self.final_ord_map.is_none()
            );
            self.finish_current_doc()?;
            let value_count = self.hash.size();
            self.final_ords = Some(self.pending.build()?);
            self.final_ord_counts = match std::mem::take(&mut self.pending_counts) {
                Some(mut pc) => Some(pc.build()?),
                None => None,
            };
            self.hash.sort()?;
            self.is_sorted = true;
            let mut ord_map = vec![0; value_count as usize];
            for ord in 0..value_count as usize {
                let index = self.hash.ids[ord] as usize;
                ord_map[index] = ord as i32;
            }
            self.final_ord_map = Some(Rc::new(ord_map));
        } else {
            debug_assert!(self.is_sorted);
        }
        Ok(())
    }
}

pub(crate) struct BufferedSortedSetDocValues<D>
where
    D: DocIdSetIterator,
{
    ord_map: Rc<Vec<i32>>,
    hash: Rc<STBytesRefHash>,
    scratch: BytesRef<Vec<u8>>,
    ords_iter: PackedLongValuesIterator,
    ord_counts_iter: PackedLongValuesIterator,
    docs_with_field: D,
    current_doc: Vec<i32>,
    ord_count: usize,
    ord_upto: usize,
}

impl<D> BufferedSortedSetDocValues<D>
where
    D: DocIdSetIterator,
{
    pub fn new(
        ord_map: Rc<Vec<i32>>,
        hash: Rc<STBytesRefHash>,
        ords: PackedLongValues,
        ord_counts: PackedLongValues,
        max_count: usize,
        docs_with_field: D,
    ) -> Result<Self> {
        Ok(Self {
            ord_map,
            hash,
            scratch: BytesRef::new(),
            ords_iter: ords.iterator()?,
            ord_counts_iter: ord_counts.iterator()?,
            docs_with_field,
            current_doc: vec![0; max_count],
            ord_count: 0,
            ord_upto: 0,
        })
    }
}

impl<D> DocIdSetIterator for BufferedSortedSetDocValues<D>
where
    D: DocIdSetIterator,
{
    fn doc_id(&self) -> i32 {
        self.docs_with_field.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let doc_id = self.docs_with_field.next_doc()?;
        if doc_id != NO_MORE_DOCS {
            let count = self.ord_counts_iter.next_value()? as usize;
            debug_assert!(count > 0);
            self.ord_count = count;
            for i in 0..count {
                let raw: i32 = self.ords_iter.next_value()?.try_into()?;
                self.current_doc[i] = self.ord_map[raw as usize];
            }
            self.current_doc[..count].sort_unstable();
            self.ord_upto = 0;
        }
        Ok(doc_id)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn cost(&self) -> Result<i64> {
        self.docs_with_field.cost()
    }
}

impl<D> DocValuesIterator for BufferedSortedSetDocValues<D>
where
    D: DocIdSetIterator,
{
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl<D> SortedSetDocValues for BufferedSortedSetDocValues<D>
where
    D: DocIdSetIterator,
{
    fn next_ord(&mut self) -> Result<i64> {
        let ord = self.current_doc[self.ord_upto] as i64;
        self.ord_upto += 1;
        Ok(ord)
    }

    fn lookup_ord(&mut self, ord: i64) -> Result<Cow<BytesRef<Vec<u8>>>> {
        debug_assert!(ord >= 0 && (ord as usize) < self.ord_map.len());
        let idx: i32 = ord.try_into()?;
        let hash_idx = self.hash.ids[idx as usize];
        self.hash.get(hash_idx, &mut self.scratch);
        Ok(Cow::Borrowed(&self.scratch))
    }

    type TermsEnum = SortedDocValuesTermsEnum;

    fn doc_value_count(&mut self) -> Result<i32> {
        Ok(self.ord_count as i32)
    }

    fn get_value_count(&mut self) -> Result<i64> {
        Ok(self.ord_map.len() as i64)
    }

    type SortedDocValues = DummySortedDocValues;
}

pub(crate) struct SortingSortedSetDocValues<S>
where
    S: SortedSetDocValues,
{
    input: S,
    ords: DocOrds,
    doc_id: i32,
    ord_upto: i64,
    count: i32,
}

impl<S> SortingSortedSetDocValues<S>
where
    S: SortedSetDocValues,
{
    pub fn new(input: S, ords: DocOrds) -> Self {
        Self {
            input,
            ords,
            doc_id: -1,
            ord_upto: 0,
            count: 0,
        }
    }

    fn init_count(&mut self) -> Result<()> {
        debug_assert!(self.ord_upto > 0);
        self.ord_upto = self.ords.offsets[self.doc_id as usize] - 1;
        self.count = self.ords.doc_value_counts.get(self.doc_id)?.try_into()?;
        Ok(())
    }
}

impl<S> DocValuesIterator for SortingSortedSetDocValues<S>
where
    S: SortedSetDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        // needed in IndexSorter#StringSorter
        self.doc_id = target;
        self.init_count()?;
        Ok(self.ords.offsets[self.doc_id as usize] > 0)
    }
}

impl<S> DocIdSetIterator for SortingSortedSetDocValues<S>
where
    S: SortedSetDocValues,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        loop {
            self.doc_id += 1;
            if (self.doc_id as usize) == self.ords.offsets.len() {
                self.doc_id = NO_MORE_DOCS;
                break;
            }
            if self.ords.offsets[self.doc_id as usize] > 0 {
                break;
            }
        }
        self.init_count()?;
        Ok(self.doc_id)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation("use next_doc instead"))
    }

    fn cost(&self) -> Result<i64> {
        self.input.cost()
    }
}

impl<S> SortedSetDocValues for SortingSortedSetDocValues<S>
where
    S: SortedSetDocValues,
{
    fn next_ord(&mut self) -> Result<i64> {
        let ord = self.ords.ords.get(self.ord_upto)?;
        self.ord_upto += 1;
        Ok(ord)
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        debug_assert!(self.doc_id >= 0);
        Ok(self.count)
    }

    fn lookup_ord(&mut self, ord: i64) -> Result<Cow<BytesRef<Vec<u8>>>> {
        self.input.lookup_ord(ord)
    }

    fn get_value_count(&mut self) -> Result<i64> {
        self.input.get_value_count()
    }

    type TermsEnum = SortedDocValuesTermsEnum;

    fn is_single_valued(&self) -> bool {
        self.input.is_single_valued()
    }

    type SortedDocValues = S::SortedDocValues;

    fn get_sorted_doc_values(&mut self) -> Result<Option<Self::SortedDocValues>> {
        self.input.get_sorted_doc_values()
    }
}

#[derive(Clone)]
pub(crate) struct DocOrds {
    pub(crate) offsets: Rc<Vec<i64>>,
    pub(crate) ords: PackedLongValues,
    pub(crate) doc_value_counts: Rc<GrowableWriter>,
}

impl DocOrds {
    pub fn new<DM>(
        max_doc: i32,
        sort_map: &DM,
        old_values: &mut impl SortedSetDocValues,
        acceptable_overhead_ratio: f32,
        bits_per_value: i32,
    ) -> Result<Self>
    where
        DM: DocMap,
    {
        let mut offsets = vec![0i64; max_doc as usize];
        let mut builder =
            PackedLongValues::packed_long_values_builder_default(acceptable_overhead_ratio)?;
        let mut doc_value_counts =
            GrowableWriter::new(bits_per_value, max_doc, acceptable_overhead_ratio)?;
        let mut ord_offset = 1i64;
        while let doc_id = old_values.next_doc()? {
            if doc_id == NO_MORE_DOCS {
                break;
            }
            let new_doc_id = sort_map.old_to_new(doc_id);
            let start_offset = ord_offset;
            let doc_value_count = old_values.doc_value_count()?;
            ord_offset += doc_value_count as i64;
            for _ in 0..doc_value_count {
                builder.add(old_values.next_ord()?)?;
            }

            doc_value_counts.set(new_doc_id, ord_offset - start_offset)?;

            if start_offset != ord_offset {
                // do we have any values?
                offsets[new_doc_id as usize] = start_offset;
            }
        }
        let ords = builder.build()?;

        Ok(DocOrds {
            offsets: Rc::new(offsets),
            ords,
            doc_value_counts: Rc::new(doc_value_counts),
        })
    }
}

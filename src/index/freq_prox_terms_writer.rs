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
use crate::index::postings_enum::PostingsEnum;
use crate::index::sorter::DocMap;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::byte_buffers_data_input::ByteBuffersDataInputOwned;
use crate::store::{ByteBuffersDataOutput, DataInput, DataOutput};
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::Result;
use crate::util::lsb_radix_sorter::LSBRadixSorter;
use crate::util::packed::PackedInts;
use crate::util::{SliceCopyOps, Sorter, TimSorter, TimSorterBase};

pub(crate) struct FreqProxTermsWriter;

pub(crate) struct SortingDocsEnum<P>
where
    P: PostingsEnum,
{
    sorter: LSBRadixSorter,
    postings_enum: Option<P>,
    docs: Vec<i32>,
    doc_it: i32,
    upto: i32,
}
impl<P> SortingDocsEnum<P>
where
    P: PostingsEnum,
{
    pub(crate) fn new() -> Self {
        Self {
            sorter: LSBRadixSorter::new(),
            postings_enum: None,
            docs: Vec::new(),
            doc_it: -1,
            upto: 0,
        }
    }
    pub(crate) fn reset(&mut self, doc_map: &impl DocMap, mut postings_enum: P) -> Result<()> {
        let mut i = 0;
        loop {
            let doc = postings_enum.next_doc()?;
            if doc == NO_MORE_DOCS {
                break;
            }
            if self.docs.len() <= i {
                ArrayUtil::grow(&mut self.docs)?;
            }
            self.docs[i] = doc_map.old_to_new(doc);
            i += 1;
        }

        self.upto = i as i32;
        if self.docs.len() == self.upto as usize {
            ArrayUtil::grow(&mut self.docs)?;
        }
        self.docs[self.upto as usize] = NO_MORE_DOCS;

        let max_doc = doc_map.size();
        let num_bits = PackedInts::bits_required(std::cmp::max(0, (max_doc - 1) as i64))? as usize;
        // Even though LSBRadixSorter cannot take advantage of partial ordering like
        // TimSorter it is often still faster for nearly-sorted inputs.
        self.sorter
            .sort(num_bits, &mut self.docs, self.upto as usize);
        self.doc_it = -1;
        self.postings_enum = Some(postings_enum);
        Ok(())
    }
}

impl<P> DocIdSetIterator for SortingDocsEnum<P>
where
    P: PostingsEnum,
{
    fn doc_id(&self) -> i32 {
        if self.doc_it < 0 {
            -1
        } else {
            self.docs[self.doc_it as usize]
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.doc_it += 1;
        Ok(self.docs[self.doc_it as usize])
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.slow_advance(target)
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.upto as i64)
    }
}

impl<P> PostingsEnum for SortingDocsEnum<P>
where
    P: PostingsEnum,
{
    fn freq(&mut self) -> Result<i32> {
        Ok(1)
    }

    fn next_position(&mut self) -> Result<i32> {
        Ok(-1)
    }

    fn start_offset(&self) -> Result<i32> {
        Ok(-1)
    }

    fn end_offset(&self) -> Result<i32> {
        Ok(-1)
    }

    fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
        Ok(None)
    }
}

struct DocOffsetSorter<'a> {
    docs: &'a mut [i32],
    offsets: &'a mut [i64],
    tmp_docs: Vec<i32>,
    tmp_offsets: Vec<i64>,
    pivot_index: i32,
}

impl<'a> DocOffsetSorter<'a> {
    pub fn new(
        docs: &'a mut [i32],
        offsets: &'a mut [i64],
        max_temp_slots: usize,
    ) -> TimSorter<DocOffsetSorter<'a>> {
        let tmp_docs = Vec::new();
        let tmp_offsets = Vec::new();
        let sorter = DocOffsetSorter {
            docs,
            offsets,
            tmp_docs,
            tmp_offsets,
            pivot_index: 0,
        };
        TimSorter::new(max_temp_slots as i32, sorter)
    }
}

impl Sorter for DocOffsetSorter<'_> {
    fn compare(&mut self, i: i32, j: i32) -> Result<i32> {
        Ok(self.docs[i as usize] - self.docs[j as usize])
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        let i = i as usize;
        let j = j as usize;
        self.docs.swap(i, j);
        self.offsets.swap(i, j);
        Ok(())
    }

    fn set_pivot(&mut self, i: i32) -> Result<()> {
        self.pivot_index = i;
        Ok(())
    }

    fn compare_pivot(&mut self, j: i32) -> Result<i32> {
        self.compare(self.pivot_index, j)
    }
}

impl TimSorterBase for DocOffsetSorter<'_> {
    fn copy(&mut self, src: i32, dest: i32) {
        let src = src as usize;
        let dest = dest as usize;
        self.docs[dest] = self.docs[src];
        self.offsets[dest] = self.offsets[src];
    }

    fn save(&mut self, i: i32, len: i32) {
        if self.tmp_docs.len() < len as usize {
            let new_len = ArrayUtil::oversize(len as usize, 0);
            self.tmp_docs = vec![0; new_len];
            self.tmp_offsets = vec![0; new_len];
        }
        let i = i as usize;
        let len = len as usize;

        self.tmp_docs.copy_from(&self.docs[i..i + len], 0);
        self.tmp_offsets.copy_from(&self.offsets[i..i + len], 0);
    }

    fn restore(&mut self, i: i32, j: i32) {
        let i = i as usize;
        let j = j as usize;
        self.docs[j] = self.tmp_docs[i];
        self.offsets[j] = self.tmp_offsets[i];
    }

    fn compare_saved(&self, i: i32, j: i32) -> i32 {
        self.tmp_docs[i as usize] - self.docs[j as usize]
    }
}
pub(crate) struct SortingPostingsEnum<P>
where
    P: PostingsEnum,
{
    docs: Vec<i32>,
    offsets: Vec<i64>,
    upto: i32,

    posting_input: Option<ByteBuffersDataInputOwned>,
    postings_enum: Option<P>,

    store_positions: bool,
    store_offsets: bool,

    doc_it: i32,
    pos: i32,
    start_offset: i32,
    end_offset: i32,

    payload: BytesRef<Vec<u8>>,
    curr_freq: i32,

    buffer: ByteBuffersDataOutput,
}
impl<P> SortingPostingsEnum<P>
where
    P: PostingsEnum,
{
    pub fn new(store_positions: bool, store_offsets: bool) -> Self {
        Self {
            docs: Vec::new(),
            offsets: Vec::new(),
            upto: 0,
            posting_input: None,
            postings_enum: None,
            store_positions,
            store_offsets,
            doc_it: -1,
            pos: 0,
            start_offset: 0,
            end_offset: 0,
            payload: BytesRef::new(),
            curr_freq: 0,
            buffer: ByteBuffersDataOutput::new_resettable_instance(),
        }
    }
    pub fn reset(
        &mut self,
        doc_map: &impl DocMap,
        mut postings_enum: P,
        store_positions: bool,
        store_offsets: bool,
    ) -> Result<()> {
        self.store_positions = store_positions;
        self.store_offsets = store_offsets;

        self.doc_it = -1;
        self.start_offset = -1;
        self.end_offset = -1;

        self.buffer.reset();

        let mut i = 0;
        loop {
            let doc = postings_enum.next_doc()?;
            if doc == NO_MORE_DOCS {
                break;
            }
            if i == self.docs.len() {
                let new_length = ArrayUtil::oversize(i + 1, 4);
                ArrayUtil::grow_exact(&mut self.docs, new_length)?;
                ArrayUtil::grow_exact(&mut self.offsets, new_length)?;
            }

            self.docs[i] = doc_map.old_to_new(doc);
            self.offsets[i] = self.buffer.size();

            self.add_positions(&mut postings_enum)?;
            i += 1;
        }
        self.postings_enum = Some(postings_enum);

        debug_assert!(i <= i32::MAX as usize);
        self.upto = i as i32;

        let num_temp_slots = doc_map.size() / 8;
        let mut sorter = DocOffsetSorter::new(&mut self.docs, &mut self.offsets, num_temp_slots);
        sorter.sort(0, self.upto)?;

        self.posting_input = Some(self.buffer.get_data_input_owner());

        Ok(())
    }
    fn add_positions(&mut self, postings: &mut impl PostingsEnum) -> Result<()> {
        let freq = postings.freq()?;
        self.buffer.write_vint(freq)?;

        if self.store_positions {
            let mut previous_position = 0;
            let mut previous_end_offset = 0;

            for _ in 0..freq {
                let pos = postings.next_position()?;
                let payload_opt = postings.get_payload()?;
                // The low-order bit of token is set only if there is a payload, the
                // previous bits are the delta-encoded position.
                let token =
                    ((pos - previous_position) << 1) | if payload_opt.is_some() { 1 } else { 0 };
                self.buffer.write_vint(token)?;
                previous_position = pos;

                if self.store_offsets {
                    // don't encode offsets if they are not stored
                    let start_offset = postings.start_offset()?;
                    let end_offset = postings.end_offset()?;
                    self.buffer.write_vint(start_offset - previous_end_offset)?;
                    self.buffer.write_vint(end_offset - start_offset)?;
                    previous_end_offset = end_offset;
                }

                if let Some(payload) = payload_opt {
                    self.buffer.write_vint(payload.length as i32)?;
                    self.buffer.write_bytes_range(
                        &payload.bytes,
                        payload.offset as i32,
                        payload.length as i32,
                    )?;
                }
            }
        }
        Ok(())
    }
}

impl<P> DocIdSetIterator for SortingPostingsEnum<P>
where
    P: PostingsEnum,
{
    fn doc_id(&self) -> i32 {
        if self.doc_it < 0 {
            -1
        } else if self.doc_it >= self.upto {
            NO_MORE_DOCS
        } else {
            self.docs[self.doc_it as usize]
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.doc_it += 1;
        if self.doc_it >= self.upto {
            return Ok(NO_MORE_DOCS);
        }

        let offset = self.offsets[self.doc_it as usize];
        let posting_input = self.posting_input.as_mut().unwrap();
        posting_input.seek(offset)?;

        posting_input.read_vint()?;

        self.pos = 0;
        self.end_offset = 0;

        Ok(self.docs[self.doc_it as usize])
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        // need to support it for checkIndex, but in practice it won't be called, so
        // don't bother to implement efficiently for now.
        self.slow_advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.postings_enum.as_ref().unwrap().cost()
    }
}

impl<P> PostingsEnum for SortingPostingsEnum<P>
where
    P: PostingsEnum,
{
    fn freq(&mut self) -> Result<i32> {
        Ok(self.curr_freq)
    }

    fn next_position(&mut self) -> Result<i32> {
        if !self.store_positions {
            return Ok(-1);
        }

        let posting_input = self.posting_input.as_mut().unwrap();

        let token = posting_input.read_vint()?;
        self.pos += ((token as u32) >> 1) as i32;

        if self.store_offsets {
            self.start_offset = self.end_offset + posting_input.read_vint()?;
            self.end_offset = self.start_offset + posting_input.read_vint()?;
        }

        if (token & 1) != 0 {
            self.payload.offset = 0;
            let length = posting_input.read_vint()? as usize;
            self.payload.length = length;

            if self.payload.bytes.len() < length {
                let new_length = ArrayUtil::oversize(length, 1);
                self.payload.bytes = vec![0; new_length];
            }

            posting_input.read_bytes(&mut self.payload.bytes, 0, self.payload.length as i32)?;
        } else {
            self.payload.length = 0;
        }

        Ok(self.pos)
    }

    fn start_offset(&self) -> Result<i32> {
        Ok(self.start_offset)
    }

    fn end_offset(&self) -> Result<i32> {
        Ok(self.end_offset)
    }

    fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
        if self.payload.length == 0 {
            Ok(None)
        } else {
            Ok(Some(&self.payload))
        }
    }
}
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rand::prelude::SliceRandom;
    use rand::Rng;

    use crate::index::freq_prox_terms_writer::DocOffsetSorter;
    use crate::test::util::lucene_test_case::{is_night_mode, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::Sorter;

    fn generate_doc_offset_data<R: Rng + ?Sized>(
        random: &mut R,
        len: usize,
    ) -> (Vec<i32>, Vec<i64>) {
        let mut docs = Vec::with_capacity(len);
        let mut offsets = Vec::with_capacity(len);

        let mut doc_id = 0;
        for _ in 0..len {
            doc_id += random.random_range(1..10);
            docs.push(doc_id);
            offsets.push(random.random_range(1000..10_000));
        }
        docs.shuffle(random);

        (docs, offsets)
    }

    fn assert_sorted_and_synced(docs: &[i32], offsets: &[i64], original_map: &HashMap<i32, i64>) {
        assert_eq!(docs.len(), offsets.len());

        for i in 0..docs.len() {
            if i > 0 {
                assert!(
                    docs[i - 1] <= docs[i],
                    "docs not sorted at index {}: {} > {}",
                    i,
                    docs[i - 1],
                    docs[i]
                );
            }

            let doc = docs[i];
            let expected_offset = original_map.get(&doc).expect("missing doc in map");

            assert_eq!(
                offsets[i], *expected_offset,
                "offset mismatch at index {}: doc={} expected={} actual={}",
                i, doc, expected_offset, offsets[i]
            );
        }
    }

    #[test]
    fn test_doc_offset_sorter_basic() {
        let mut random = random();
        let len = if is_night_mode() {
            random.random_range(1000..5000)
        } else {
            random.random_range(10000..20000)
        };

        let (mut docs, mut offsets) = generate_doc_offset_data(&mut random, len);
        assert_eq!(docs.len(), offsets.len());

        let mut original_map: HashMap<i32, i64> = HashMap::with_capacity(len);
        for (doc, offset) in docs.iter().cloned().zip(offsets.iter().cloned()) {
            original_map.insert(doc, offset);
        }

        let max_temp_slots = TestUtil::next_int(&mut random, 0, len as i32);
        let mut sorter = DocOffsetSorter::new(&mut docs, &mut offsets, max_temp_slots as usize);
        sorter.sort(0, len as i32).unwrap();

        assert_sorted_and_synced(&docs, &offsets, &original_map);
    }
}

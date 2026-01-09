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
use crate::core::index::index_writer::MAX_POSITION;
use crate::core::index::merge_state::{DocMapEnum, MergeStateMeta};
use crate::core::index::multi_postings_enum::MultiPostingsEnum;
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::index::{BytesRef, DocIDMerger, DocIDMergerEnum, Sub, SubBase, of_with_max_count};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;
use std::rc::Rc;

/// Exposes flex API, merged from flex API of sub-segments, remapping docIDs (this is used for segment merging).
pub struct MappingMultiPostingsEnum<PE>
where
    PE: PostingsEnum,
{
    // for easy taken
    multi_docs_and_positions_enum: Option<MultiPostingsEnum<PE>>,
    pub(crate) field: String,
    doc_id_merger: DocIDMergerEnum<MappingPostingsSub<PE>>,
    current: Option<usize>,
    all_subs: Vec<MappingPostingsSub<PE>>,
    idxs: Vec<(usize, usize)>,
}
impl<PE> MappingMultiPostingsEnum<PE>
where
    PE: PostingsEnum,
{
    pub(crate) fn new(field: String, merge_state: &MergeStateMeta) -> Result<Self> {
        let mut all_subs = Vec::with_capacity(merge_state.fields_producers_len);

        for i in 0..merge_state.fields_producers_len {
            all_subs.push(MappingPostingsSub::new(merge_state.doc_maps[i].clone()));
        }
        let subs: Vec<Sub<MappingPostingsSub<PE>>> = Vec::new();

        let doc_id_merger = of_with_max_count(subs, all_subs.len(), merge_state.needs_index_sort)?;

        Ok(Self {
            multi_docs_and_positions_enum: None,
            field,
            doc_id_merger,
            current: None,
            all_subs,
            idxs: Vec::new(),
        })
    }
    pub(crate) fn reset(&mut self, mut postings_enum: MultiPostingsEnum<PE>) -> Result<&mut Self> {
        let mut postings = postings_enum.take_postings_enums();
        let subs_array = postings_enum.get_subs();
        let count = postings_enum.get_num_subs() as usize;

        self.doc_id_merger.clear_subs();
        #[allow(clippy::needless_range_loop)]
        for i in 0..count {
            let enum_with_slice = &subs_array[i];
            let reader_index = enum_with_slice.slice.get_reader_index() as usize;

            let sub = &mut self.all_subs[reader_index];
            sub.postings = postings[enum_with_slice.postings_enum_idx].take();
            self.idxs
                .push((reader_index, enum_with_slice.postings_enum_idx));
        }
        let subs = self.doc_id_merger.get_subs_vec();
        debug_assert!(subs.is_empty());
        for i in 0..count {
            let doc_map = self.all_subs[i].doc_map.clone();
            let padding = MappingPostingsSub::new(doc_map);
            let v = std::mem::replace(&mut self.all_subs[i], padding);
            subs.push(Sub::new(v));
        }
        self.multi_docs_and_positions_enum = Some(postings_enum);
        self.doc_id_merger.reset()?;
        Ok(self)
    }
    pub(crate) fn take_multi_docs_and_positions_enum(&mut self) -> Option<MultiPostingsEnum<PE>> {
        let mut postings_enum = self.multi_docs_and_positions_enum.take()?;

        debug_assert!(self.idxs.len() == postings_enum.get_num_subs() as usize);

        let subs = self.doc_id_merger.get_subs_mut();
        for (reader_index, enum_with_slice_postings_enum_idx) in &self.idxs {
            let v = subs[*reader_index].sub.postings.take();
            postings_enum.postings_enums_mut()[*enum_with_slice_postings_enum_idx] = v;
        }

        Some(postings_enum)
    }
}

impl<PE> DocIdSetIterator for MappingMultiPostingsEnum<PE>
where
    PE: PostingsEnum,
{
    fn doc_id(&self) -> i32 {
        match self.current {
            None => -1,
            Some(idx) => self.doc_id_merger.get_subs()[idx].mapped_doc_id,
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.current = self.doc_id_merger.next()?;
        match self.current {
            None => Ok(NO_MORE_DOCS),
            Some(idx) => Ok(self.doc_id_merger.get_subs()[idx].mapped_doc_id),
        }
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn cost(&self) -> Result<i64> {
        let mut cost = 0;
        for sub in self.doc_id_merger.get_subs() {
            if let Some(postings) = &sub.sub.postings {
                cost += postings.cost()?;
            }
        }
        Ok(cost)
    }
}

impl<PE> PostingsEnum for MappingMultiPostingsEnum<PE>
where
    PE: PostingsEnum,
{
    fn freq(&mut self) -> Result<i32> {
        let v = self.current.unwrap();
        self.doc_id_merger.get_subs_mut()[v]
            .sub
            .postings
            .as_mut()
            .unwrap()
            .freq()
    }

    fn next_position(&mut self) -> Result<i32> {
        let idx = self.current.unwrap();
        let postings = self.doc_id_merger.get_subs_mut()[idx]
            .sub
            .postings
            .as_mut()
            .unwrap();

        let pos = postings.next_position()?;
        if pos < 0 {
            return Err(LuceneError::corrupt_index(format!(
                "position={} is negative, field=\"{}\" doc={}",
                pos,
                self.field,
                self.doc_id(),
            )));
        }
        if pos > MAX_POSITION {
            return Err(LuceneError::corrupt_index(format!(
                "position={} is too large (> IndexWriter::MAX_POSITION={}), field=\"{}\" doc={}",
                pos,
                MAX_POSITION,
                self.field,
                self.doc_id(),
            )));
        }
        Ok(pos)
    }

    fn start_offset(&self) -> Result<i32> {
        let idx = self.current.unwrap();
        self.doc_id_merger.get_subs()[idx]
            .sub
            .postings
            .as_ref()
            .unwrap()
            .start_offset()
    }

    fn end_offset(&self) -> Result<i32> {
        let idx = self.current.unwrap();
        self.doc_id_merger.get_subs()[idx]
            .sub
            .postings
            .as_ref()
            .unwrap()
            .end_offset()
    }

    fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        let idx = self.current.unwrap();
        self.doc_id_merger.get_subs()[idx]
            .sub
            .postings
            .as_ref()
            .unwrap()
            .get_payload()
    }
}
pub(crate) struct MappingPostingsSub<PE>
where
    PE: PostingsEnum,
{
    postings: Option<PE>,
    doc_map: Rc<DocMapEnum>,
}
impl<PE> MappingPostingsSub<PE>
where
    PE: PostingsEnum,
{
    fn new(doc_map: Rc<DocMapEnum>) -> Self {
        Self {
            postings: None,
            doc_map,
        }
    }
}
impl<PE> SubBase for MappingPostingsSub<PE>
where
    PE: PostingsEnum,
{
    fn next_doc(&mut self) -> Result<i32> {
        match self.postings {
            Some(ref mut postings_enum) => postings_enum.next_doc(),
            None => Err(LuceneError::illegal_state("PostingsEnum is not set")),
        }
    }

    fn get_doc_map(&self) -> Result<&Rc<DocMapEnum>> {
        Ok(&self.doc_map)
    }
}

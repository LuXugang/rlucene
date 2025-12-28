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
use crate::core::index::BytesRef;
use crate::core::index::index_reader::Identity;
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::index::reader_slice::ReaderSlice;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
/// Exposes [`PostingsEnum`], merged from [`PostingsEnum`] API of sub-segments.
pub struct MultiPostingsEnum<PE>
where
    PE: PostingsEnum,
{
    parent: Identity,
    pub(crate) sub_postings_enums: Vec<Option<PE>>,
    subs: Vec<EnumWithSlice>,
    num_subs: i32,
    upto: i32,
    current: Option<usize>,
    current_base: i32,
    doc: i32,
}
impl<PE> MultiPostingsEnum<PE>
where
    PE: PostingsEnum,
{
    pub fn new(parent: Identity, sub_reader_count: usize) -> Self {
        let mut subs = Vec::with_capacity(sub_reader_count);
        let mut sub_postings_enums = Vec::with_capacity(sub_reader_count);
        for _ in 0..sub_reader_count {
            subs.push(EnumWithSlice::new());
            sub_postings_enums.push(None);
        }
        Self {
            parent,
            sub_postings_enums,
            subs,
            num_subs: 0,
            upto: -1,
            current: None,
            current_base: 0,
            doc: -1,
        }
    }
    /// Returns `true` if this instance can be reused by the provided [`MultiTermsEnum`](crate::core::index::multi_terms_enum::MultiTermsEnum).
    pub fn can_reuse(&self, other: &Identity) -> bool {
        self.parent == *other
    }
    /// Re-use and reset this instance on the provided slices.
    pub fn reset(&mut self, subs: &[EnumWithSlice], num_subs: i32) {
        self.num_subs = num_subs;

        for (i, sub) in subs.iter().enumerate().take(num_subs as usize) {
            self.subs[i].postings_enum = sub.postings_enum;
            self.subs[i].slice = sub.slice.clone();
        }

        self.upto = -1;
        self.doc = -1;
        self.current = None;
    }

    /// How many sub-readers we are merging.
    pub fn get_num_subs(&self) -> i32 {
        self.num_subs
    }

    /// Returns sub-readers we are merging.
    pub fn get_subs(&self) -> &[EnumWithSlice] {
        &self.subs
    }
}

impl<PE> DocIdSetIterator for MultiPostingsEnum<PE>
where
    PE: PostingsEnum,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        loop {
            if self.current.is_none() {
                if self.upto == self.num_subs - 1 {
                    self.doc = NO_MORE_DOCS;
                    return Ok(self.doc);
                } else {
                    self.upto += 1;
                    let idx = self.upto as usize;
                    self.current = Some(idx);
                    self.current_base = self.subs[idx].slice.get_start();
                }
            }

            let idx = self.subs[self.current.unwrap()].postings_enum;
            let doc = self.sub_postings_enums[idx].as_mut().unwrap().next_doc()?;
            if doc != NO_MORE_DOCS {
                self.doc = self.current_base + doc;
                return Ok(self.doc);
            } else {
                self.current = None;
            }
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        debug_assert!(target > self.doc);
        loop {
            if let Some(idx) = self.current {
                let doc = if target < self.current_base {
                    // target was in the previous slice but there was no matching doc after it
                    self.sub_postings_enums[self.subs[idx].postings_enum]
                        .as_mut()
                        .unwrap()
                        .next_doc()?
                } else {
                    self.sub_postings_enums[self.subs[idx].postings_enum]
                        .as_mut()
                        .unwrap()
                        .advance(target - self.current_base)?
                };

                if doc == NO_MORE_DOCS {
                    self.current = None;
                } else {
                    self.doc = doc + self.current_base;
                    return Ok(self.doc);
                }
            } else if self.upto == self.num_subs - 1 {
                self.doc = NO_MORE_DOCS;
                return Ok(self.doc);
            } else {
                self.upto += 1;
                let idx = self.upto as usize;
                self.current = Some(idx);
                self.current_base = self.subs[idx].slice.get_start();
            }
        }
    }

    fn cost(&self) -> Result<i64> {
        let mut cost: i64 = 0;
        for i in 0..(self.num_subs as usize) {
            let pe_idx = self.subs[i].postings_enum;
            cost += self.sub_postings_enums[pe_idx].as_ref().unwrap().cost()?;
        }
        Ok(cost)
    }
}

impl<PE> PostingsEnum for MultiPostingsEnum<PE>
where
    PE: PostingsEnum,
{
    fn freq(&mut self) -> Result<i32> {
        match self.current {
            Some(idx) => self.sub_postings_enums[self.subs[idx].postings_enum]
                .as_mut()
                .unwrap()
                .freq(),
            None => Err(LuceneError::illegal_state("No current sub PostingsEnum")),
        }
    }

    fn next_position(&mut self) -> Result<i32> {
        match self.current {
            Some(idx) => self.sub_postings_enums[self.subs[idx].postings_enum]
                .as_mut()
                .unwrap()
                .next_position(),
            None => Err(LuceneError::illegal_state("No current sub PostingsEnum")),
        }
    }

    fn start_offset(&self) -> Result<i32> {
        match self.current {
            Some(idx) => self.sub_postings_enums[self.subs[idx].postings_enum]
                .as_ref()
                .unwrap()
                .start_offset(),
            None => Err(LuceneError::illegal_state("No current sub PostingsEnum")),
        }
    }

    fn end_offset(&self) -> Result<i32> {
        match self.current {
            Some(idx) => self.sub_postings_enums[self.subs[idx].postings_enum]
                .as_ref()
                .unwrap()
                .end_offset(),
            None => Err(LuceneError::illegal_state("No current sub PostingsEnum")),
        }
    }

    fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        match self.current {
            Some(idx) => self.sub_postings_enums[self.subs[idx].postings_enum]
                .as_ref()
                .unwrap()
                .get_payload(),
            None => Err(LuceneError::illegal_state("No current sub PostingsEnum")),
        }
    }
}
impl<PE> Display for MultiPostingsEnum<PE>
where
    PE: PostingsEnum,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for sub in self.get_subs().iter() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{}", sub)?;
        }
        write!(f, "])")
    }
}
/// Holds a [`PostingsEnum`] along with the corresponding [`ReaderSlice`].
pub struct EnumWithSlice {
    /// [`PostingsEnum`]'s idx for this sub-reader
    pub(crate) postings_enum: usize,
    /// [`ReaderSlice`] describing how this sub-reader fits into the composite reader.
    pub(crate) slice: Rc<ReaderSlice>,
}
impl EnumWithSlice {
    /// Creates a new `EnumWithSlice`.
    pub fn new() -> Self {
        Self {
            postings_enum: 0,
            slice: Rc::new(ReaderSlice::default()),
        }
    }
    pub fn with_slice(slice: Rc<ReaderSlice>) -> Self {
        Self {
            postings_enum: 0,
            slice,
        }
    }
}
impl Default for EnumWithSlice {
    fn default() -> Self {
        Self::new()
    }
}
impl Display for EnumWithSlice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.slice)
    }
}

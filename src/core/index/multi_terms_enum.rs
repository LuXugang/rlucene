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
use crate::core::index::reader_slice::ReaderSlice;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::index::terms_enum_index::TermsEnumIndex;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::priority_queue::{Compare, PriorityQueue};

pub struct MultiTermsEnum;

struct TermsEnumWithSlice<TE>
where
    TE: TermsEnum,
{
    base: TermsEnumIndex<TE>,
    sub_slice: ReaderSlice,
}
impl<TE> TermsEnumWithSlice<TE>
where
    TE: TermsEnum,
{
    pub fn new(index: i32, sub_slice: ReaderSlice) -> Self {
        debug_assert!(sub_slice.length >= 0, "length={}", sub_slice.length);

        Self {
            base: TermsEnumIndex::new(None, index),
            sub_slice,
        }
    }
}

struct TermMergeQueue<TE>
where
    TE: TermsEnum,
{
    stack: Vec<i32>,
    queue: PriorityQueue<TermsEnumWithSlice<TE>, TermMergeQueueCmp>,
}
impl<TE> TermMergeQueue<TE>
where
    TE: TermsEnum,
{
    pub fn new(size: i32) -> Result<Self> {
        let queue = PriorityQueue::new(size, TermMergeQueueCmp)?;
        Ok(Self {
            stack: vec![0; size as usize],
            queue,
        })
    }
    /// Add the top() slice as well as all slices that are positionned on the same term to tops and return how many of them there are.
    pub(crate) fn fill_top(&mut self, _tops: &mut Vec<TermsEnumWithSlice<TE>>) -> Result<i32> {
        todo!()
    }
}
struct TermMergeQueueCmp;
impl<TE> Compare<TermsEnumWithSlice<TE>> for TermMergeQueueCmp
where
    TE: TermsEnum,
{
    fn less_than(
        &self,
        terms_a: &TermsEnumWithSlice<TE>,
        terms_b: &TermsEnumWithSlice<TE>,
    ) -> Result<bool> {
        Ok(terms_a.base.compare_term_to(&terms_b.base)? < 0)
    }
}

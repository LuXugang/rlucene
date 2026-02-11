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
use crate::core::index::impact::Impact;
use crate::core::index::impacts::Impacts;
use crate::core::index::impacts_source::ImpactsSource;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::phrase_positions::PhrasePositions;
use crate::core::search::phrase_queue::PhraseQueueCmp;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::priority_queue::PriorityQueue;
use std::borrow::Cow;
use std::vec;

pub struct SloppyPhraseMatcher {
    slop: i32,
    num_postings: i32,
    /// for advancing min position
    pq: PriorityQueue<PhrasePositions, PhraseQueueCmp>,
    capture_lead_match: bool,

    // impacts_approximation: I,
    /// current largest phrase position
    end: i32,

    lead_position: i32,
    lead_offset: i32,
    lead_end_offset: i32,
    lead_ord: i32,
    /// flag indicating that there are repetitions (as checked in first candidate doc)
    has_rpts: bool,
    checked_rpts: bool,
    has_multi_term_rpts: bool,
    /// in each group are PPs that repeats each other (i.e. same term), sorted by (query) offset
    rpt_groups: Vec<Vec<usize>>,
    /// temporary stack for switching colliding repeating pps
    rpt_stack: Vec<usize>,

    positioned: bool,
    match_length: i32,
}
impl SloppyPhraseMatcher {}

struct ImpactsSourceImpl;
impl ImpactsSource for ImpactsSourceImpl {
    fn advance_shallow(&mut self, _target: i32) -> Result<()> {
        Ok(())
    }

    type Impacts<'a>
        = ImpactsImpl
    where
        Self: 'a;

    fn get_impacts(&self) -> Result<Self::Impacts<'_>> {
        Ok(ImpactsImpl)
    }
}
#[derive(Default)]
struct ImpactsImpl;
impl Impacts for ImpactsImpl {
    fn num_levels(&self) -> i32 {
        1
    }

    fn get_doc_id_upto(&self, _level: i32) -> i32 {
        NO_MORE_DOCS
    }

    fn get_impacts(&'_ mut self, _level: i32) -> Result<Cow<'_, [Impact]>> {
        Ok(Cow::Owned(vec![Impact::new(i32::MAX, 1)]))
    }
}

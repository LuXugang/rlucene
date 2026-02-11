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
use crate::core::index::impact::Impact;
use crate::core::index::impacts::Impacts;
use crate::core::index::impacts_enum::ImpactsEnum;
use crate::core::index::impacts_source::ImpactsSource;
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::search::conjunction_disi::{ConjunctionDISI, DISIEnum};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::dummy::dummy_scorer::DummyScorer;
use crate::core::search::impacts_disi::ImpactsDISI;
use crate::core::search::phrase_matcher::PhraseMatcher;
use crate::core::search::phrase_positions::PhrasePositions;
use crate::core::search::phrase_queue::PhraseQueueCmp;
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::PriorityQueue;
use std::borrow::Cow;
use std::rc::Rc;
use std::vec;

pub type SloopyImpactsDISI<PE, SS> =
    ImpactsDISI<ConjunctionDISI<DummyScorer, PE>, ImpactsSourceImpl, SS>;
pub struct SloppyPhraseMatcher<PE, SS>
where
    PE: PostingsEnum,
    SS: SimScorer,
{
    slop: i32,
    num_postings: i32,
    /// for advancing min position
    pub(crate) pq: PriorityQueue<usize, PhraseQueueCmp>,
    capture_lead_match: bool,

    impacts_approximation: SloopyImpactsDISI<PE, SS>,
    /// current largest phrase position
    end: usize,

    lead_position: i32,
    lead_offset: i32,
    lead_end_offset: i32,
    lead_ord: i32,
    /// flag indicating that there are repetitions (as checked in first candidate doc)
    has_rpts: bool,
    checked_rpts: bool,
    has_multi_term_rpts: bool,
    /// in each group are PPs that repeats each other (i.e. same term), sorted by (query) offset
    rpt_groups: Rc<Vec<Vec<usize>>>,
    /// temporary stack for switching colliding repeating pps
    rpt_stack: Vec<usize>,

    positioned: bool,
    match_length: i32,
}
impl<PE, SS> SloppyPhraseMatcher<PE, SS>
where
    PE: PostingsEnum,
    SS: SimScorer,
{
    /// advance a PhrasePosition and update `end`, return false if exhausted
    fn advance_pp(&mut self, pp_idx: usize) -> Result<bool> {
        if !PhrasePositions::next_position(self, pp_idx)? {
            return Ok(false);
        }
        let pp = &mut self.pq.compare.phrase_positions[pp_idx];
        if pp.position > self.end {
            self.end = pp.position;
        }

        Ok(true)
    }

    /// compare two pps, but only by position and offset
    fn lesser(&self, pp1_idx: usize, pp2_idx: usize) -> usize {
        let pp1 = &self.pq.compare.phrase_positions[pp1_idx];
        let pp2 = &self.pq.compare.phrase_positions[pp2_idx];

        if pp1.position < pp2.position || (pp1.position == pp2.position && pp1.offset < pp2.offset)
        {
            pp1_idx
        } else {
            pp2_idx
        }
    }

    /// index of a pp2 colliding with pp, or -1 if none
    fn collide(&self, pp_idx: usize) -> Result<i32> {
        let tp_pos = self.tp_pos(pp_idx);
        let rpt_group = self.pq.compare.phrase_positions[pp_idx]
            .rpt_group
            .try_convert()?;
        let rg = &self.rpt_groups[rpt_group];

        for &pp2_idx in rg {
            if pp2_idx != pp_idx && self.tp_pos(pp2_idx) == tp_pos {
                let v: i32 = self.pq.compare.phrase_positions[pp2_idx]
                    .rpt_ind
                    .try_convert()?;
                return Ok(v);
            }
        }
        Ok(-1)
    }
    /// with repeats: not so simple.
    fn init_complex(&mut self) -> Result<bool> {
        // System.err.println("initComplex: doc: "+min.doc);
        self.place_first_positions()?;
        if !self.advance_repeat_groups()? {
            return Ok(false); // PPs exhausted
        }
        self.fill_queue()?;
        Ok(true) // PPs available
    }

    /// move all PPs to their first position
    fn place_first_positions(&mut self) -> Result<()> {
        let len = self.pq.compare.phrase_positions.len();
        for pp_idx in 0..len {
            PhrasePositions::first_position(self, pp_idx)?;
        }
        Ok(())
    }
    /// Fill the queue (all pps are already placed
    fn fill_queue(&mut self) -> Result<()> {
        self.pq.clear();
        let len = self.pq.compare.phrase_positions.len();
        // iterate cyclic list: done once handled max
        for idx in 0..len {
            let pos = self.pq.compare.phrase_positions[idx].position;
            if pos > self.end {
                self.end = pos;
            }
            self.pq.add(idx)?;
        }

        Ok(())
    }
    fn advance_repeat_groups(&mut self) -> Result<bool> {
        for rg in self.rpt_groups.clone().as_ref() {
            if self.has_multi_term_rpts {
                // more involved, some may not collide
                let mut i = 0;
                while i < rg.len() {
                    let mut incr: usize = 1;
                    let pp_idx = rg[i];

                    loop {
                        let k = self.collide(pp_idx)?;
                        if k < 0 {
                            break;
                        }
                        let k = k as usize;
                        let pp2_idx = self.lesser(pp_idx, rg[k]);

                        // at initialization always advance pp with higher offset
                        if !self.advance_pp(pp2_idx)? {
                            return Ok(false); // exhausted
                        }

                        if (self.pq.compare.phrase_positions[pp2_idx].rpt_ind) < i {
                            // should not happen?
                            incr = 0;
                            break;
                        }
                    }

                    i += incr;
                }
            } else {
                // simpler, we know exactly how much to advance
                for (j, &pp_idx) in rg.iter().enumerate().skip(1) {
                    for _ in 0..j {
                        if !PhrasePositions::next_position(self, pp_idx)? {
                            return Ok(false); // PPs exhausted
                        }
                    }
                }
            }
        }
        Ok(true) // PPs available
    }

    fn tp_pos(&self, pp_idx: usize) -> usize {
        let pp = &self.pq.compare.phrase_positions[pp_idx];
        pp.position + pp.offset
    }

    #[inline]
    pub(crate) fn posting(&mut self, posting_idx: usize) -> Result<&mut PE> {
        let impacts = self
            .impacts_approximation
            .in_
            .as_mut()
            .ok_or_else(|| LuceneError::illegal_state("impacts approximation missing"))?;
        let disi = &mut impacts.all_disi[posting_idx];
        match disi {
            DISIEnum::DocIdSetIterator(v) => Ok(v),
            DISIEnum::Scorer(_) => Err(LuceneError::illegal_state(
                "unexpected scorer in SloppyPhraseMatcher",
            )),
        }
    }
}

impl<PE, SS> PhraseMatcher for SloppyPhraseMatcher<PE, SS>
where
    PE: PostingsEnum,
    SS: SimScorer,
{
    type Disi = ConjunctionDISI<DummyScorer, PE>;

    fn approximation(&mut self) -> &mut Self::Disi {
        self.impacts_approximation.in_.as_mut().unwrap()
    }

    type ImpactsApproximation = SloopyImpactsDISI<PE, SS>;

    fn impacts_approximation(&mut self) -> &mut Self::ImpactsApproximation {
        &mut self.impacts_approximation
    }

    fn max_freq(&mut self) -> Result<f32> {
        // every term position in each postings list can be at the head of at most
        // one matching phrase, so the maximum possible phrase freq is the sum of
        // the freqs of the postings lists.
        let impacts = self
            .impacts_approximation
            .in_
            .as_mut()
            .ok_or_else(|| LuceneError::illegal_state("impacts approximation missing"))?;
        let mut max_freq = 0f32;
        for phrase_position in &self.pq.compare.phrase_positions {
            let idx = phrase_position.postings_idx;
            let disi = &mut impacts.all_disi[idx];
            let freq = match disi {
                DISIEnum::DocIdSetIterator(v) => v.freq()?,
                DISIEnum::Scorer(_) => {
                    return Err(LuceneError::illegal_state(
                        "unexpected scorer in SloppyPhraseMatcher",
                    ));
                },
            };

            max_freq += freq as f32;
        }
        Ok(max_freq)
    }

    fn reset(&mut self) -> Result<()> {
        todo!()
    }

    fn next_match(&mut self) -> Result<bool> {
        todo!()
    }

    fn sloppy_weight(&self) -> f32 {
        todo!()
    }

    fn start_position(&self) -> i32 {
        todo!()
    }

    fn end_position(&self) -> i32 {
        todo!()
    }

    fn start_offset(&self) -> Result<i32> {
        todo!()
    }

    fn end_offset(&self) -> Result<i32> {
        todo!()
    }

    fn get_match_cost(&self) -> f32 {
        todo!()
    }
}
#[derive(Default)]
pub struct ImpactsSourceImpl;
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

impl PostingsEnum for ImpactsSourceImpl {
    fn freq(&mut self) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn next_position(&mut self) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn start_offset(&self) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn end_offset(&self) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl DocIdSetIterator for ImpactsSourceImpl {
    fn doc_id(&self) -> i32 {
        unreachable!()
    }

    fn next_doc(&mut self) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn slow_advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn cost(&self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl ImpactsEnum for ImpactsSourceImpl {}
#[derive(Default)]
pub struct ImpactsImpl;
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

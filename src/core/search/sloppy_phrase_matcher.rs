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
use crate::core::index::term::Term;
use crate::core::search::conjunction_disi::ConjunctionDISI;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::impacts_disi::ImpactsDISI;
use crate::core::search::max_score_cache::MaxScoreCache;
use crate::core::search::phrase_matcher::PhraseMatcher;
use crate::core::search::phrase_positions::PhrasePositions;
use crate::core::search::phrase_query::PostingsAndFreq;
use crate::core::search::phrase_queue::PhraseQueueCmp;
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::util::TryIntoInt;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::priority_queue::PriorityQueue;
use linked_hash_map::LinkedHashMap;
use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;
use std::vec;

pub type SloopyImpactsDISI<PE, SS> = ImpactsDISI<ConjunctionDISI<PE>, ImpactsSourceImpl, SS>;
/**
 * Find all slop-valid position-combinations (matches) encountered while traversing/hopping the
 * PhrasePositions. <br>
 * The sloppy frequency contribution of a match depends on the distance: <br>
 * highest freq for distance=0 (exact match). <br>
 * freq gets lower as distance gets higher. <br>
 * Example: for query "a b"~2, a document "x a b a y" can be matched twice: once for "a b"
 * (distance=0), and once for "b a" (distance=2). <br>
 * Possibly not all valid combinations are encountered, because for efficiency we always propagate
 * the least PhrasePosition. This allows to base on PriorityQueue and move forward faster. As
 * result, for example, document "a b c b a" would score differently for queries "a b c"~4 and "c b
 * a"~4, although they really are equivalent. Similarly, for doc "a b c b a f g", query "c b"~2
 * would get same score as "g f"~2, although "c b"~2 could be matched twice. We may want to fix this
 * in the future (currently not, for performance reasons).
 */
pub struct SloppyPhraseMatcher<PE, SS>
where
    PE: PostingsEnum,
    SS: SimScorer,
{
    slop: usize,
    num_postings: usize,
    /// for advancing min position
    pub(crate) pq: PriorityQueue<usize, PhraseQueueCmp>,
    capture_lead_match: bool,

    impacts_approximation: SloopyImpactsDISI<PE, SS>,
    /// current largest phrase position
    end: usize,

    lead_position: usize,
    lead_offset: i32,
    lead_end_offset: i32,
    lead_ord: usize,
    /// flag indicating that there are repetitions (as checked in first candidate doc)
    has_rpts: bool,
    checked_rpts: bool,
    has_multi_term_rpts: bool,
    /// in each group are PPs that repeats each other (i.e. same term), sorted by (query) offset
    rpt_groups: Rc<Vec<Vec<usize>>>,
    /// temporary stack for switching colliding repeating pps
    rpt_stack: Vec<usize>,

    positioned: bool,
    match_length: usize,
    match_cost: f32,
}
impl<PE, SS> SloppyPhraseMatcher<PE, SS>
where
    PE: PostingsEnum,
    SS: SimScorer,
{
    pub fn new(
        postings: Vec<PostingsAndFreq<PE>>,
        slop: usize,
        scorer: SS,
        match_cost: f32,
        capture_lead_match: bool,
    ) -> Result<Self> {
        let num_postings = postings.len();

        let mut phrase_positions = Vec::with_capacity(num_postings);
        let mut posting_vec = Vec::with_capacity(num_postings);
        for (i, p) in postings.into_iter().enumerate() {
            posting_vec.push(p.postings);
            phrase_positions.push(PhrasePositions::new(i, p.position, i, p.terms));
        }
        let cmp = PhraseQueueCmp::new(phrase_positions);
        let pq = PriorityQueue::new(num_postings, cmp)?;

        let approximation = ConjunctionDISI::from_disi(posting_vec)?;

        // What would be a good upper bound of the sloppy frequency? A sum of the
        // sub frequencies would be correct, but it is usually so much higher than
        // the actual sloppy frequency that it doesn't help skip irrelevant
        // documents. As a consequence for now, sloppy phrase queries use dummy
        // impacts:
        let impacts_source = ImpactsSourceImpl;
        let max_score_cache = MaxScoreCache::new(impacts_source, scorer);

        let impacts_approximation = ImpactsDISI::new(approximation, max_score_cache, true);

        Ok(Self {
            slop,
            num_postings,
            pq,
            capture_lead_match,
            impacts_approximation,
            end: 0,
            lead_position: 0,
            lead_offset: 0,
            lead_end_offset: 0,
            lead_ord: 0,
            has_rpts: false,
            checked_rpts: false,
            has_multi_term_rpts: false,
            rpt_groups: Rc::new(Vec::new()),
            rpt_stack: vec![],
            positioned: false,
            match_length: 0,
            match_cost,
        })
    }
    fn capture_lead(&mut self, pp_idx: usize) -> Result<()> {
        if !self.capture_lead_match {
            return Ok(());
        }

        let pp = &self.pq.compare.phrase_positions[pp_idx];

        self.lead_ord = pp.ord;
        self.lead_position = pp.position + pp.offset;

        let postings_idx = pp.postings_idx;
        let postings = self.posting_mut(postings_idx);
        let start_offset = postings.start_offset()?;
        let end_offset = postings.end_offset()?;
        self.lead_offset = start_offset;
        self.lead_end_offset = end_offset;
        Ok(())
    }

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

    /// pp was just advanced. If that caused a repeater collision, resolve by advancing the lesser of
    /// the two colliding pps. Note that there can only be one collision, as by the initialization
    /// there were no collisions before pp was advanced.
    fn advance_rpts(&mut self, mut pp_idx: usize) -> Result<bool> {
        if self.pq.compare.phrase_positions[pp_idx].rpt_group < 0 {
            return Ok(true); // not a repeater
        }

        let g = self.pq.compare.phrase_positions[pp_idx]
            .rpt_group
            .try_convert()?;
        let rg = &self.rpt_groups[g].clone();

        // for re-queuing after collisions are resolved
        let mut bits = FixedBitSet::new(rg.len());

        let k0 = self.pq.compare.phrase_positions[pp_idx].rpt_ind;
        let mut k;

        while {
            k = self.collide(pp_idx)?;
            k >= 0
        } {
            let k_usize = k as usize;
            pp_idx = self.lesser(pp_idx, rg[k_usize]); // always advance the lesser of the (only) two colliding pps

            if !self.advance_pp(pp_idx)? {
                return Ok(false); // exhausted
            }
            // careful: mark only those currently in the queue
            if k_usize != k0 {
                FixedBitSet::ensure_capacity(&mut bits, k_usize);
                // mark that pp2 need to be re-queued
                bits.set(k_usize);
            }
        }

        // collisions resolved, now re-queue
        // empty (partially) the queue until seeing all pps advanced for resolving collisions
        let mut n: usize = 0;
        let num_bits = bits.length(); // largest bit we set

        while bits.cardinality() > 0 {
            let pp2_idx = self
                .pq
                .pop()?
                .ok_or_else(|| LuceneError::illegal_state("no phrase positions available"))?;
            self.rpt_stack[n] = pp2_idx;
            n += 1;

            if self.pq.compare.phrase_positions[pp2_idx].rpt_group >= 0 {
                let ind = self.pq.compare.phrase_positions[pp2_idx].rpt_ind;
                if (ind) < num_bits && bits.get(ind)? {
                    bits.clear_with_index(ind);
                }
            }
        }

        // add back to queue
        for i in (0..n).rev() {
            self.pq.add(self.rpt_stack[i])?;
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
    /// Initialize PhrasePositions in place. A one time initialization for this scorer (on first doc
    /// matching all terms):
    ///
    /// - Check if there are repetitions
    /// - If there are, find groups of repetitions.
    ///
    /// Examples:
    ///
    /// 1. no repetitions: **"ho my"~2**
    /// 2. repetitions: **"ho my my"~2**
    /// 3. repetitions: **"my ho my"~2**
    ///
    /// Returns `false` if PPs are exhausted (and so current doc will not be a match).
    fn init_phrase_positions(&mut self) -> Result<bool> {
        self.end = i32::MIN as usize;

        if !self.checked_rpts {
            return self.init_first_time();
        }

        if !self.has_rpts {
            self.init_simple()?;
            return Ok(true); // PPs available
        }

        self.init_complex()
    }

    fn init_simple(&mut self) -> Result<()> {
        // System.err.println("initSimple: doc: "+min.doc);
        self.pq.clear();

        // position pps and build queue from list
        let len = self.pq.compare.phrase_positions.len();
        for pp_idx in 0..len {
            PhrasePositions::first_position(self, pp_idx)?;

            let pos = self.pq.compare.phrase_positions[pp_idx].position;
            if pos > self.end {
                self.end = pos;
            }

            self.pq.add(pp_idx)?;
        }

        Ok(())
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
    /// initialize with checking for repeats. Heavy work, but done only for the first candidate doc.
    ///
    /// If there are repetitions, check if multi-term postings (MTP) are involved.
    ///
    /// Without MTP, once PPs are placed in the first candidate doc, repeats (and groups) are visible.
    /// With MTP, a more complex check is needed, up-front, as there may be "hidden collisions".
    ///
    /// The more complex initialization has two parts:
    /// (1) identification of repetition groups.
    /// (2) advancing repeat groups at the start of the doc.
    ///
    /// For (1), a possible solution is to just create a single repetition group, made of all repeating
    /// pps. But this would slow down the check for collisions, as all pps would need to be checked.
    /// Instead, we compute "connected regions" on the bipartite graph of postings and terms.
    fn init_first_time(&mut self) -> Result<bool> {
        self.checked_rpts = true;
        self.place_first_positions()?;

        let rpt_terms = self.repeating_terms();
        self.has_rpts = !rpt_terms.is_empty();

        if self.has_rpts {
            // needed with repetitions
            self.rpt_stack = vec![0usize; self.num_postings];

            let rgs = self.gather_rpt_groups(&rpt_terms)?;
            self.sort_rpt_groups(rgs);

            if !self.advance_repeat_groups()? {
                return Ok(false); // PPs exhausted
            }
        }

        self.fill_queue()?;
        Ok(true) // PPs available
    }

    /// sort each repetition group by (query) offset. Done only once (at first doc)
    /// and allows to initialize faster for each doc.
    fn sort_rpt_groups(&mut self, rgs: Vec<Vec<usize>>) {
        let mut rpt_groups = Vec::with_capacity(rgs.len());

        for mut rg in rgs {
            // sort by offset
            rg.sort_by_key(|&pp_idx| self.pq.compare.phrase_positions[pp_idx].offset);
            for (j, &pp_idx) in rg.iter().enumerate() {
                // we use this index for efficient re-queuing
                self.pq.compare.phrase_positions[pp_idx].rpt_ind = j;
            }

            rpt_groups.push(rg);
        }
        self.rpt_groups = Rc::new(rpt_groups);
    }

    /// Detect repetition groups. Done once - for first doc
    fn gather_rpt_groups(
        &mut self,
        rpt_terms: &LinkedHashMap<Term, i32>,
    ) -> Result<Vec<Vec<usize>>> {
        let rpp = self.repeating_pps(rpt_terms);
        let mut res: Vec<Vec<usize>> = Vec::new();

        if !self.has_multi_term_rpts {
            // simpler - no multi-terms - can base on positions in first doc
            #[allow(clippy::needless_range_loop)]
            for i in 0..rpp.len() {
                let pp_idx = rpp[i];
                if self.pq.compare.phrase_positions[pp_idx].rpt_group >= 0 {
                    continue; // already marked as a repetition
                }

                let tp_pos = self.tp_pos(pp_idx);

                for j in (i + 1)..rpp.len() {
                    let pp2_idx = rpp[j];

                    if self.pq.compare.phrase_positions[pp2_idx].rpt_group >= 0
                        || self.pq.compare.phrase_positions[pp2_idx].offset
                            == self.pq.compare.phrase_positions[pp_idx].offset
                        || self.tp_pos(pp2_idx) != tp_pos
                    {
                        continue;
                    }

                    // a repetition
                    let mut g = self.pq.compare.phrase_positions[pp_idx].rpt_group;
                    if g < 0 {
                        g = res.len() as i32;
                        self.pq.compare.phrase_positions[pp_idx].rpt_group = g;
                        let mut rl = Vec::with_capacity(2);
                        rl.push(pp_idx);
                        res.push(rl);
                    }

                    self.pq.compare.phrase_positions[pp2_idx].rpt_group = g;
                    res[g as usize].push(pp2_idx);
                }
            }
        } else {
            // more involved - has multi-terms
            let bb = self.pp_terms_bit_sets(&rpp, rpt_terms);
            let mut bb = bb;
            self.union_term_groups(&mut bb);

            let tg = self.term_groups(rpt_terms, bb)?;

            use std::collections::HashSet;

            let mut ids: HashSet<i32> = HashSet::new();
            for &v in tg.values() {
                ids.insert(v);
            }
            let num_distinct_group_ids = ids.len();

            let mut tmp: Vec<HashSet<usize>> = Vec::with_capacity(num_distinct_group_ids);
            for _ in 0..num_distinct_group_ids {
                tmp.push(HashSet::new());
            }

            for &pp_idx in &rpp {
                let mut gset = self.pq.compare.phrase_positions[pp_idx].rpt_group;
                {
                    let pp = &self.pq.compare.phrase_positions[pp_idx];
                    for t in &pp.terms {
                        if rpt_terms.contains_key(t) {
                            let g = *tg.get(t).ok_or_else(|| {
                                LuceneError::illegal_state("missing term group id")
                            })?;
                            tmp[g as usize].insert(pp_idx);
                            debug_assert!(gset == -1 || gset == g);
                            gset = g;
                        }
                    }
                }

                self.pq.compare.phrase_positions[pp_idx].rpt_group = gset;
            }

            for hs in tmp {
                res.push(hs.into_iter().collect());
            }
        }

        Ok(res)
    }

    fn tp_pos(&self, pp_idx: usize) -> usize {
        let pp = &self.pq.compare.phrase_positions[pp_idx];
        pp.position + pp.offset
    }

    /// find repeating terms and assign them ordinal values
    fn repeating_terms(&self) -> LinkedHashMap<Term, i32> {
        let mut tord = LinkedHashMap::new();
        let mut tcnt = HashMap::new();

        for pp in &self.pq.compare.phrase_positions {
            for t in &pp.terms {
                let cnt = tcnt.entry(t.clone()).and_modify(|c| *c += 1).or_insert(1);

                if *cnt == 2 {
                    let ord = tord.len() as i32;
                    tord.insert(t.clone(), ord);
                }
            }
        }

        tord
    }

    /// find repeating pps, and for each, if has multi-terms, update this.has_multi_term_rpts
    fn repeating_pps(&mut self, rpt_terms: &LinkedHashMap<Term, i32>) -> Vec<usize> {
        let mut rp = Vec::new();
        for (pp_idx, pp) in self.pq.compare.phrase_positions.iter().enumerate() {
            for t in &pp.terms {
                if rpt_terms.contains_key(t) {
                    rp.push(pp_idx);
                    if pp.terms.len() > 1 {
                        self.has_multi_term_rpts = true;
                    }
                    break;
                }
            }
        }
        rp
    }
    /// bit-sets - for each repeating pp, for each of its repeating terms,
    /// the term ordinal values is set
    fn pp_terms_bit_sets(
        &self,
        rpp: &[usize],
        tord: &LinkedHashMap<Term, i32>,
    ) -> Vec<FixedBitSet> {
        let mut bb = Vec::with_capacity(rpp.len());

        for &pp_idx in rpp {
            let mut b = FixedBitSet::new(tord.len());

            let pp = &self.pq.compare.phrase_positions[pp_idx];
            for t in &pp.terms {
                if let Some(&ord) = tord.get(t) {
                    b.set(ord as usize);
                }
            }

            bb.push(b);
        }

        bb
    }
    /// union (term group) bit-sets until they are disjoint (O(n^^2)),
    /// and each group have different terms
    fn union_term_groups(&self, bb: &mut Vec<FixedBitSet>) {
        let mut i = 0;
        while i + 1 < bb.len() {
            let mut incr = 1;
            let mut j = i + 1;

            while j < bb.len() {
                if bb[i].intersects(&bb[j]) {
                    let rhs = bb.remove(j);
                    bb[i].or(&rhs);
                    incr = 0;
                } else {
                    j += 1;
                }
            }

            i += incr;
        }
    }

    /// map each term to the single group that contains it
    fn term_groups(
        &self,
        tord: &LinkedHashMap<Term, i32>,
        bb: Vec<FixedBitSet>,
    ) -> Result<HashMap<Term, i32>> {
        let mut tg: HashMap<Term, i32> = HashMap::new();
        let terms: Vec<Term> = tord.keys().cloned().collect();

        for (i, bits) in bb.iter().enumerate() {
            let mut ord = bits.next_set_bit(0);

            while ord != NO_MORE_DOCS as usize {
                tg.insert(terms[ord].clone(), i as i32);

                let next = ord + 1;
                if next >= bits.length() {
                    ord = NO_MORE_DOCS as usize;
                } else {
                    ord = bits.next_set_bit(next);
                }
            }
        }

        Ok(tg)
    }

    #[inline]
    pub(crate) fn posting_mut(&mut self, posting_idx: usize) -> &mut PE {
        debug_assert!(self.impacts_approximation.use_disi);
        let impacts = &mut self.impacts_approximation.in_;
        &mut impacts.all_disi[posting_idx]
    }
    #[inline]
    pub(crate) fn posting(&self, posting_idx: usize) -> &PE {
        debug_assert!(self.impacts_approximation.use_disi);
        let impacts = &self.impacts_approximation.in_;
        &impacts.all_disi[posting_idx]
    }
}

impl<PE, SS> PhraseMatcher for SloppyPhraseMatcher<PE, SS>
where
    PE: PostingsEnum,
    SS: SimScorer,
{
    type Disi = ConjunctionDISI<PE>;

    fn approximation(&mut self) -> &mut Self::Disi {
        debug_assert!(self.impacts_approximation.use_disi);
        &mut self.impacts_approximation.in_
    }

    type ImpactsApproximation = SloopyImpactsDISI<PE, SS>;

    fn impacts_approximation(&mut self) -> &mut Self::ImpactsApproximation {
        &mut self.impacts_approximation
    }

    fn max_freq(&mut self) -> Result<f32> {
        // every term position in each postings list can be at the head of at most
        // one matching phrase, so the maximum possible phrase freq is the sum of
        // the freqs of the postings lists.
        let impacts = &mut self.impacts_approximation.in_;
        let mut max_freq = 0f32;
        for phrase_position in &self.pq.compare.phrase_positions {
            let idx = phrase_position.postings_idx;
            let freq = impacts.all_disi[idx].freq()?;
            max_freq += freq as f32;
        }
        Ok(max_freq)
    }

    fn reset(&mut self) -> Result<()> {
        self.positioned = self.init_phrase_positions()?;
        self.match_length = i32::MAX as usize;
        self.lead_position = i32::MAX as usize;
        Ok(())
    }

    fn next_match(&mut self) -> Result<bool> {
        if !self.positioned {
            return Ok(false);
        }

        let mut pp_idx = match self.pq.pop()? {
            Some(v) => v,
            None => return Err(LuceneError::illegal_state("no phrase positions available")),
        };

        self.capture_lead(pp_idx)?;

        let pp_pos = self.pq.compare.phrase_positions[pp_idx].position;
        let diff = self
            .end
            .checked_sub(pp_pos)
            .ok_or_else(|| LuceneError::illegal_state("end underflow"))?;
        self.match_length = diff;

        let mut next_idx = *self
            .pq
            .top()
            .ok_or_else(|| LuceneError::illegal_state("no phrase positions available"))?;
        let mut next = self.pq.compare.phrase_positions[next_idx].position;

        while self.advance_pp(pp_idx)? {
            if self.has_rpts && !self.advance_rpts(pp_idx)? {
                break; // pps exhausted
            }

            let pp_pos = self.pq.compare.phrase_positions[pp_idx].position;

            // done minimizing current match-length
            if pp_pos > next {
                self.pq.add(pp_idx)?;

                if self.match_length <= self.slop {
                    return Ok(true);
                }

                pp_idx = self
                    .pq
                    .pop()?
                    .ok_or_else(|| LuceneError::illegal_state("no phrase positions available"))?;

                next_idx = *self
                    .pq
                    .top()
                    .ok_or_else(|| LuceneError::illegal_state("no phrase positions available"))?;
                next = self.pq.compare.phrase_positions[next_idx].position;

                let pp_pos = self.pq.compare.phrase_positions[pp_idx].position;
                let diff = self
                    .end
                    .checked_sub(pp_pos)
                    .ok_or_else(|| LuceneError::illegal_state("end underflow"))?;
                self.match_length = diff;
            } else {
                let diff2 = self
                    .end
                    .checked_sub(pp_pos)
                    .ok_or_else(|| LuceneError::illegal_state("end underflow"))?;
                let match_length2 = diff2;
                if match_length2 < self.match_length {
                    self.match_length = match_length2;
                }
            }

            self.capture_lead(pp_idx)?;
        }

        self.positioned = false;
        Ok(self.match_length <= self.slop)
    }

    fn sloppy_weight(&self) -> f32 {
        1.0f32 / (1.0f32 + self.match_length as f32)
    }

    fn start_position(&self) -> i32 {
        // when a match is detected, the top postings is advanced until it has moved
        // beyond its successor, to ensure that the match is of minimal width.  This
        // means that we need to record the lead position before it is advanced.
        // However, the priority queue doesn't guarantee that the top postings is in fact the
        // earliest in the list, so we need to cycle through all terms to check.
        // this is slow, but Matches is slow anyway...
        let mut lead_position = self.lead_position;

        for pp in &self.pq.compare.phrase_positions {
            lead_position = lead_position.min(pp.position + pp.offset);
        }

        lead_position as i32
    }

    fn end_position(&self) -> i32 {
        let mut end_position = self.lead_position;

        for pp in &self.pq.compare.phrase_positions {
            if pp.ord != self.lead_ord {
                end_position = end_position.max(pp.position + pp.offset);
            }
        }

        end_position as i32
    }

    fn start_offset(&self) -> Result<i32> {
        // when a match is detected, the top postings is advanced until it has moved
        // beyond its successor, to ensure that the match is of minimal width.  This
        // means that we need to record the lead offset before it is advanced.
        // However, the priority queue doesn't guarantee that the top postings is in fact the
        // earliest in the list, so we need to cycle through all terms to check
        // this is slow, but Matches is slow anyway...
        let mut lead_offset = self.lead_offset;

        let len = self.pq.compare.phrase_positions.len();
        for idx in 0..len {
            let postings_idx = self.pq.compare.phrase_positions[idx].postings_idx;
            let offset = self.posting(postings_idx).start_offset()?;
            lead_offset = lead_offset.min(offset);
        }

        Ok(lead_offset)
    }

    fn end_offset(&self) -> Result<i32> {
        let mut end_offset = self.lead_end_offset;

        let len = self.pq.compare.phrase_positions.len();
        for idx in 0..len {
            let pp = &self.pq.compare.phrase_positions[idx];
            if pp.ord != self.lead_ord {
                let postings_idx = pp.postings_idx;
                let offset = self.posting(postings_idx).end_offset()?;
                end_offset = end_offset.max(offset);
            }
        }

        Ok(end_offset)
    }

    fn get_match_cost(&self) -> f32 {
        self.match_cost
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

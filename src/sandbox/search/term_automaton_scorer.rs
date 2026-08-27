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
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::run_automaton::RunAutomaton;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::sync::Arc;

pub(crate) struct TermAutomatonScorer<PE, SS, N> {
  // The original EnumAndScorer values used to create this scorer. The matching queues refer to
  // these values by index; direct access is only exposed for explain purposes.
  subs: Vec<EnumAndScorer<PE>>,
  subs_on_doc: Vec<usize>,
  doc_id_queue: DocIdQueue,
  pos_queue: PositionQueue,
  run_automaton: RunAutomaton,

  // We reuse this array to check for matches starting from an initial
  // position; we increase pos_shift every time we move to a new possible
  // start:
  positions: Vec<PosState>,
  pos_shift: i32,

  // This is -1 if wildcard (`None`) terms were not used; otherwise it is the
  // wildcard term ID.
  any_term_id: i32,
  scorer: Arc<SS>,
  norms: Option<N>,

  cost: i64,

  doc_id: i32,
  freq: i32,
}

pub(crate) struct EnumAndScorer<PE> {
  pub(crate) term_id: i32,
  pub(crate) pos_enum: PE,

  // How many positions left in the current document:
  pos_left: i32,

  // Current position
  pos: i32,
}

impl<PE> EnumAndScorer<PE> {
  pub(crate) fn new(term_id: i32, pos_enum: PE) -> Self {
    Self {
      term_id,
      pos_enum,
      pos_left: 0,
      pos: 0,
    }
  }
}

impl<PE, SS, N> TermAutomatonScorer<PE, SS, N>
where
  PE: PostingsEnum,
  SS: SimScorer,
  N: NumericDocValues,
{
  pub(crate) fn new(
    automaton: Automaton,
    subs: Vec<EnumAndScorer<PE>>,
    term_count: usize,
    any_term_id: i32,
    scorer: Arc<SS>,
    norms: Option<N>,
  ) -> Result<Self> {
    // println!("  automaton:\n{}", automaton.to_dot()?);
    let run_automaton = RunAutomaton::new(automaton, term_count)?;
    let mut cost = 0i64;
    for sub in &subs {
      cost = cost.saturating_add(sub.pos_enum.cost()?);
    }
    let subs_on_doc = (0..subs.len()).collect();
    let positions = (0..4).map(|_| PosState::new()).collect();
    Ok(Self {
      subs,
      subs_on_doc,
      doc_id_queue: DocIdQueue::new(),
      pos_queue: PositionQueue::new(),
      run_automaton,
      positions,
      pos_shift: 0,
      any_term_id,
      scorer,
      norms,
      cost,
      doc_id: -1,
      freq: 0,
    })
  }

  /// Pops all enums positioned on the current (minimum) doc.
  fn pop_current_doc(&mut self) -> Result<()> {
    debug_assert!(self.subs_on_doc.is_empty());
    debug_assert!(!self.doc_id_queue.is_empty());
    let first = self.doc_id_queue.pop(&self.subs)?;
    self.doc_id = self.subs[first].pos_enum.doc_id();
    self.subs_on_doc.push(first);
    while let Some(top) = self.doc_id_queue.top() {
      if self.subs[top].pos_enum.doc_id() != self.doc_id {
        break;
      }
      self.subs_on_doc.push(self.doc_id_queue.pop(&self.subs)?);
    }
    Ok(())
  }

  /// Pushes all previously popped enums back into the doc ID queue.
  fn push_current_doc(&mut self) {
    for index in self.subs_on_doc.drain(..) {
      self.doc_id_queue.add(index, &self.subs);
    }
  }

  fn position_sub_on_doc(&mut self, index: usize) -> Result<()> {
    if self.subs[index].pos_enum.doc_id() != NO_MORE_DOCS {
      self.subs[index].pos_left = self.subs[index].pos_enum.freq()? - 1;
      self.subs[index].pos = self.subs[index].pos_enum.next_position()?;
    }
    Ok(())
  }

  fn do_next(&mut self) -> Result<i32> {
    debug_assert!(self.subs_on_doc.is_empty());
    debug_assert!(
      self
        .doc_id_queue
        .top()
        .is_none_or(|top| self.subs[top].pos_enum.doc_id() > self.doc_id)
    );
    loop {
      // println!("  do_next: cycle");
      self.pop_current_doc()?;
      // println!("    doc_id={}", self.doc_id);
      if self.doc_id == NO_MORE_DOCS {
        return Ok(self.doc_id);
      }
      self.count_matches()?;
      if self.freq > 0 {
        return Ok(self.doc_id);
      }
      let current = self.subs_on_doc.clone();
      for index in current {
        self.subs[index].pos_enum.next_doc()?;
        self.position_sub_on_doc(index)?;
      }
      self.push_current_doc();
    }
  }

  fn shift(&mut self, pos: i32) {
    let limit = (pos - self.pos_shift) as usize;
    for position in &mut self.positions[..limit] {
      position.count = 0;
    }
    self.pos_shift = pos;
  }

  #[allow(unused_assignments)] // Preserves the Java control flow when ANY matching peters out.
  fn count_matches(&mut self) -> Result<()> {
    self.freq = 0;
    for &index in &self.subs_on_doc {
      self.pos_queue.add(index, &self.subs);
    }
    // println!("\ncount_matches: {} terms in doc={} any_term_id={}",
    //   self.subs_on_doc.len(), self.doc_id, self.any_term_id);

    let mut last_pos = -1;
    self.pos_shift = -1;

    while !self.pos_queue.is_empty() {
      let index = self.pos_queue.pop(&self.subs)?;

      // This is a graph intersection, and pos is the state this token
      // leaves from. Until index stores posLength (which we could
      // stuff into a payload using a simple TokenFilter), this token
      // always transitions from state=pos to state=pos+1:
      let pos = self.subs[index].pos;

      if self.pos_shift == -1 {
        self.pos_shift = pos;
      }

      let needed = (pos + 1 - self.pos_shift) as usize;
      if needed >= self.positions.len() {
        let new_len = ArrayUtil::oversize(needed + 1, std::mem::size_of::<PosState>())?;
        self.positions.resize_with(new_len, PosState::new);
      }

      // println!("  term_id={} pos={} (count={} last_pos={}) pos_queue.size={} pos_shift={}",
      //   self.subs[index].term_id, pos, self.positions[(pos - self.pos_shift) as usize].count,
      //   last_pos, self.pos_queue.heap.len(), self.pos_shift);

      // Maybe advance ANY matches:
      if last_pos != -1 && self.any_term_id != -1 {
        let start_last_pos = last_pos;
        while last_pos < pos {
          let current_index = (last_pos - self.pos_shift) as usize;
          if self.positions[current_index].count == 0 && last_pos > start_last_pos {
            // Petered out...
            last_pos = pos;
            break;
          }
          // println!("  iter last_pos={} count={}", last_pos,
          //   self.positions[current_index].count);
          let (current, next) = {
            let (left, right) = self.positions.split_at_mut(current_index + 1);
            (&left[current_index], &mut right[0])
          };
          for &state in &current.states[..current.count] {
            let state = self.run_automaton.step(state, self.any_term_id);
            if state != -1 {
              // println!("    add pos={} state={}", last_pos + 1, state);
              next.add(state);
            }
          }
          last_pos += 1;
        }
      }

      let current_index = (pos - self.pos_shift) as usize;
      let next_index = current_index + 1;

      // If there are no pending matches at neither this position or the
      // next position, then it's safe to shift back to positions[0]:
      if self.positions[current_index].count == 0 && self.positions[next_index].count == 0 {
        self.shift(pos);
      }

      let current_index = (pos - self.pos_shift) as usize;
      let (current, next) = {
        let (left, right) = self.positions.split_at_mut(current_index + 1);
        (&left[current_index], &mut right[0])
      };

      // Match current token:
      for &state in &current.states[..current.count] {
        // println!("    check cur state={state}");
        let state = self.run_automaton.step(state, self.subs[index].term_id);
        if state != -1 {
          // println!("      --> {state}");
          next.add(state);
          if self.run_automaton.is_accept(state)? {
            // println!("      *** (1)");
            self.freq += 1;
          }
        }
      }

      // Also consider starting a new match from this position:
      let state = self.run_automaton.step(0, self.subs[index].term_id);
      if state != -1 {
        // println!("  add init state={state}");
        next.add(state);
        if self.run_automaton.is_accept(state)? {
          // println!("      *** (2)");
          self.freq += 1;
        }
      }

      if self.subs[index].pos_left > 0 {
        // Put this sub back into the posQueue:
        self.subs[index].pos = self.subs[index].pos_enum.next_position()?;
        self.subs[index].pos_left -= 1;
        self.pos_queue.add(index, &self.subs);
      }

      last_pos = pos;
    }

    let limit = (last_pos + 1 - self.pos_shift) as usize;
    // reset
    for position in &mut self.positions[..=limit] {
      position.count = 0;
    }
    Ok(())
  }

  pub(crate) fn original_subs_on_doc(&mut self) -> &mut [EnumAndScorer<PE>] {
    &mut self.subs
  }
}

impl<PE, SS, N> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for TermAutomatonScorer<PE, SS, N>
where
  PE: PostingsEnum,
  SS: SimScorer,
  N: NumericDocValues,
{
}
impl<PE, SS, N> DocIdSetIterator for TermAutomatonScorer<PE, SS, N>
where
  PE: PostingsEnum,
  SS: SimScorer,
  N: NumericDocValues,
{
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    // We only need to advance docs that are positioned since all docs in the
    // pq are guaranteed to be beyond the current doc already.
    let current = self.subs_on_doc.clone();
    for index in current {
      self.subs[index].pos_enum.next_doc()?;
      self.position_sub_on_doc(index)?;
    }
    self.push_current_doc();
    self.do_next()
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    // Both positioned docs and docs in the pq might be behind target.

    // 1. Advance the PQ
    while let Some(top) = self.doc_id_queue.top() {
      if self.subs[top].pos_enum.doc_id() >= target {
        break;
      }
      let index = self.doc_id_queue.pop(&self.subs)?;
      self.subs[index].pos_enum.advance(target)?;
      self.position_sub_on_doc(index)?;
      self.doc_id_queue.add(index, &self.subs);
    }

    // 2. Advance subsOnDoc
    let current = self.subs_on_doc.clone();
    for index in current {
      self.subs[index].pos_enum.advance(target)?;
      self.position_sub_on_doc(index)?;
    }
    self.push_current_doc();
    self.do_next()
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.cost)
  }
}

impl<PE, SS, N> Scorable for TermAutomatonScorer<PE, SS, N>
where
  PE: PostingsEnum + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
{
  fn score(&mut self) -> Result<f32> {
    let mut norm = 1i64;
    if let Some(norms) = &mut self.norms
      && norms.advance_exact(self.doc_id)?
    {
      norm = norms.long_value()?;
    }
    Ok(self.scorer.score(self.freq as f32, norm))
  }

  fn cost(&self) -> Result<i64> {
    <Self as DocIdSetIterator>::cost(self)
  }
}

impl<PE, SS, N> FixedScore for TermAutomatonScorer<PE, SS, N> {}

impl<PE, SS, N> Scorer for TermAutomatonScorer<PE, SS, N>
where
  PE: PostingsEnum + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
{
  fn doc_id(&mut self) -> Result<i32> {
    Ok(self.doc_id)
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(self)
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(self)
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    self
  }

  fn get_max_score(&mut self, _upto: i32) -> Result<f32> {
    Ok(self.scorer.score(f32::MAX, 1))
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    self.iterator()
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    self.iterator_mut()
  }
}

/// Sorts by doc ID so we can quickly pull out all scorers that are on the same (lowest) doc ID.
struct DocIdQueue {
  heap: Vec<usize>,
}

impl DocIdQueue {
  fn new() -> Self {
    Self { heap: Vec::new() }
  }

  fn is_empty(&self) -> bool {
    self.heap.is_empty()
  }

  fn top(&self) -> Option<usize> {
    self.heap.first().copied()
  }

  fn add<PE>(&mut self, value: usize, subs: &[EnumAndScorer<PE>])
  where
    PE: PostingsEnum,
  {
    self.heap.push(value);
    let mut index = self.heap.len() - 1;
    while index > 0 {
      let parent = (index - 1) / 2;
      if subs[self.heap[parent]].pos_enum.doc_id() <= subs[self.heap[index]].pos_enum.doc_id() {
        break;
      }
      self.heap.swap(parent, index);
      index = parent;
    }
  }

  fn pop<PE>(&mut self, subs: &[EnumAndScorer<PE>]) -> Result<usize>
  where
    PE: PostingsEnum,
  {
    let last = self
      .heap
      .pop()
      .ok_or_else(|| LuceneError::illegal_state("document ID queue is empty"))?;
    if self.heap.is_empty() {
      return Ok(last);
    }

    let result = std::mem::replace(&mut self.heap[0], last);
    let mut index = 0;
    loop {
      let left = index * 2 + 1;
      if left >= self.heap.len() {
        break;
      }
      let right = left + 1;
      let mut child = left;
      if right < self.heap.len()
        && subs[self.heap[right]].pos_enum.doc_id() < subs[self.heap[left]].pos_enum.doc_id()
      {
        child = right;
      }
      if subs[self.heap[index]].pos_enum.doc_id() <= subs[self.heap[child]].pos_enum.doc_id() {
        break;
      }
      self.heap.swap(index, child);
      index = child;
    }
    Ok(result)
  }
}

/// Sorts by position so we can visit all scorers on one doc, by position.
struct PositionQueue {
  heap: Vec<usize>,
}

impl PositionQueue {
  fn new() -> Self {
    Self { heap: Vec::new() }
  }

  fn is_empty(&self) -> bool {
    self.heap.is_empty()
  }

  fn add<PE>(&mut self, value: usize, subs: &[EnumAndScorer<PE>])
  where
    PE: PostingsEnum,
  {
    self.heap.push(value);
    let mut index = self.heap.len() - 1;
    while index > 0 {
      let parent = (index - 1) / 2;
      if subs[self.heap[parent]].pos <= subs[self.heap[index]].pos {
        break;
      }
      self.heap.swap(parent, index);
      index = parent;
    }
  }

  fn pop<PE>(&mut self, subs: &[EnumAndScorer<PE>]) -> Result<usize>
  where
    PE: PostingsEnum,
  {
    let last = self
      .heap
      .pop()
      .ok_or_else(|| LuceneError::illegal_state("position queue is empty"))?;
    if self.heap.is_empty() {
      return Ok(last);
    }

    let result = std::mem::replace(&mut self.heap[0], last);
    let mut index = 0;
    loop {
      let left = index * 2 + 1;
      if left >= self.heap.len() {
        break;
      }
      let right = left + 1;
      let mut child = left;
      if right < self.heap.len() && subs[self.heap[right]].pos < subs[self.heap[left]].pos {
        child = right;
      }
      if subs[self.heap[index]].pos <= subs[self.heap[child]].pos {
        break;
      }
      self.heap.swap(index, child);
      index = child;
    }
    Ok(result)
  }
}

struct PosState {
  // Which automaton states we are in at this position
  states: Vec<i32>,

  // How many states
  count: usize,
}

impl PosState {
  fn new() -> Self {
    Self {
      states: vec![0; 2],
      count: 0,
    }
  }

  fn add(&mut self, state: i32) {
    if self.states.len() == self.count {
      self.states.push(0);
    }
    self.states[self.count] = state;
    self.count += 1;
  }
}

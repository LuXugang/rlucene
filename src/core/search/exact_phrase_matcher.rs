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
use crate::core::search::conjunction_disi::ConjunctionDISI;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum2};
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::impacts_disi::ImpactsDISI;
use crate::core::search::max_score_cache::MaxScoreCache;
use crate::core::search::phrase_matcher::PhraseMatcher;
use crate::core::search::phrase_query::PostingsAndFreq;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};
use std::borrow::Cow;
pub type ImpactsApproximationType<IE, SS> = ImpactsDISI<DummyDISI, ImpactsSourceImpl<IE>, SS>;
/// Expert: Find exact phrases
pub struct ExactPhraseMatcher<IE, SS>
where
  IE: ImpactsEnum,
  SS: SimScorer,
{
  pub(crate) impacts_approximation: ImpactsApproximationType<IE, SS>,
  match_cost: f32,
  pub(crate) score_mode: ScoreMode,
  postings: Vec<PostingsAndPosition>,
}
impl<IE, SS> ExactPhraseMatcher<IE, SS>
where
  IE: ImpactsEnum,
  SS: SimScorer,
{
  pub fn new(
    postings: Vec<PostingsAndFreq<IE>>,
    score_mode: ScoreMode,
    scorer: SS,
    match_cost: f32,
  ) -> Result<Self> {
    let postings_len = postings.len();
    let mut impacts_enum = Vec::with_capacity(postings_len);
    let mut postings_and_positions = Vec::with_capacity(postings_len);
    for (i, p) in postings.into_iter().enumerate() {
      impacts_enum.push(p.postings);
      postings_and_positions.push(PostingsAndPosition::new(i, p.position))
    }
    let wrapped_impacts_enum = ConjunctionDISI::from_disi(impacts_enum)?;
    let impacts_source = merge_impacts(wrapped_impacts_enum)?;
    let impacts_approximation =
      ImpactsDISI::new(DummyDISI, MaxScoreCache::new(impacts_source, scorer), false);
    Ok(Self {
      impacts_approximation,
      match_cost,
      score_mode,
      postings: postings_and_positions,
    })
  }
  /**
   * Advance the given pos enum to the first position on or after {@code target}. Return {@code
   * false} if the enum was exhausted before reaching {@code target} and {@code true} otherwise.
   */
  fn advance_position(&mut self, idx: usize, target: i32) -> Result<bool> {
    let mut pos = self.postings[idx].pos;
    while pos < target {
      if self.postings[idx].upto == self.postings[idx].freq {
        return Ok(false);
      } else {
        let postings_idx = self.postings[idx].postings_idx;
        let next_pos = self.posting_mut(postings_idx).next_position()?;
        let posting = &mut self.postings[idx];
        posting.pos = next_pos;
        pos = next_pos;
        posting.upto += 1;
      }
    }

    Ok(true)
  }

  #[inline]
  fn posting(&self, idx: usize) -> &IE {
    &self
      .impacts_approximation
      .max_score_cache
      .impacts_source
      .impacts_enums
      .all_disi[idx]
  }
  #[inline]
  fn posting_mut(&mut self, idx: usize) -> &mut IE {
    &mut self
      .impacts_approximation
      .max_score_cache
      .impacts_source
      .impacts_enums
      .all_disi[idx]
  }
  pub(crate) fn approximation_top_scorers_mut(&mut self) -> &mut ImpactsApproximationType<IE, SS> {
    &mut self.impacts_approximation
  }
  pub(crate) fn approximation_top_scorers(&self) -> &ImpactsApproximationType<IE, SS> {
    &self.impacts_approximation
  }
  pub(crate) fn approximation_mut(&mut self) -> &mut ConjunctionDISI<IE> {
    &mut self
      .impacts_approximation
      .max_score_cache
      .impacts_source
      .impacts_enums
  }
  pub(crate) fn approximation(&self) -> &ConjunctionDISI<IE> {
    &self
      .impacts_approximation
      .max_score_cache
      .impacts_source
      .impacts_enums
  }
}
pub type Disi<IE, SS> =
  DocIdSetIteratorEnum2<ImpactsApproximationType<IE, SS>, ConjunctionDISI<IE>>;
impl<IE, SS> PhraseMatcher for ExactPhraseMatcher<IE, SS>
where
  IE: ImpactsEnum,
  SS: SimScorer,
{
  fn max_freq(&mut self) -> Result<f32> {
    let mut min_freq = self.postings[0].freq;

    for i in 1..self.postings.len() {
      min_freq = min_freq.min(self.postings[i].freq);
    }
    Ok(min_freq as f32)
  }

  fn reset(&mut self) -> Result<()> {
    for i in 0..self.postings.len() {
      let postings_idx = self.postings[i].postings_idx;
      let freq = self.posting_mut(postings_idx).freq()?;
      self.postings[i].freq = freq;
      self.postings[i].pos = -1;
      self.postings[i].upto = 0;
    }

    Ok(())
  }

  fn next_match(&mut self) -> Result<bool> {
    if self.postings[0].upto < self.postings[0].freq {
      let postings_idx = self.postings[0].postings_idx;
      self.postings[0].pos = self.posting_mut(postings_idx).next_position()?;
      self.postings[0].upto += 1;
    } else {
      return Ok(false);
    }

    'advance_head: loop {
      let phrase_pos = self.postings[0].pos - self.postings[0].offset as i32;
      for j in 1..self.postings.len() {
        let expected_pos = phrase_pos + self.postings[j].offset as i32;
        // advance up to the same position as the lead
        if !self.advance_position(j, expected_pos)? {
          break 'advance_head;
        }

        if self.postings[j].pos != expected_pos {
          // we advanced too far
          let target =
            self.postings[j].pos - self.postings[j].offset as i32 + self.postings[0].offset as i32;

          if self.advance_position(0, target)? {
            continue 'advance_head;
          } else {
            break 'advance_head;
          }
        }
      }
      return Ok(true);
    }
    Ok(false)
  }

  fn sloppy_weight(&self) -> f32 {
    1f32
  }

  fn start_position(&self) -> i32 {
    self.postings[0].pos
  }

  fn end_position(&self) -> i32 {
    self.postings[self.postings.len() - 1].pos
  }

  fn start_offset(&self) -> Result<i32> {
    let idx = self.postings[0].postings_idx;
    let posting = self.posting(idx);
    posting.start_offset()
  }

  fn end_offset(&self) -> Result<i32> {
    let idx = self.postings[self.postings.len() - 1].postings_idx;
    self.posting(idx).end_offset()
  }

  fn get_match_cost(&self) -> f32 {
    self.match_cost
  }
}
#[cfg(test)]
pub(crate) fn merge_impacts_from_ie<IE>(
  wrapped_impacts_enums: Vec<IE>,
) -> Result<ImpactsSourceImpl<IE>>
where
  IE: ImpactsEnum,
{
  merge_impacts(ConjunctionDISI::from_disi(wrapped_impacts_enums)?)
}

/// Merge impacts for multiple terms of an exact phrase.
pub(crate) fn merge_impacts<IE>(
  wrapped_impacts_enums: ConjunctionDISI<IE>,
) -> Result<ImpactsSourceImpl<IE>>
where
  IE: ImpactsEnum,
{
  // Iteration of block boundaries uses the impacts enum with the lower cost.
  // This is consistent with BlockMaxConjunctionScorer.
  let impacts_enums = &wrapped_impacts_enums.all_disi;
  let mut tmp_lead_index: i32 = -1;
  for i in 0..impacts_enums.len() {
    if tmp_lead_index == -1
      || impacts_enums[i].cost()? < impacts_enums[tmp_lead_index as usize].cost()?
    {
      tmp_lead_index = i as i32;
    }
  }
  let lead_index: usize = tmp_lead_index.try_convert()?;
  Ok(ImpactsSourceImpl::new(wrapped_impacts_enums, lead_index))
}

pub struct ImpactsSourceImpl<IE>
where
  IE: ImpactsEnum,
{
  pub(crate) impacts_enums: ConjunctionDISI<IE>,
  lead_index: usize,
}
impl<IE> ImpactsSourceImpl<IE>
where
  IE: ImpactsEnum,
{
  pub(crate) fn new(impacts_enums: ConjunctionDISI<IE>, lead_index: usize) -> Self {
    Self {
      impacts_enums,
      lead_index,
    }
  }
}

impl<IE> ImpactsSource for ImpactsSourceImpl<IE>
where
  IE: ImpactsEnum,
{
  fn advance_shallow(&mut self, target: i32) -> Result<()> {
    for impacts_enum in self.impacts_enums.all_disi.iter_mut() {
      impacts_enum.advance_shallow(target)?;
    }
    Ok(())
  }

  type Impacts<'a>
    = ImpactsImpl<IE::Impacts<'a>>
  where
    Self: 'a;

  fn get_impacts(&self) -> Result<Self::Impacts<'_>> {
    let mut impacts = Vec::with_capacity(self.impacts_enums.all_disi.len());
    for v in self.impacts_enums.all_disi.iter() {
      impacts.push(v.get_impacts()?);
    }
    Ok(ImpactsImpl::new(impacts, self.lead_index))
  }
}

impl<IE> PostingsEnum for ImpactsSourceImpl<IE>
where
  IE: ImpactsEnum,
{
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

impl<IE> DocIdSetIterator for ImpactsSourceImpl<IE>
where
  IE: ImpactsEnum,
{
  fn doc_id(&self) -> i32 {
    self.impacts_enums.doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.impacts_enums.next_doc()
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.impacts_enums.advance(target)
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.impacts_enums.slow_advance(target)
  }

  fn cost(&self) -> Result<i64> {
    self.impacts_enums.cost()
  }
}

impl<IE> ImpactsEnum for ImpactsSourceImpl<IE> where IE: ImpactsEnum {}
pub struct ImpactsImpl<I>
where
  I: Impacts,
{
  impacts: Vec<I>,
  lead_index: usize,
}

impl<I> ImpactsImpl<I>
where
  I: Impacts,
{
  fn new(impacts: Vec<I>, lead_index: usize) -> Self {
    Self {
      impacts,
      lead_index,
    }
  }
}
fn get_level<I>(impacts: &I, doc_id_up_to: i32) -> i32
where
  I: Impacts,
{
  let num_levels = impacts.num_levels();
  for level in 0..num_levels {
    if impacts.get_doc_id_upto(level) >= doc_id_up_to {
      return level;
    }
  }
  -1
}

impl<I> Impacts for ImpactsImpl<I>
where
  I: Impacts,
{
  fn num_levels(&self) -> i32 {
    // Delegate to the lead
    self.impacts[self.lead_index].num_levels()
  }

  fn get_doc_id_upto(&self, level: i32) -> i32 {
    // Delegate to the lead
    self.impacts[self.lead_index].get_doc_id_upto(level)
  }

  fn get_impacts(&'_ self, level: i32) -> Result<Vec<Impact>> {
    let doc_id_up_to = self.get_doc_id_upto(level);
    let impact_len = self.impacts.len();
    let mut sub_iterators = Vec::new();
    let mut has_impacts = false;
    let mut only_impact_list = false;
    let mut pq = PriorityQueue::new(impact_len, SubIteratorCmp)?;

    for i in 0..impact_len {
      let impacts_level = get_level(&self.impacts[i], doc_id_up_to);
      if impacts_level == -1 {
        // This instance doesn't have useful impacts, ignore it: this is safe.
        continue;
      }

      let impact_list = self.impacts[i].get_impacts(impacts_level)?;
      let first = impact_list
        .first()
        .ok_or_else(|| LuceneError::illegal_state("impact is None"))?;

      if first.freq == i32::MAX && first.norm == 1 {
        // Dummy impacts, ignore it too.
        continue;
      }

      let sub = SubIterator::new(impact_list);
      sub_iterators.push(sub);

      if !has_impacts {
        has_impacts = true;
        only_impact_list = true;
      } else {
        only_impact_list = false;
      }
    }

    if !has_impacts {
      return Ok(vec![Impact::new(i32::MAX, 1)]);
    } else if only_impact_list {
      if sub_iterators.len() != 1 {
        return Err(LuceneError::illegal_state(
          "only_impact_list is true but there are multiple sub iterators",
        ));
      }
      return match sub_iterators.pop() {
        Some(sub) => Ok(sub.iterator),
        None => Err(LuceneError::illegal_state(
          "should at least one sub iterator",
        )),
      };
    }
    // Idea: merge impacts by freq. The tricky thing is that we need to
    // consider freq values that are not in the impacts too. For
    // instance if the list of impacts is [{freq=2,norm=10}, {freq=4,norm=12}],
    // there might well be a document that has a freq of 2 and a length of 11,
    // which was just not added to the list of impacts because {freq=2,norm=10}
    // is more competitive.
    // We walk impacts in parallel through a PQ ordered by freq. At any time,
    // the competitive impact consists of the lowest freq among all entries of
    // the PQ (the top) and the highest norm (tracked separately).
    pq.add_all(sub_iterators)?;

    let mut merged_impacts: Vec<Impact> = Vec::new();

    let mut current_freq = {
      let top = pq
        .top()
        .ok_or_else(|| LuceneError::illegal_state("top is None"))?;
      top.current()?.freq
    };

    let mut current_norm = 0;
    for it in pq.iter_ref() {
      let norm = it.current()?.norm;
      if norm as u64 > current_norm as u64 {
        current_norm = norm;
      }
    }
    let mut top = pq
      .top_mut()
      .ok_or_else(|| LuceneError::illegal_state("top is None"))?;

    'outer: loop {
      if let Some(last) = merged_impacts.last_mut() {
        if last.norm == current_norm {
          last.freq = current_freq;
        } else {
          merged_impacts.push(Impact::new(current_freq, current_norm));
        }
      } else {
        merged_impacts.push(Impact::new(current_freq, current_norm));
      }

      loop {
        if !top.next() {
          // At least one clause doesn't have any more documents below the current norm,
          // so we can safely ignore further clauses. The only reason why they have more
          // impacts is because they cover more documents that we are not interested in.
          break 'outer;
        }

        let top_current = top.current()?;
        if top_current.norm as u64 > current_norm as u64 {
          current_norm = top_current.norm;
        }
        top = pq.update_top()?;

        let impact = top.current()?;
        if impact.freq != current_freq {
          break;
        }
      }

      current_freq = top.current()?.freq;
    }

    Ok(merged_impacts)
  }
}

struct SubIterator {
  iterator: Vec<Impact>,
  current: Option<usize>,
}
impl SubIterator {
  fn new(impacts: Vec<Impact>) -> Self {
    let current = if impacts.is_empty() { None } else { Some(0) };

    Self {
      iterator: impacts,
      current,
    }
  }
  fn next(&mut self) -> bool {
    match self.current {
      None => false,
      Some(idx) => {
        let next_idx = idx + 1;
        if next_idx >= self.iterator.len() {
          self.current = None;
          false
        } else {
          self.current = Some(next_idx);
          true
        }
      },
    }
  }
  fn current(&self) -> Result<&Impact> {
    match self.current {
      Some(idx) => Ok(&self.iterator[idx]),
      None => Err(LuceneError::illegal_state("current is None")),
    }
  }
}
struct SubIteratorCmp;
impl Compare<SubIterator> for SubIteratorCmp {
  fn less_than(&self, a: &SubIterator, b: &SubIterator) -> Result<bool> {
    match (a.current, b.current) {
      (Some(i1), Some(i2)) => Ok(a.iterator[i1].freq < b.iterator[i2].freq),
      _ => Err(LuceneError::illegal_state(
        "one of the iterators is exhausted",
      )),
    }
  }
}
struct PostingsAndPosition {
  postings_idx: usize,
  offset: usize,
  freq: i32,
  upto: i32,
  pos: i32,
}

impl PostingsAndPosition {
  fn new(postings_idx: usize, offset: usize) -> Self {
    Self {
      postings_idx,
      offset,
      freq: 0,
      upto: 0,
      pos: 0,
    }
  }
}

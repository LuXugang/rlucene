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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::scorer_util::ScorerUtil;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::search::vector_scorer::VectorScorer;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::collection_util::CollectionUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{Comparator, ToInt, TryIntoInt};

pub struct ConjunctionDISI<D>
where
  D: DocIdSetIterator,
{
  lead1: usize,
  lead2: usize,
  others: Vec<usize>,
  pub(crate) all_disi: Vec<D>,
}
impl<S> ConjunctionDISI<ScorerDisi<S>>
where
  S: Scorer,
{
  pub(crate) fn from_scorer(scorers: Vec<S>) -> Result<Self> {
    let mut iters = Vec::with_capacity(scorers.len());
    for x in scorers.into_iter() {
      iters.push(ScorerDisi::new(x));
    }
    Self::new(iters)
  }
}
impl<D> ConjunctionDISI<D>
where
  D: DocIdSetIterator,
{
  pub(crate) fn from_disi(iterators: Vec<D>) -> Result<Self> {
    let mut iters = Vec::with_capacity(iterators.len());
    for x in iterators.into_iter() {
      iters.push(x);
    }
    Self::new(iters)
  }
}

impl<D> ConjunctionDISI<D>
where
  D: DocIdSetIterator,
{
  fn new(iterators: Vec<D>) -> Result<Self> {
    debug_assert!(iterators.len() >= 2);
    let mut cost = Vec::with_capacity(iterators.len());
    for idx in 0..iterators.len() {
      cost.push(idx);
    }
    let cmp = DisiCmp::new(iterators.as_ref());
    CollectionUtil::tim_sort_with_comparator(&mut cost, cmp)?;
    let mut iters = Vec::with_capacity(iterators.len());
    for idx in cost {
      iters.push(idx);
    }
    let lead1 = iters.remove(0);
    let lead2 = iters.remove(0);
    let v = Self {
      lead1,
      lead2,
      others: iters,
      all_disi: iterators,
    };
    Ok(v)
  }
  fn do_next(&mut self, mut doc: i32) -> Result<i32> {
    'advance_head: loop {
      debug_assert_eq!(doc, self.all_disi[self.lead1].doc_id());
      // find agreement between the two iterators with the lower costs
      // we special case them because they do not need the
      // 'other.docID() < doc' check that the 'others' iterators need
      let next2 = self.all_disi[self.lead2].advance(doc)?;
      if next2 != doc {
        doc = self.all_disi[self.lead1].advance(next2)?;
        if doc != next2 {
          continue;
        }
      }
      // then find agreement with other iterators
      for other_idx in self.others.iter() {
        let other_doc = self.all_disi[*other_idx].doc_id();

        // other.doc may already be equal to doc if we "continued advanceHead"
        // on the previous iteration and the advance on the lead scorer exactly matched.
        if other_doc < doc {
          let next = self.all_disi[*other_idx].advance(doc)?;

          if next > doc {
            // iterator beyond the current doc - advance lead and continue to the new highest doc.
            doc = self.all_disi[self.lead1].advance(next)?;
            continue 'advance_head;
          }
        }
      }
      return Ok(doc);
    }
  }
  // Returns {@code true} if all sub-iterators are on the same doc ID, {@code false} otherwise
  fn assert_iters_on_same_doc(&self) -> bool {
    let cur_doc = self.all_disi[self.lead1].doc_id();
    let mut iterators_on_the_same_doc = self.all_disi[self.lead2].doc_id() == cur_doc;
    let mut i = 0;
    while i < self.others.len() && iterators_on_the_same_doc {
      iterators_on_the_same_doc =
        iterators_on_the_same_doc && (self.all_disi[self.others[i]].doc_id() == cur_doc);
      i += 1;
    }
    iterators_on_the_same_doc
  }
}
impl<D> DocIdSetIterator for ConjunctionDISI<D>
where
  D: DocIdSetIterator,
{
  fn doc_id(&self) -> i32 {
    self.all_disi[self.lead1].doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    debug_assert!(
      self.assert_iters_on_same_doc(),
      "Sub-iterators of ConjunctionDISI are not on the same document!"
    );
    let doc = self.all_disi[self.lead1].next_doc()?;
    self.do_next(doc)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    debug_assert!(
      self.assert_iters_on_same_doc(),
      "Sub-iterators of ConjunctionDISI are not on the same document!"
    );
    let doc = self.all_disi[self.lead1].advance(target)?;
    self.do_next(doc)
  }

  fn cost(&self) -> Result<i64> {
    self.all_disi[self.lead1].cost()
  }
}
struct DisiCmp<'a, D>
where
  D: DocIdSetIterator,
{
  disi: &'a [D],
}
impl<'a, D> DisiCmp<'a, D>
where
  D: DocIdSetIterator,
{
  fn new(disi: &'a [D]) -> Self {
    DisiCmp { disi }
  }
}
impl<D> Comparator<usize> for DisiCmp<'_, D>
where
  D: DocIdSetIterator,
{
  const TYPE: &'static str = "DisiCmp";

  fn compare(&self, a: &usize, b: &usize) -> Result<i32> {
    let lf = self.disi[*a].cost()?;
    let rg = self.disi[*b].cost()?;
    Ok(lf.cmp(&rg).to_int())
  }
}
/// Conjunction between a [`DocIdSetIterator`] and one or more BitSetIterators.
pub struct BitSetConjunctionDISI<DISI, T>
where
  DISI: DocIdSetIterator,
  T: BitSet,
{
  lead: DISI,
  bit_set_iterators: Vec<BitSetIterator<T>>,
  min_length: usize,
}
impl<DISI, T> BitSetConjunctionDISI<DISI, T>
where
  DISI: DocIdSetIterator,
  T: BitSet,
{
  pub fn new(lead: DISI, bit_set_iterators: Vec<BitSetIterator<T>>) -> Result<Self> {
    debug_assert!(!bit_set_iterators.is_empty());
    let mut temp_bit_set_iterators = Vec::with_capacity(bit_set_iterators.len());
    let mut cost = Vec::with_capacity(bit_set_iterators.len());
    for (idx, v) in bit_set_iterators.into_iter().enumerate() {
      cost.push(idx);
      temp_bit_set_iterators.push(Some(v));
    }
    let cmp = BitSetIteratorCmp::new(temp_bit_set_iterators.as_ref());
    ArrayUtil::tim_sort_with_comparator(&mut cost, cmp)?;

    let bit_set_iterators = cost
      .into_iter()
      .map(|idx| temp_bit_set_iterators[idx].take().unwrap())
      .collect::<Vec<_>>();
    let mut min_length = i32::MAX;
    for iter in &bit_set_iterators {
      let bit_set = iter.get_bit_set();
      min_length = min_length.min(bit_set.length() as i32);
    }

    Ok(Self {
      lead,
      bit_set_iterators,
      min_length: min_length.try_convert()?,
    })
  }
  fn do_next(&mut self, mut doc: i32) -> Result<i32> {
    'advance_lead: loop {
      if doc >= self.min_length as i32 {
        if doc != NO_MORE_DOCS {
          self.lead.advance(NO_MORE_DOCS)?;
        }
        return Ok(NO_MORE_DOCS);
      }

      for bs_iter in &self.bit_set_iterators {
        let bs = bs_iter.get_bit_set();
        if !bs.get(doc as usize)? {
          doc = self.lead.next_doc()?;
          continue 'advance_lead;
        }
      }

      for iter in &mut self.bit_set_iterators {
        iter.set_doc_id(doc);
      }

      return Ok(doc);
    }
  }
  fn assert_iters_on_same_doc(&self) -> bool {
    let cur_doc = self.lead.doc_id();
    for iter in &self.bit_set_iterators {
      if iter.doc_id() != cur_doc {
        return false;
      }
    }
    true
  }
}
impl<DISI, T> DocIdSetIterator for BitSetConjunctionDISI<DISI, T>
where
  DISI: DocIdSetIterator,
  T: BitSet,
{
  fn doc_id(&self) -> i32 {
    self.lead.doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    debug_assert!(
      self.assert_iters_on_same_doc(),
      "Sub-iterators of BitSetConjunctionDISI are not on the same document!"
    );
    let doc = self.lead.next_doc()?;
    self.do_next(doc)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    debug_assert!(
      self.assert_iters_on_same_doc(),
      "Sub-iterators of BitSetConjunctionDISI are not on the same document!"
    );
    let doc = self.lead.advance(target)?;
    self.do_next(doc)
  }

  fn cost(&self) -> Result<i64> {
    self.lead.cost()
  }
}
struct BitSetIteratorCmp<'a, B>
where
  B: BitSet,
{
  disi: &'a [Option<BitSetIterator<B>>],
}
impl<'a, B> BitSetIteratorCmp<'a, B>
where
  B: BitSet,
{
  fn new(disi: &'a [Option<BitSetIterator<B>>]) -> Self {
    BitSetIteratorCmp { disi }
  }
}
impl<B> Comparator<usize> for BitSetIteratorCmp<'_, B>
where
  B: BitSet,
{
  const TYPE: &'static str = "BitSetIteratorCmp";

  fn compare(&self, a: &usize, b: &usize) -> Result<i32> {
    Ok(
      self.disi[*a]
        .as_ref()
        .unwrap()
        .cost()?
        .cmp(&self.disi[*b].as_ref().unwrap().cost()?)
        .to_int(),
    )
  }
}
/// [`TwoPhaseIterator`] implementing a conjunction.
pub struct ConjunctionTwoPhaseIterator<S>
where
  S: Scorer,
{
  two_phase_iterator_idx: Vec<usize>,
  pub(crate) approximation: ConjunctionDISI<ScorerDisi<S>>,
  match_cost: f32,
}
impl<S> ConjunctionTwoPhaseIterator<S>
where
  S: Scorer,
{
  pub(crate) fn new(mut approximation: ConjunctionDISI<ScorerDisi<S>>) -> Result<Self> {
    debug_assert!(
      {
        let mut has_tpi = false;
        for x in approximation.all_disi.iter() {
          if x.two_phase_iterator().is_some() {
            has_tpi = true;
            break;
          }
        }
        has_tpi
      },
      "ConjunctionTwoPhaseIterator requires at least one TwoPhaseIterator"
    );
    let (two_phase_iterator_idx, total_match_cost) = {
      let mut tpis = Vec::with_capacity(approximation.all_disi.len());
      let mut two_phase_iterator_idx = Vec::with_capacity(tpis.len());
      for (idx, x) in approximation.all_disi.iter_mut().enumerate() {
        if let Some(tpi) = x.two_phase_iterator_mut() {
          two_phase_iterator_idx.push(idx);
          tpis.push(Some(tpi));
        } else {
          tpis.push(None);
        }
      }
      let mut total_match_cost = 0.0;
      for x in tpis.iter_mut().flatten() {
        total_match_cost += x.match_cost();
      }
      let cmp = TwoPhaseIteratorCmp::new(tpis.as_mut());
      ArrayUtil::tim_sort_with_comparator(&mut two_phase_iterator_idx, cmp)?;
      (two_phase_iterator_idx, total_match_cost)
    };

    Ok(ConjunctionTwoPhaseIterator {
      two_phase_iterator_idx,
      approximation,
      match_cost: total_match_cost,
    })
  }
}
impl<S> TwoPhaseIterator for ConjunctionTwoPhaseIterator<S>
where
  S: Scorer,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.approximation)
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.approximation)
  }

  fn matches(&mut self) -> Result<bool> {
    for idx in self.two_phase_iterator_idx.iter() {
      match self.approximation.all_disi[*idx].two_phase_iterator_mut() {
        Some(ref mut tpi) => {
          if !tpi.matches()? {
            return Ok(false);
          }
        },
        None => return Err(LuceneError::illegal_state("TwoPhaseIterator is None")),
      }
    }
    Ok(true)
  }

  fn match_cost(&self) -> f32 {
    self.match_cost
  }
}
struct TwoPhaseIteratorCmp<'a, T>
where
  T: TwoPhaseIterator,
{
  tpis: &'a [Option<T>],
}
impl<'a, T> TwoPhaseIteratorCmp<'a, T>
where
  T: TwoPhaseIterator,
{
  fn new(tpis: &'a [Option<T>]) -> Self {
    TwoPhaseIteratorCmp { tpis }
  }
}
impl<T> Comparator<usize> for TwoPhaseIteratorCmp<'_, T>
where
  T: TwoPhaseIterator,
{
  const TYPE: &'static str = "TwoPhaseIteratorCmp";

  fn compare(&self, a: &usize, b: &usize) -> Result<i32> {
    let l = self.tpis[*a]
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("tpi is None"))?;
    let r = self.tpis[*b]
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("tpi is None"))?;
    let ord = l
      .match_cost()
      .partial_cmp(&r.match_cost())
      .ok_or_else(|| LuceneError::illegal_state("can compare float?, match_cost is NaN"))?;
    Ok(ord.to_int())
  }
}

pub struct VectorScorerDisi<V>
where
  V: VectorScorer,
{
  vector_scorer: V,
}
impl<V> VectorScorerDisi<V>
where
  V: VectorScorer,
{
  pub fn new(vector_scorer: V) -> Self {
    Self { vector_scorer }
  }
}
impl<V> DocIdSetIterator for VectorScorerDisi<V>
where
  V: VectorScorer,
{
  fn doc_id(&self) -> i32 {
    self.vector_scorer.iterator().doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.vector_scorer.iterator_mut().next_doc()
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.vector_scorer.iterator_mut().advance(target)
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.vector_scorer.iterator_mut().slow_advance(target)
  }

  fn cost(&self) -> Result<i64> {
    self.vector_scorer.iterator().cost()
  }
}
impl<V> VectorScorer for VectorScorerDisi<V>
where
  V: VectorScorer,
{
  fn score(&mut self) -> Result<f32> {
    self.vector_scorer.score()
  }

  type DocIdSetIterator = V::DocIdSetIterator;

  fn iterator(&self) -> &Self::DocIdSetIterator {
    self.vector_scorer.iterator()
  }

  fn iterator_mut(&mut self) -> &mut Self::DocIdSetIterator {
    self.vector_scorer.iterator_mut()
  }
}

pub struct ScorerDisi<S>
where
  S: Scorer,
{
  scorer: S,
}
impl<S> ScorerDisi<S>
where
  S: Scorer,
{
  pub fn new(scorer: S) -> Self {
    Self { scorer }
  }
}
impl<S> DocIdSetIterator for ScorerDisi<S>
where
  S: Scorer,
{
  fn doc_id(&self) -> i32 {
    ScorerUtil::doc_id(&self.scorer)
  }

  fn next_doc(&mut self) -> Result<i32> {
    ScorerUtil::next_doc(&mut self.scorer)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    ScorerUtil::advance(&mut self.scorer, target)
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    ScorerUtil::slow_advance(&mut self.scorer, target)
  }

  fn cost(&self) -> Result<i64> {
    ScorerUtil::cost(&self.scorer)
  }
}

impl<S> Scorable for ScorerDisi<S>
where
  S: Scorer,
{
  fn score(&mut self) -> Result<f32> {
    self.scorer.score()
  }

  fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
    self.scorer.smoothing_score(doc_id)
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    self.scorer.set_min_competitive_score(min_score)
  }

  fn get_children(&self) -> Result<Vec<ChildScorable<Box<dyn Scorable>>>> {
    self.scorer.get_children()
  }

  fn cost(&self) -> Result<i64> {
    self.scorer.cost()
  }
}

impl<S> crate::core::search::scorable::FixedScore for ScorerDisi<S> where S: Scorer {}

impl<S> Scorer for ScorerDisi<S>
where
  S: Scorer,
{
  fn doc_id(&mut self) -> Result<i32> {
    self.scorer.doc_id()
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    self.scorer.iterator()
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    self.scorer.iterator_mut()
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let ScorerDisi { scorer } = *self;
    Box::new(scorer).take_iterator()
  }

  fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    self.scorer.two_phase_iterator()
  }

  fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    self.scorer.two_phase_iterator_mut()
  }

  fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>> {
    let ScorerDisi { scorer } = *self;
    Box::new(scorer).take_two_phase_iterator()
  }

  fn advance_shallow(&mut self, _target: i32) -> Result<i32> {
    self.scorer.advance_shallow(_target)
  }

  fn default_advance_shallow(&mut self, _target: i32) -> Result<i32> {
    self.scorer.default_advance_shallow(_target)
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    self.scorer.get_max_score(upto)
  }

  fn default_cost(&mut self) -> Result<i64> {
    self.scorer.default_cost()
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    self.scorer.has_two_phase_iterator()
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    self.scorer.approximation()
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    self.scorer.approximation_mut()
  }
}

#[cfg(test)]
mod tests {
  use crate::core::search::conjunction_disi::{ConjunctionDISI, ConjunctionTwoPhaseIterator};
  use crate::core::search::conjunction_scorer::ConjunctionScorerDisi;
  use rand::{Rng, RngExt};
  use std::sync::Arc;

  use crate::core::search::constant_score_scorer::ConstantScoreScorer;
  use crate::core::search::doc_id_set::DocIdSet;
  use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
  use crate::core::search::doc_id_set_iterator::{
    DocIdSetIterator, DocIdSetIteratorEnum2, RangeDISI,
  };

  use crate::core::search::scorable::{FixedScore, Scorable};
  use crate::core::search::score_mode::ScoreMode;
  use crate::core::search::scorer::{Scorer, ScorerEnum2, ScorerEnum3, TwoPhaseState};
  use crate::core::search::two_phase_iterator::{
    TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
  };
  use crate::core::util::bit_doc_id_set::BitDocIdSet;
  use crate::core::util::bit_set::BitSet;
  use crate::core::util::bits::Bits;
  use crate::core::util::error::lucene_error::Result;
  use crate::core::util::fixed_bit_set::FixedBitSet;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{at_least, random};
  use crate::test::core::util::test_util::TestUtil;

  #[allow(dead_code)] // for quick search
  struct TestConjunctionDISI;

  fn approximation<D: DocIdSetIterator, R: Rng + ?Sized>(
    random: &mut R,
    iterator: D,
    confirmed: Arc<FixedBitSet>,
  ) -> TwoPhaseIteratorImpl<DocIdSetIteratorEnum2<DocIdSetIteratorImpl<D>, D>> {
    let v = if random.random_bool(0.5) {
      DocIdSetIteratorEnum2::A(anonymize_iterator(iterator))
    } else {
      DocIdSetIteratorEnum2::B(iterator)
    };
    TwoPhaseIteratorImpl::new(v, confirmed)
  }

  /// Return an anonym class so that ConjunctionDISI cannot optimize it like it does eg. for BitSetIterators.
  fn anonymize_iterator<D>(it: D) -> DocIdSetIteratorImpl<D>
  where
    D: DocIdSetIterator,
  {
    DocIdSetIteratorImpl { it }
  }

  fn scorer<TPI>(two_phase_iterator: TPI) -> ScorerImpl<TPI>
  where
    TPI: TwoPhaseIterator,
  {
    ScorerImpl::new(two_phase_iterator)
  }
  fn random_set<R: Rng + ?Sized>(random: &mut R, max_doc: usize) -> FixedBitSet {
    let step = TestUtil::next_usize(random, 1, 10);
    let mut set = FixedBitSet::new(max_doc);

    let mut doc = random.random_range(0..step);
    while doc < max_doc {
      set.set(doc);
      doc += TestUtil::next_usize(random, 1, step);
    }

    set
  }
  fn clear_random_bits<R: Rng + ?Sized>(random: &mut R, other: &FixedBitSet) -> FixedBitSet {
    let mut set = FixedBitSet::new(other.length());

    set.or(other);

    for i in 0..set.length() {
      if random.random_bool(0.5) {
        set.clear_with_index(i);
      }
    }

    set
  }
  fn intersect(bit_sets: &[Arc<FixedBitSet>]) -> FixedBitSet {
    let mut intersection = FixedBitSet::new(bit_sets[0].length());

    intersection.or(&bit_sets[0]);

    for bs in &bit_sets[1..] {
      intersection.and(bs);
    }

    intersection
  }
  #[test]
  fn test_conjunction() -> Result<()> {
    let mut random = random();

    let iters = at_least(&mut random, 100);

    for _ in 0..iters {
      let max_doc = TestUtil::next_usize(&mut random, 100, 10000);
      let num_iterators = TestUtil::next_usize(&mut random, 2, 5);

      let mut sets = Vec::with_capacity(num_iterators);
      let mut iterators = Vec::with_capacity(num_iterators);

      for _ in 0..num_iterators {
        let set = Arc::new(random_set(&mut random, max_doc));

        match random.random_range(0..3) {
          0 => {
            sets.push(set.clone());

            let it = BitDocIdSet::new(Some(set))?.iterator()?;
            let scorer =
              ConstantScoreScorer::from_disi(0f32, ScoreMode::TopScores, anonymize_iterator(it));

            iterators.push(ScorerEnum3::A(scorer));
          },
          1 => {
            // bitset iterator
            sets.push(set.clone());

            let it = BitDocIdSet::new(Some(set))?.iterator()?;
            let scorer = ConstantScoreScorer::from_disi(0f32, ScoreMode::TopScores, it);

            iterators.push(ScorerEnum3::B(scorer));
          },
          _ => {
            // two-phase iterator
            let confirmed = Arc::new(clear_random_bits(&mut random, &set));
            sets.push(confirmed.clone());

            let approx = approximation(
              &mut random,
              BitDocIdSet::new(Some(set))?.iterator()?,
              confirmed,
            );

            let scorer = scorer(approx);
            iterators.push(ScorerEnum3::C(scorer));
          },
        }
      }
      let mut has_tpi = false;
      for x in &iterators {
        if x.has_two_phase_iterator() == TwoPhaseState::Yes {
          has_tpi = true;
          break;
        }
      }
      let conjunction = ConjunctionDISI::from_scorer(iterators)?;
      let mut disi = match has_tpi {
        false => ConjunctionScorerDisi::A(conjunction),
        true => {
          let v =
            TwoPhaseIteratorAsDocIdSetIterator::new(ConjunctionTwoPhaseIterator::new(conjunction)?);
          ConjunctionScorerDisi::B(v)
        },
      };

      let actual = to_bit_set(max_doc, &mut disi)?;
      let expected = intersect(&sets);

      assert_eq!(expected, actual);
    }

    Ok(())
  }
  #[test]
  fn test_conjunction_approximation() -> Result<()> {
    let mut random = random();

    let iters = at_least(&mut random, 100);

    for _ in 0..iters {
      let max_doc = TestUtil::next_usize(&mut random, 100, 10000);
      let num_iterators = TestUtil::next_usize(&mut random, 2, 5);

      let mut sets = Vec::with_capacity(num_iterators);
      let mut iterators = Vec::with_capacity(num_iterators);

      let mut has_approximation = false;

      for _ in 0..num_iterators {
        let set = Arc::new(random_set(&mut random, max_doc));

        if random.random_bool(0.5) {
          // simple iterator
          sets.push(set.clone());

          let it = BitDocIdSet::new(Some(set))?.iterator()?;
          let scorer = ConstantScoreScorer::from_disi(0f32, ScoreMode::CompleteNoScores, it);

          iterators.push(ScorerEnum2::A(scorer));
        } else {
          // scorer with approximation
          let confirmed = Arc::new(clear_random_bits(&mut random, &set));
          sets.push(confirmed.clone());

          let approx = approximation(
            &mut random,
            BitDocIdSet::new(Some(set))?.iterator()?,
            confirmed,
          );

          let scorer = scorer(approx);
          iterators.push(ScorerEnum2::B(scorer));

          has_approximation = true;
        }
      }

      let mut has_tpi = false;
      for x in &iterators {
        if x.has_two_phase_iterator() == TwoPhaseState::Yes {
          has_tpi = true;
          break;
        }
      }
      assert_eq!(has_approximation, has_tpi);
      let v = ConjunctionDISI::from_scorer(iterators)?;
      let mut disi = match has_tpi {
        false => ConjunctionScorerDisi::A(v),
        true => {
          let v = TwoPhaseIteratorAsDocIdSetIterator::new(ConjunctionTwoPhaseIterator::new(v)?);
          ConjunctionScorerDisi::B(v)
        },
      };
      assert_eq!(intersect(&sets), to_bit_set(max_doc, &mut disi)?);
    }

    Ok(())
  }
  #[test]
  fn test_recursive_conjunction_approximation() -> Result<()> {
    // TODO IMPORTANT
    Ok(())
  }
  fn to_bit_set<I>(max_doc: usize, iterator: &mut I) -> Result<FixedBitSet>
  where
    I: DocIdSetIterator,
  {
    let mut set = FixedBitSet::new(max_doc);

    let mut doc = iterator.next_doc()?;
    while doc != NO_MORE_DOCS {
      set.set(doc as usize);
      doc = iterator.next_doc()?;
    }

    Ok(set)
  }
  #[test]
  fn test_collapse_sub_conjunction_disis() -> Result<()> {
    test_collapse_sub_conjunctions_inner(false)
  }

  #[test]
  fn test_collapse_sub_conjunction_scorers() -> Result<()> {
    test_collapse_sub_conjunctions_inner(true)
  }

  fn test_collapse_sub_conjunctions_inner(_wrap_with_scorer: bool) -> Result<()> {
    // TODO
    Ok(())
  }
  #[test]
  fn test_illegal_advancement_of_sub_iterators_trips_assertion() -> Result<()> {
    if !cfg!(debug_assertions) {
      return Ok(());
    }

    let mut random = random();

    let max_doc = 100;
    let num_iterators = TestUtil::next_usize(&mut random, 2, 5);

    let set = Arc::new(random_set(&mut random, max_doc));

    let mut iterators = Vec::with_capacity(num_iterators);
    for _ in 0..num_iterators {
      iterators.push(BitDocIdSet::new(Some(set.clone()))?.iterator()?);
    }
    let len = iterators.len();
    let mut conjunction = ConjunctionDISI::from_disi(iterators)?;

    let idx = TestUtil::next_usize(&mut random, 0, len - 1);
    let rogue = &mut conjunction.all_disi[idx];
    let _ = rogue.next_doc()?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      let _ = conjunction.next_doc();
    }));
    if let Err(err) = result {
      let msg = if let Some(s) = err.downcast_ref::<&str>() {
        *s
      } else if let Some(s) = err.downcast_ref::<String>() {
        s.as_str()
      } else {
        ""
      };
      assert!(msg.contains("Sub-iterators of ConjunctionDISI are not on the same document!"));
    }

    Ok(())
  }
  #[test]
  fn test_bit_set_conjunction_disi_doc_id_on_exhaust() -> Result<()> {
    let mut random = random();

    let num_bitset_iterators = TestUtil::next_usize(&mut random, 2, 5);
    let mut iterators = Vec::with_capacity(num_bitset_iterators + 1);

    let max_bitset_length = 1000;
    let min_bitset_length = 2;
    let lead_max_doc = max_bitset_length + 1;

    iterators.push(DocIdSetIteratorEnum2::A(RangeDISI::new(
      lead_max_doc as i32,
      (lead_max_doc + 1) as i32,
    )?));

    for _ in 0..num_bitset_iterators {
      let bitset_length = TestUtil::next_usize(&mut random, min_bitset_length, max_bitset_length);

      let mut bitset = FixedBitSet::new(bitset_length);
      bitset.set_with_range(0, bitset_length - 1);

      let it = BitDocIdSet::new(Some(Arc::new(bitset)))?.iterator()?;
      iterators.push(DocIdSetIteratorEnum2::B(it));
    }

    let mut conjunction = ConjunctionDISI::from_disi(iterators)?;

    assert_eq!(NO_MORE_DOCS, conjunction.next_doc()?);
    assert_eq!(NO_MORE_DOCS, conjunction.doc_id());

    Ok(())
  }

  struct ScorerImpl<TPI>
  where
    TPI: TwoPhaseIterator,
  {
    tpi_disi: TwoPhaseIteratorAsDocIdSetIterator<TPI>,
  }
  impl<TPI> ScorerImpl<TPI>
  where
    TPI: TwoPhaseIterator,
  {
    fn new(two_phase_iterator: TPI) -> Self {
      Self {
        tpi_disi: TwoPhaseIteratorAsDocIdSetIterator::new(two_phase_iterator),
      }
    }
  }

  impl<TPI> Scorable for ScorerImpl<TPI>
  where
    TPI: TwoPhaseIterator,
  {
    fn score(&mut self) -> Result<f32> {
      Ok(0f32)
    }
  }

  impl<TPI> FixedScore for ScorerImpl<TPI> where TPI: TwoPhaseIterator {}

  impl<TPI> Scorer for ScorerImpl<TPI>
  where
    TPI: TwoPhaseIterator + 'static,
  {
    fn doc_id(&mut self) -> Result<i32> {
      Ok(self.tpi_disi.doc_id())
    }

    fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
      Box::new(&self.tpi_disi)
    }

    fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
      Box::new(&mut self.tpi_disi)
    }

    fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
      let ScorerImpl { tpi_disi, .. } = *self;
      Box::new(tpi_disi)
    }

    fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
      Some(Box::new(&self.tpi_disi.two_phase_iterator))
    }

    fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
      Some(Box::new(&mut self.tpi_disi.two_phase_iterator))
    }

    fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>> {
      let ScorerImpl { tpi_disi, .. } = *self;
      Some(Box::new(tpi_disi.two_phase_iterator))
    }

    fn get_max_score(&mut self, _upto: i32) -> Result<f32> {
      Ok(0f32)
    }

    fn has_two_phase_iterator(&self) -> TwoPhaseState {
      TwoPhaseState::Yes
    }

    fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
      self.tpi_disi.two_phase_iterator.approximation()
    }

    fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
      self.tpi_disi.two_phase_iterator.approximation_mut()
    }
  }

  struct TwoPhaseIteratorImpl<D>
  where
    D: DocIdSetIterator,
  {
    approximation: D,
    confirmed: Arc<FixedBitSet>,
  }
  impl<D> TwoPhaseIteratorImpl<D>
  where
    D: DocIdSetIterator,
  {
    fn new(approximation: D, confirmed: Arc<FixedBitSet>) -> Self {
      Self {
        approximation,
        confirmed,
      }
    }
  }
  impl<D> TwoPhaseIterator for TwoPhaseIteratorImpl<D>
  where
    D: DocIdSetIterator,
  {
    fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
      Box::new(&mut self.approximation)
    }

    fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
      Box::new(&self.approximation)
    }

    fn matches(&mut self) -> Result<bool> {
      self.confirmed.get(self.approximation.doc_id() as usize)
    }

    fn match_cost(&self) -> f32 {
      5f32
    }
  }

  struct DocIdSetIteratorImpl<D>
  where
    D: DocIdSetIterator,
  {
    it: D,
  }
  impl<D> DocIdSetIterator for DocIdSetIteratorImpl<D>
  where
    D: DocIdSetIterator,
  {
    fn doc_id(&self) -> i32 {
      self.it.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
      self.it.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
      self.it.advance(target)
    }

    fn cost(&self) -> Result<i64> {
      self.it.cost()
    }
  }
}

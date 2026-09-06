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
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum2};
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::scorer_util::ScorerUtil;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::search::vector_scorer::VectorScorer;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::collection_util::CollectionUtil;
use crate::core::util::core_helper::CoreHelper;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{Comparator, ToInt, TryIntoInt};

/// The two implementations selected by Java's `createConjunction`.
pub type ConjunctionDISIEnum<D> =
  DocIdSetIteratorEnum2<ConjunctionDISI<D>, BitSetConjunctionDISI<D>>;

pub type ConjunctionDISIWithBitSet<D, B> =
  ConjunctionDISIEnum<DocIdSetIteratorEnum2<D, BitSetIterator<B>>>;

impl<D> ConjunctionDISIEnum<D> {
  pub(crate) fn len(&self) -> usize {
    match self {
      Self::A(conjunction) => conjunction.all_disi.len(),
      Self::B(conjunction) => conjunction.iterator_locations.len(),
    }
  }

  pub(crate) fn iterator_at(&self, index: usize) -> &D {
    match self {
      Self::A(conjunction) => &conjunction.all_disi[index],
      Self::B(conjunction) => match conjunction.iterator_locations[index] {
        IteratorLocation::Lead(index) => match &conjunction.lead {
          DocIdSetIteratorEnum2::A(iterator) => iterator,
          DocIdSetIteratorEnum2::B(lead) => &lead.all_disi[index],
        },
        IteratorLocation::BitSet(index) => &conjunction.bit_set_iterators[index],
      },
    }
  }

  pub(crate) fn iterator_at_mut(&mut self, index: usize) -> &mut D {
    match self {
      Self::A(conjunction) => &mut conjunction.all_disi[index],
      Self::B(conjunction) => match conjunction.iterator_locations[index] {
        IteratorLocation::Lead(index) => match &mut conjunction.lead {
          DocIdSetIteratorEnum2::A(iterator) => iterator,
          DocIdSetIteratorEnum2::B(lead) => &mut lead.all_disi[index],
        },
        IteratorLocation::BitSet(index) => &mut conjunction.bit_set_iterators[index],
      },
    }
  }
}

impl<D, B> ConjunctionDISIWithBitSet<D, B> {
  pub(crate) fn non_bit_set_iterator(&self) -> Result<&D> {
    match self.iterator_at(0) {
      DocIdSetIteratorEnum2::A(iterator) => Ok(iterator),
      DocIdSetIteratorEnum2::B(_) => Err(LuceneError::illegal_state(
        "Expected the non-bit-set iterator first in the conjunction",
      )),
    }
  }
}

pub struct ConjunctionDISI<D> {
  lead1: usize,
  lead2: usize,
  others: Vec<usize>,
  pub(crate) all_disi: Vec<D>,
}
impl<S> ConjunctionDISI<ScorerDisi<S>>
where
  S: Scorer,
{
  pub(crate) fn from_scorer(scorers: Vec<S>) -> Result<ConjunctionDISIEnum<ScorerDisi<S>>> {
    let mut iters = Vec::with_capacity(scorers.len());
    for x in scorers.into_iter() {
      iters.push(ScorerDisi::new(x));
    }
    Self::create_conjunction(iters)
  }
}
impl<D> ConjunctionDISI<D>
where
  D: DocIdSetIterator,
{
  /// Select the conjunction implementation using Java's `createConjunction`
  /// cost rule. Minimum-cost bit sets remain eligible to lead iteration.
  pub(crate) fn create_conjunction(iterators: Vec<D>) -> Result<ConjunctionDISIEnum<D>> {
    debug_assert!(iterators.len() >= 2);
    let cur_doc = iterators[0].doc_id();
    let mut min_cost = i64::MAX;
    for iterator in &iterators {
      if iterator.doc_id() != cur_doc {
        return Err(LuceneError::illegal_argument(
          "Sub-iterators of ConjunctionDISI are not on the same document!",
        ));
      }
      min_cost = min_cost.min(iterator.cost()?);
    }

    let mut ordinary = Vec::with_capacity(iterators.len());
    let mut bit_sets = Vec::new();
    let input_count = iterators.len();
    let mut iterator_locations = Vec::new();
    for iterator in iterators {
      if iterator.is_bit_iter() && iterator.cost()? > min_cost {
        if bit_sets.is_empty() {
          iterator_locations.reserve(input_count);
          iterator_locations.extend((0..ordinary.len()).map(IteratorLocation::Lead));
        }
        iterator_locations.push(IteratorLocation::BitSet(bit_sets.len()));
        bit_sets.push(iterator);
      } else {
        if !bit_sets.is_empty() {
          iterator_locations.push(IteratorLocation::Lead(ordinary.len()));
        }
        ordinary.push(iterator);
      }
    }

    if bit_sets.is_empty() {
      return Ok(ConjunctionDISIEnum::A(Self::new(ordinary)?));
    }
    let lead = if ordinary.len() == 1 {
      DocIdSetIteratorEnum2::A(ordinary.remove(0))
    } else {
      DocIdSetIteratorEnum2::B(Self::new(ordinary)?)
    };
    Ok(ConjunctionDISIEnum::B(BitSetConjunctionDISI::new(
      lead,
      bit_sets,
      iterator_locations,
    )?))
  }
}

impl<D> ConjunctionDISI<D>
where
  D: DocIdSetIterator,
{
  pub(crate) fn new(iterators: Vec<D>) -> Result<Self> {
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
      // `other.doc_id() < doc` check that the remaining iterators need.
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
  // Returns `true` if all sub-iterators are on the same document ID, `false` otherwise.
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
impl<D> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for ConjunctionDISI<D> where
  D: DocIdSetIterator
{
}
impl<D> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for ConjunctionDISI<D> where
  D: DocIdSetIterator
{
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
struct DisiCmp<'a, D> {
  disi: &'a [D],
}
impl<'a, D> DisiCmp<'a, D> {
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
#[derive(Clone, Copy)]
enum IteratorLocation {
  Lead(usize),
  BitSet(usize),
}

/// Conjunction between a [`DocIdSetIterator`] and one or more BitSetIterators.
pub struct BitSetConjunctionDISI<D> {
  lead: DocIdSetIteratorEnum2<D, ConjunctionDISI<D>>,
  bit_set_iterators: Vec<D>,
  min_length: usize,
  iterator_locations: Vec<IteratorLocation>,
}
impl<D> BitSetConjunctionDISI<D>
where
  D: DocIdSetIterator,
{
  fn new(
    lead: DocIdSetIteratorEnum2<D, ConjunctionDISI<D>>,
    bit_set_iterators: Vec<D>,
    mut iterator_locations: Vec<IteratorLocation>,
  ) -> Result<Self> {
    debug_assert!(!bit_set_iterators.is_empty());
    let mut temp_bit_set_iterators = Vec::with_capacity(bit_set_iterators.len());
    let mut cost = Vec::with_capacity(bit_set_iterators.len());
    for (idx, v) in bit_set_iterators.into_iter().enumerate() {
      cost.push(idx);
      temp_bit_set_iterators.push(Some(v));
    }
    let cmp = BitSetIteratorCmp::new(temp_bit_set_iterators.as_ref());
    ArrayUtil::tim_sort_with_comparator(&mut cost, cmp)?;

    let mut bit_set_iterators = Vec::with_capacity(cost.len());
    let mut sorted_indices = vec![0; cost.len()];
    for (sorted_idx, idx) in cost.into_iter().enumerate() {
      sorted_indices[idx] = sorted_idx;
      bit_set_iterators.push(
        temp_bit_set_iterators[idx]
          .take()
          .ok_or_else(|| LuceneError::illegal_state("sorted bit-set iterator is missing"))?,
      );
    }
    for location in &mut iterator_locations {
      if let IteratorLocation::BitSet(index) = location {
        *index = sorted_indices[*index];
      }
    }
    let mut min_length = i32::MAX;
    for iter in &bit_set_iterators {
      min_length = min_length.min(iter.bit_set_length()? as i32);
    }

    Ok(Self {
      lead,
      bit_set_iterators,
      min_length: min_length.try_convert()?,
      iterator_locations,
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
        if !bs_iter.get(doc as usize)? {
          doc = self.lead.next_doc()?;
          continue 'advance_lead;
        }
      }

      for iter in &mut self.bit_set_iterators {
        iter.set_doc_id(doc)?;
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
impl<D> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for BitSetConjunctionDISI<D>
where
  D: DocIdSetIterator,
{
}
impl<D> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for BitSetConjunctionDISI<D> where
  D: DocIdSetIterator
{
}

impl<D> DocIdSetIterator for BitSetConjunctionDISI<D>
where
  D: DocIdSetIterator,
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
struct BitSetIteratorCmp<'a, B> {
  disi: &'a [Option<B>],
}
impl<'a, B> BitSetIteratorCmp<'a, B> {
  fn new(disi: &'a [Option<B>]) -> Self {
    BitSetIteratorCmp { disi }
  }
}
impl<B> Comparator<usize> for BitSetIteratorCmp<'_, B>
where
  B: DocIdSetIterator,
{
  const TYPE: &'static str = "BitSetIteratorCmp";

  fn compare(&self, a: &usize, b: &usize) -> Result<i32> {
    let left = self.disi[*a]
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("left bit-set iterator is missing"))?;
    let right = self.disi[*b]
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("right bit-set iterator is missing"))?;
    Ok(left.cost()?.cmp(&right.cost()?).to_int())
  }
}
/// [`TwoPhaseIterator`] implementing a conjunction.
pub struct ConjunctionTwoPhaseIterator<S> {
  two_phase_iterator_idx: Vec<usize>,
  pub(crate) approximation: ConjunctionDISIEnum<ScorerDisi<S>>,
  match_cost: f32,
}
impl<S> ConjunctionTwoPhaseIterator<S>
where
  S: Scorer,
{
  pub(crate) fn new(approximation: ConjunctionDISIEnum<ScorerDisi<S>>) -> Result<Self> {
    debug_assert!(
      {
        let mut has_tpi = false;
        for idx in 0..approximation.len() {
          let x = approximation.iterator_at(idx);
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
      let mut tpis = Vec::with_capacity(approximation.len());
      let mut two_phase_iterator_idx = Vec::with_capacity(tpis.len());
      for idx in 0..approximation.len() {
        if let Some(tpi) = approximation.iterator_at(idx).two_phase_iterator() {
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
      match self
        .approximation
        .iterator_at_mut(*idx)
        .two_phase_iterator_mut()
      {
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
struct TwoPhaseIteratorCmp<'a, T> {
  tpis: &'a [Option<T>],
}
impl<'a, T> TwoPhaseIteratorCmp<'a, T> {
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
    Ok(CoreHelper::compare_f32(l.match_cost(), r.match_cost()).to_int())
  }
}

pub struct VectorScorerDisi<V> {
  vector_scorer: V,
}
impl<V> VectorScorerDisi<V> {
  pub fn new(vector_scorer: V) -> Self {
    Self { vector_scorer }
  }
}
impl<V> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for VectorScorerDisi<V> where
  V: VectorScorer
{
}
impl<V> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for VectorScorerDisi<V> where
  V: VectorScorer
{
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
  fn score(&self) -> Result<f32> {
    self.vector_scorer.score()
  }

  type DocIdSetIteratorRef<'a>
    = V::DocIdSetIteratorRef<'a>
  where
    Self: 'a;

  fn iterator(&self) -> Self::DocIdSetIteratorRef<'_> {
    self.vector_scorer.iterator()
  }

  type DocIdSetIteratorMut<'a>
    = V::DocIdSetIteratorMut<'a>
  where
    Self: 'a;

  fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_> {
    self.vector_scorer.iterator_mut()
  }
}

pub struct ScorerDisi<S> {
  scorer: S,
}
impl<S> ScorerDisi<S> {
  pub fn new(scorer: S) -> Self {
    Self { scorer }
  }
}
impl<S> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for ScorerDisi<S> where
  S: Scorer
{
}
impl<S> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for ScorerDisi<S>
where
  S: Scorer,
{
  fn is_bit_iter(&self) -> bool {
    self.scorer.approximation().is_bit_iter()
  }
  fn get(&self, index: usize) -> Result<bool> {
    self.scorer.approximation().get(index)
  }
  fn set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.scorer.approximation_mut().set_doc_id(doc)
  }
  fn bit_set_length(&self) -> Result<usize> {
    self.scorer.approximation().bit_set_length()
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

impl<S> crate::core::search::scorable::FixedScore for ScorerDisi<S> {}

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

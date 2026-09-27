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
use crate::core::index::impacts_enum::{ImpactsEnum, ImpactsEnumEnum2};
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::index::slow_impacts_enum::SlowImpactsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::impacts_disi::SourceImpactsDISI;
use crate::core::search::max_score_cache::MaxScoreCache;
use crate::core::search::scorable::Scorable;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::util::error::lucene_error::Result;

/// Expert: A Scorer for documents matching a Term.
pub struct TermScorer<PE, SS, N, IE> {
  norms: Option<N>,
  state: TermScorerState<PE, SS, IE>,
}

enum TermScorerState<PE, SS, IE> {
  ImpactsDisi(SourceImpactsDISI<IE, SS>),
  MaxScoreCache(MaxScoreCache<ImpactsEnums<IE, PE>, SS>),
}

enum TSPostings<'a, IE, PE> {
  Impacts(&'a mut IE),
  Posting(&'a mut PE),
}

impl<'a, IE, PE> TSPostings<'a, IE, PE>
where
  IE: PostingsEnum,
  PE: PostingsEnum,
{
  fn freq(&mut self) -> Result<i32> {
    match self {
      TSPostings::Impacts(disi) => disi.freq(),
      TSPostings::Posting(impacts) => impacts.freq(),
    }
  }

  fn doc_id(&mut self) -> Result<i32> {
    match self {
      TSPostings::Impacts(disi) => Ok(disi.doc_id()),
      TSPostings::Posting(impacts) => Ok(impacts.doc_id()),
    }
  }
}
impl<PE, SS, N, IE> TermScorer<PE, SS, N, IE>
where
  SS: SimScorer,
{
  /// Construct a [`TermScorer`] that will iterate all documents.
  pub fn from_postings(postings_enum: PE, scorer: SS, norms: Option<N>) -> Self {
    let impacts_enum = SlowImpactsEnum::new(postings_enum);
    let max_score_cache = MaxScoreCache::new(ImpactsEnumEnum2::B(impacts_enum), scorer);
    Self {
      norms,
      state: TermScorerState::MaxScoreCache(max_score_cache),
    }
  }
  /// Construct a [`TermScorer`] that will use impacts to skip blocks of non-competitive documents.
  pub fn from_impacts(
    impacts_enum: IE,
    scorer: SS,
    norms: Option<N>,
    top_level_scoring_clause: bool,
  ) -> Self {
    let state = if top_level_scoring_clause {
      let max_score_cache = MaxScoreCache::new(impacts_enum, scorer);
      let disi = SourceImpactsDISI::from_source(max_score_cache);
      TermScorerState::ImpactsDisi(disi)
    } else {
      let max_score_cache = MaxScoreCache::new(ImpactsEnumEnum2::A(impacts_enum), scorer);
      TermScorerState::MaxScoreCache(max_score_cache)
    };

    TermScorer { norms, state }
  }
}

impl<PE, SS, N, IE> TermScorer<PE, SS, N, IE>
where
  PE: PostingsEnum,
  SS: SimScorer,
  N: NumericDocValues,
  IE: ImpactsEnum,
{
  /// Returns term frequency in the current document.
  pub fn freq(&mut self) -> Result<i32> {
    let mut postings = self.postings();
    postings.freq()
  }

  fn postings(&mut self) -> TSPostings<'_, IE, PE> {
    match &mut self.state {
      TermScorerState::ImpactsDisi(impacts_disi) => {
        TSPostings::Impacts(impacts_disi.iterator_mut())
      },
      TermScorerState::MaxScoreCache(inner) => match inner.impacts_source {
        ImpactsEnumEnum2::A(ref mut impacts_enum) => TSPostings::Impacts(impacts_enum),
        ImpactsEnumEnum2::B(ref mut slow_impacts) => {
          TSPostings::Posting(&mut slow_impacts.delegate)
        },
      },
    }
  }

  fn sim_scorer(&self) -> &SS {
    match &self.state {
      TermScorerState::ImpactsDisi(impacts_disi) => &impacts_disi.max_score_cache().scorer,
      TermScorerState::MaxScoreCache(inner) => &inner.scorer,
    }
  }
}

impl<PE, SS, N, IE> Scorable for TermScorer<PE, SS, N, IE>
where
  IE: ImpactsEnum + 'static,
  N: NumericDocValues,
  PE: PostingsEnum + 'static,
  SS: SimScorer + 'static,
{
  fn score(&mut self) -> Result<f32> {
    let mut norm = 1;
    let (freq, doc_id) = {
      let mut postings = self.postings();
      let freq = postings.freq()?;
      let doc_id = postings.doc_id()?;
      (freq, doc_id)
    };
    if let Some(ref mut norms) = self.norms
      && norms.advance_exact(doc_id)?
    {
      norm = norms.long_value()?;
    }
    let scorer = self.sim_scorer();
    Ok(scorer.score(freq as f32, norm))
  }

  fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
    let mut norm = 1;
    if let Some(ref mut norms) = self.norms
      && norms.advance_exact(doc_id)?
    {
      norm = norms.long_value()?;
    }
    let scorer = self.sim_scorer();
    Ok(scorer.score(0f32, norm))
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    if let TermScorerState::ImpactsDisi(impacts_disi) = &mut self.state {
      impacts_disi.set_min_competitive_score(min_score);
    }
    Ok(())
  }

  fn cost(&self) -> Result<i64> {
    self.approximation().cost()
  }
}

impl<PE, SS, N, IE> crate::core::search::scorable::FixedScore for TermScorer<PE, SS, N, IE> {}

impl<PE, SS, N, IE> DocIdSetIterator for TermScorer<PE, SS, N, IE>
where
  PE: PostingsEnum + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues,
  IE: ImpactsEnum + 'static,
{
  fn doc_id(&self) -> i32 {
    match &self.state {
      TermScorerState::ImpactsDisi(impacts_disi) => DocIdSetIterator::doc_id(impacts_disi),
      TermScorerState::MaxScoreCache(inner) => match inner.impacts_source {
        ImpactsEnumEnum2::A(ref impacts_enum) => DocIdSetIterator::doc_id(impacts_enum),
        ImpactsEnumEnum2::B(ref slow_impacts) => DocIdSetIterator::doc_id(&slow_impacts.delegate),
      },
    }
  }
  fn next_doc(&mut self) -> Result<i32> {
    match &mut self.state {
      TermScorerState::ImpactsDisi(impacts_disi) => DocIdSetIterator::next_doc(impacts_disi),
      TermScorerState::MaxScoreCache(inner) => match &mut inner.impacts_source {
        ImpactsEnumEnum2::A(impacts_enum) => DocIdSetIterator::next_doc(impacts_enum),
        ImpactsEnumEnum2::B(slow_impacts) => DocIdSetIterator::next_doc(&mut slow_impacts.delegate),
      },
    }
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    match &mut self.state {
      TermScorerState::ImpactsDisi(impacts_disi) => DocIdSetIterator::advance(impacts_disi, target),
      TermScorerState::MaxScoreCache(inner) => match &mut inner.impacts_source {
        ImpactsEnumEnum2::A(impacts_enum) => DocIdSetIterator::advance(impacts_enum, target),
        ImpactsEnumEnum2::B(slow_impacts) => {
          DocIdSetIterator::advance(&mut slow_impacts.delegate, target)
        },
      },
    }
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    match &mut self.state {
      TermScorerState::ImpactsDisi(impacts_disi) => {
        DocIdSetIterator::slow_advance(impacts_disi, target)
      },
      TermScorerState::MaxScoreCache(inner) => match inner.impacts_source {
        ImpactsEnumEnum2::A(ref mut impacts_enum) => {
          DocIdSetIterator::slow_advance(impacts_enum, target)
        },
        ImpactsEnumEnum2::B(ref mut slow_impacts) => {
          DocIdSetIterator::slow_advance(&mut slow_impacts.delegate, target)
        },
      },
    }
  }
  fn cost(&self) -> Result<i64> {
    match &self.state {
      TermScorerState::ImpactsDisi(impacts_disi) => DocIdSetIterator::cost(impacts_disi),
      TermScorerState::MaxScoreCache(inner) => match inner.impacts_source {
        ImpactsEnumEnum2::A(ref impacts_enum) => DocIdSetIterator::cost(impacts_enum),
        ImpactsEnumEnum2::B(ref slow_impacts) => DocIdSetIterator::cost(&slow_impacts.delegate),
      },
    }
  }
}
impl<PE, SS, N, IE> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for TermScorer<PE, SS, N, IE>
where
  PE: PostingsEnum + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues,
  IE: ImpactsEnum + 'static,
{
  fn get_fixed_bit_set(&self) -> Option<&crate::core::util::fixed_bit_set::FixedBitSet> {
    let iterator: &dyn DocIdSetIterator = {
      match &self.state {
        TermScorerState::ImpactsDisi(impacts_disi) => impacts_disi,
        TermScorerState::MaxScoreCache(inner) => match inner.impacts_source {
          ImpactsEnumEnum2::A(ref impacts_enum) => impacts_enum,
          ImpactsEnumEnum2::B(ref slow_impacts) => &slow_impacts.delegate,
        },
      }
    };
    crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions::get_fixed_bit_set(
      iterator,
    )
  }
  fn get_sparse_fixed_bit_set(
    &self,
  ) -> Option<&crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet> {
    let iterator: &dyn DocIdSetIterator = {
      match &self.state {
        TermScorerState::ImpactsDisi(impacts_disi) => impacts_disi,
        TermScorerState::MaxScoreCache(inner) => match inner.impacts_source {
          ImpactsEnumEnum2::A(ref impacts_enum) => impacts_enum,
          ImpactsEnumEnum2::B(ref slow_impacts) => &slow_impacts.delegate,
        },
      }
    };
    crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions::get_sparse_fixed_bit_set(
      iterator,
    )
  }
  fn get_doc_base_fixed_bit_set(
    &self,
  ) -> Option<(usize, &crate::core::util::fixed_bit_set::FixedBitSet)> {
    let iterator: &dyn DocIdSetIterator = {
      match &self.state {
        TermScorerState::ImpactsDisi(impacts_disi) => impacts_disi,
        TermScorerState::MaxScoreCache(inner) => match inner.impacts_source {
          ImpactsEnumEnum2::A(ref impacts_enum) => impacts_enum,
          ImpactsEnumEnum2::B(ref slow_impacts) => &slow_impacts.delegate,
        },
      }
    };
    crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions::get_doc_base_fixed_bit_set(
      iterator,
    )
  }
}
impl<PE, SS, N, IE> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for TermScorer<PE, SS, N, IE>
where
  PE: PostingsEnum + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues,
  IE: ImpactsEnum + 'static,
{
  fn is_bit_iter(&self) -> bool {
    match &self.state {
      TermScorerState::ImpactsDisi(impacts_disi) => {
        crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::is_bit_iter(impacts_disi)
      },
      TermScorerState::MaxScoreCache(inner) => match inner.impacts_source {
        ImpactsEnumEnum2::A(ref impacts_enum) => {
          crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::is_bit_iter(impacts_enum)
        },
        ImpactsEnumEnum2::B(ref slow_impacts) => {
          crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::is_bit_iter(
            &slow_impacts.delegate,
          )
        },
      },
    }
  }
  fn get(&self, index: usize) -> Result<bool> {
    match &self.state {
      TermScorerState::ImpactsDisi(impacts_disi) => {
        crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::get(impacts_disi, index)
      },
      TermScorerState::MaxScoreCache(inner) => match inner.impacts_source {
        ImpactsEnumEnum2::A(ref impacts_enum) => {
          crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::get(impacts_enum, index)
        },
        ImpactsEnumEnum2::B(ref slow_impacts) => {
          crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::get(
            &slow_impacts.delegate,
            index,
          )
        },
      },
    }
  }
  fn set_doc_id(&mut self, doc: i32) -> Result<()> {
    match &mut self.state {
      TermScorerState::ImpactsDisi(impacts_disi) => {
        crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::set_doc_id(
          impacts_disi,
          doc,
        )
      },
      TermScorerState::MaxScoreCache(inner) => match inner.impacts_source {
        ImpactsEnumEnum2::A(ref mut impacts_enum) => {
          crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::set_doc_id(
            impacts_enum,
            doc,
          )
        },
        ImpactsEnumEnum2::B(ref mut slow_impacts) => {
          crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::set_doc_id(
            &mut slow_impacts.delegate,
            doc,
          )
        },
      },
    }
  }
  fn bit_set_length(&self) -> Result<usize> {
    match &self.state {
      TermScorerState::ImpactsDisi(impacts_disi) => {
        crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::bit_set_length(impacts_disi)
      },
      TermScorerState::MaxScoreCache(inner) => match inner.impacts_source {
        ImpactsEnumEnum2::A(ref impacts_enum) => {
          crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::bit_set_length(
            impacts_enum,
          )
        },
        ImpactsEnumEnum2::B(ref slow_impacts) => {
          crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::bit_set_length(
            &slow_impacts.delegate,
          )
        },
      },
    }
  }
}
impl<PE, SS, N, IE> Scorer for TermScorer<PE, SS, N, IE>
where
  PE: PostingsEnum + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues,
  IE: ImpactsEnum + 'static,
{
  fn scoring_doc_id(&mut self) -> Result<i32> {
    let mut postings = self.postings();
    postings.doc_id()
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let this = *self;
    match this.state {
      TermScorerState::ImpactsDisi(impacts_disi) => Box::new(impacts_disi),
      TermScorerState::MaxScoreCache(inner) => match inner.impacts_source {
        ImpactsEnumEnum2::A(impacts_enum) => Box::new(impacts_enum),
        ImpactsEnumEnum2::B(slow_impacts) => Box::new(slow_impacts.delegate),
      },
    }
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    match &mut self.state {
      TermScorerState::ImpactsDisi(impacts_disi) => {
        impacts_disi.max_score_cache_mut().advance_shallow(target)
      },
      TermScorerState::MaxScoreCache(inner) => inner.advance_shallow(target),
    }
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    match &mut self.state {
      TermScorerState::ImpactsDisi(impacts_disi) => {
        impacts_disi.max_score_cache_mut().get_max_score(upto)
      },
      TermScorerState::MaxScoreCache(inner) => inner.get_max_score(upto),
    }
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }

  fn approximation(&self) -> &dyn DocIdSetIterator {
    self
  }

  fn approximation_mut(&mut self) -> &mut dyn DocIdSetIterator {
    self
  }
}
pub type ImpactsEnums<IE, PE> = ImpactsEnumEnum2<IE, SlowImpactsEnum<PE>>;

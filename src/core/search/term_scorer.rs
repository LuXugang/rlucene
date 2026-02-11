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
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::impacts_disi::ImpactsDISI;
use crate::core::search::max_score_cache::MaxScoreCache;
use crate::core::search::scorable::Scorable;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Expert: A Scorer for documents matching a Term.
pub struct TermScorer<PE, SS, N, IE>
where
    PE: PostingsEnum,
    SS: SimScorer,
    N: NumericDocValues,
    IE: ImpactsEnum,
{
    norms: Option<N>,
    impacts_disi: Option<ImpactsDISI<DummyDISI, IE, SS>>,
    max_score_cache: Option<MaxScoreCache<ImpactsEnums<IE, PE>, SS>>,
}

enum TSPostings<'a, IE, PE>
where
    IE: ImpactsEnum,
    PE: PostingsEnum,
{
    Impacts(&'a mut IE),
    Posting(&'a mut PE),
}

impl<'a, IE, PE> TSPostings<'a, IE, PE>
where
    IE: ImpactsEnum,
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
    PE: PostingsEnum,
    SS: SimScorer,
    N: NumericDocValues,
    IE: ImpactsEnum,
{
    /// Construct a [`TermScorer`] that will iterate all documents.
    pub fn from_postings(postings_enum: PE, scorer: SS, norms: Option<N>) -> Self {
        let impacts_enum = SlowImpactsEnum::new(postings_enum);
        let max_score_cache = MaxScoreCache::new(ImpactsEnumEnum2::B(impacts_enum), scorer);
        Self {
            norms,
            impacts_disi: None,
            max_score_cache: Some(max_score_cache),
        }
    }
    /// Construct a [`TermScorer`] that will use impacts to skip blocks of non-competitive documents.
    pub fn from_impacts(
        impacts_enum: IE,
        scorer: SS,
        norms: Option<N>,
        top_level_scoring_clause: bool,
    ) -> Self {
        let (impacts_disi, max_score_cache) = if top_level_scoring_clause {
            let max_score_cache = MaxScoreCache::new(impacts_enum, scorer);
            let disi = ImpactsDISI::new(DummyDISI, max_score_cache, false);
            (Some(disi), None)
        } else {
            let max_score_cache = MaxScoreCache::new(ImpactsEnumEnum2::A(impacts_enum), scorer);
            (None, Some(max_score_cache))
        };

        TermScorer {
            norms,
            impacts_disi,
            max_score_cache,
        }
    }
    /// Returns term frequency in the current document.
    pub fn freq(&mut self) -> Result<i32> {
        let mut postings = self.postings()?;
        postings.freq()
    }

    fn postings(&mut self) -> Result<TSPostings<'_, IE, PE>> {
        match (&mut self.impacts_disi, &mut self.max_score_cache) {
            (Some(impacts_disi), None) => {
                let v = &mut impacts_disi.max_score_cache.impacts_source;
                Ok(TSPostings::Impacts(v))
            },
            (None, Some(inner)) => match inner.impacts_source {
                ImpactsEnumEnum2::A(ref mut impacts_enum) => Ok(TSPostings::Impacts(impacts_enum)),
                ImpactsEnumEnum2::B(ref mut slow_impacts) => {
                    Ok(TSPostings::Posting(&mut slow_impacts.delegate))
                },
            },
            _ => {
                debug_assert!(false);
                unreachable!("")
            },
        }
    }

    fn sim_scorer(&self) -> Result<&SS> {
        match (&self.impacts_disi, &self.max_score_cache) {
            (Some(impacts_disi), None) => Ok(&impacts_disi.max_score_cache.scorer),
            (None, Some(inner)) => Ok(&inner.scorer),
            _ => Err(LuceneError::illegal_state("")),
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
            let mut postings = self.postings()?;
            let freq = postings.freq()?;
            let doc_id = postings.doc_id()?;
            (freq, doc_id)
        };
        if let Some(ref mut norms) = self.norms
            && norms.advance_exact(doc_id)?
        {
            norm = norms.long_value()?;
        }
        let scorer = self.sim_scorer()?;
        Ok(scorer.score(freq as f32, norm))
    }

    fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
        let mut norm = 1;
        if let Some(ref mut norms) = self.norms
            && norms.advance_exact(doc_id)?
        {
            norm = norms.long_value()?;
        }
        let scorer = self.sim_scorer()?;
        Ok(scorer.score(0f32, norm))
    }

    fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
        if let Some(impacts_disi) = &mut self.impacts_disi {
            impacts_disi.set_min_competitive_score(min_score);
        }
        Ok(())
    }

    fn cost(&mut self) -> Result<i64> {
        Scorer::default_cost(self)
    }
}

impl<PE, SS, N, IE> Scorer for TermScorer<PE, SS, N, IE>
where
    PE: PostingsEnum + 'static,
    SS: SimScorer + 'static,
    N: NumericDocValues,
    IE: ImpactsEnum + 'static,
{
    fn doc_id(&mut self) -> Result<i32> {
        let mut postings = self.postings()?;
        postings.doc_id()
    }

    fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
        match (&self.impacts_disi, &self.max_score_cache) {
            (Some(impacts_disi), None) => Box::new(impacts_disi),
            (None, Some(inner)) => match inner.impacts_source {
                ImpactsEnumEnum2::A(ref impacts_enum) => Box::new(impacts_enum),
                ImpactsEnumEnum2::B(ref slow_impacts) => Box::new(&slow_impacts.delegate),
            },
            _ => {
                debug_assert!(false);
                unreachable!("")
            },
        }
    }

    fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        match (&mut self.impacts_disi, &mut self.max_score_cache) {
            (Some(impacts_disi), None) => Box::new(impacts_disi),
            (None, Some(inner)) => match inner.impacts_source {
                ImpactsEnumEnum2::A(ref mut impacts_enum) => Box::new(impacts_enum),
                ImpactsEnumEnum2::B(ref mut slow_impacts) => Box::new(&mut slow_impacts.delegate),
            },
            _ => {
                debug_assert!(false);
                unreachable!("")
            },
        }
    }

    fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
        let mut this = *self;
        match (this.impacts_disi.take(), this.max_score_cache.take()) {
            (Some(impacts_disi), None) => Box::new(impacts_disi),
            (None, Some(inner)) => match inner.impacts_source {
                ImpactsEnumEnum2::A(impacts_enum) => Box::new(impacts_enum),
                ImpactsEnumEnum2::B(slow_impacts) => Box::new(slow_impacts.delegate),
            },
            _ => {
                debug_assert!(false);
                unreachable!()
            },
        }
    }

    fn advance_shallow(&mut self, target: i32) -> Result<i32> {
        match (&mut self.impacts_disi, &mut self.max_score_cache) {
            (Some(impacts_disi), None) => impacts_disi.max_score_cache.advance_shallow(target),
            (None, Some(inner)) => inner.advance_shallow(target),
            _ => Err(LuceneError::illegal_state("")),
        }
    }

    fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
        match (&mut self.impacts_disi, &mut self.max_score_cache) {
            (Some(impacts_disi), None) => impacts_disi.max_score_cache.get_max_score(up_to),
            (None, Some(inner)) => inner.get_max_score(up_to),
            _ => Err(LuceneError::illegal_state("")),
        }
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
pub type ImpactsEnums<IE, PE> = ImpactsEnumEnum2<IE, SlowImpactsEnum<PE>>;

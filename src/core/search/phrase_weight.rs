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
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::index_reader_context::{
    IRCImpactsEnum, IRCLeafReader, IRCPostingsEnum, IndexReaderContext,
};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::phrase_matcher::{DefaultPhraseMatcherEnum, PhraseMatcher};
use crate::core::search::phrase_scorer::PhraseScorer;
use crate::core::search::query::{Query, QueryBase, QueryWeightSs};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::{SimScorer, SimilarityEnum};
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

pub struct PhraseWeight<IRC, S>
where
    IRC: IndexReaderContext,
    S: PhraseWeightBase<IRC>,
{
    stats: S::SimScorer,
    sub: S,
}
impl<IRC, S> PhraseWeight<IRC, S>
where
    IRC: IndexReaderContext,
    S: PhraseWeightBase<IRC>,
{
    pub(crate) fn new(searcher: &IndexSearcher<IRC>, mut sub: S) -> Result<Self> {
        let stats = sub.get_stats(searcher)?;
        Ok(Self { stats, sub })
    }
}
impl<IRC, S> SegmentCacheable<IRCLeafReader<IRC>> for PhraseWeight<IRC, S>
where
    IRC: IndexReaderContext,
    S: PhraseWeightBase<IRC>,
{
    fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
        Ok(true)
    }
}

impl<IRC, S> Weight<IRCLeafReader<IRC>> for PhraseWeight<IRC, S>
where
    IRC: IndexReaderContext,
    S: PhraseWeightBase<IRC>,
    IRCImpactsEnum<IRC>: 'static,
    IRCPostingsEnum<IRC>: 'static,
    <IRCLeafReader<IRC> as LeafReader>::NormNumericDocValues: 'static,
    S::SimScorer: 'static,
{
    type Matches = MatchWithNoTerms;

    fn matches(
        &self,
        _context: &LeafReaderContext<IRCLeafReader<IRC>>,
        _doc: i32,
    ) -> Result<Option<Self::Matches>> {
        todo!()
    }

    fn explain(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        doc: i32,
    ) -> Result<Explanation> {
        let mut matcher = match self
            .sub
            .get_phrase_matcher(context, self.stats.clone(), false)?
        {
            Some(m) => m,
            None => {
                return Ok(Explanation::no_match_no_details("no matching terms"));
            },
        };

        if matcher.approximation_mut().advance(doc)? != doc {
            return Ok(Explanation::no_match_no_details("no matching terms"));
        }

        matcher.reset()?;

        if !matcher.next_match()? {
            return Ok(Explanation::no_match_no_details("no matching phrase"));
        }

        let mut freq = matcher.sloppy_weight();
        while matcher.next_match()? {
            freq += matcher.sloppy_weight();
        }

        let freq_explanation = Explanation::match_no_details(freq, format!("phraseFreq={}", freq));

        let norms = if self.sub.base().score_mode.needs_scores() {
            context.reader().get_norm_values(&self.sub.base().field)?
        } else {
            None
        };

        let mut norm: i64 = 1;

        if let Some(mut norms) = norms
            && norms.advance_exact(doc)?
        {
            norm = norms.long_value()?;
        }

        let score_explanation = self.stats.explain(freq_explanation, norm);

        Ok(Explanation::match_(
            score_explanation.value,
            format!(
                "weight({} in {}) [{}], result of:",
                self.sub.base().query.as_string(&self.sub.base().field),
                doc,
                self.sub.base().similarity
            ),
            vec![score_explanation],
        ))
    }

    fn get_query(&self) -> Arc<Query> {
        self.sub.base().query.clone()
    }

    type ScorerSupplier = QueryWeightSs<IRCLeafReader<IRC>>;

    fn scorer_supplier(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        match self
            .sub
            .get_phrase_matcher(context, self.stats.clone(), false)?
        {
            Some(matcher) => {
                let norms = if self.sub.base().score_mode.needs_scores() {
                    context.reader().get_norm_values(&self.sub.base().field)?
                } else {
                    None
                };
                let scorer = PhraseScorer::new(
                    matcher,
                    self.sub.base().score_mode,
                    self.stats.clone(),
                    norms,
                );
                Ok(Some(Box::new(DefaultScorerSupplier::new(scorer))))
            },
            None => Ok(None),
        }
    }
}

pub trait PhraseWeightBase<IRC>
where
    IRC: IndexReaderContext,
{
    type SimScorer: SimScorer + Clone;
    fn get_stats(&mut self, searcher: &IndexSearcher<IRC>) -> Result<Self::SimScorer>;

    fn get_phrase_matcher(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        scorer: Self::SimScorer,
        expose_offsets: bool,
    ) -> Result<Option<DefaultPhraseMatcherEnum<IRCLeafReader<IRC>, Self::SimScorer>>>;
    fn base(&self) -> &PhraseWeightMeta;
}
pub struct PhraseWeightMeta {
    pub(crate) field: String,
    pub(crate) score_mode: ScoreMode,
    pub(crate) similarity: Arc<SimilarityEnum>,
    pub(crate) query: Arc<Query>,
}
impl PhraseWeightMeta {
    pub(crate) fn new(
        field: String,
        score_mode: ScoreMode,
        similarity: Arc<SimilarityEnum>,
        query: Query,
    ) -> Self {
        Self {
            field,
            score_mode,
            similarity,
            query: Arc::new(query),
        }
    }
}
#[derive(Default)]
pub struct SimScorerImpl;
impl SimScorer for SimScorerImpl {
    fn score(&self, _freq: f32, _norm: i64) -> f32 {
        1f32
    }
}

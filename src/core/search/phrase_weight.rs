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
use crate::core::index::impacts_enum::ImpactsEnum;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::{LRNormNumericDocValues, LeafReader};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_iterator::MatchesIterator;
use crate::core::search::matches_utils::for_field;
use crate::core::search::phrase_matcher::{PhraseMatcher, PhraseMatcherEnum};
use crate::core::search::phrase_scorer::PhraseScorer;
use crate::core::search::query::{Query, QueryBase, QueryWeightMatchesIterator, QueryWeightSs};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::{
  SimScorer, SimScorerEnum2, Similarity, SimilarityEnum,
};
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

pub type SimScorerType = SimScorerEnum2<<SimilarityEnum as Similarity>::SimScorer, SimScorerImpl>;

pub type PhraseWeightScorer<S, IRC> = PhraseScorer<
  <S as PhraseWeightBase>::IE<IRCLeafReader<IRC>>,
  <S as PhraseWeightBase>::SimScorer,
  LRNormNumericDocValues<IRCLeafReader<IRC>>,
>;

pub type PhraseMatcherResult<IE, SS> = Result<Option<PhraseMatcherEnum<IE, SS>>>;

struct PhraseMatchesIterator<IE, SS>
where
  IE: ImpactsEnum,
  SS: SimScorer,
{
  matcher: PhraseMatcherEnum<IE, SS>,
  started: bool,
  query: Arc<Query>,
}
impl<IE, SS> MatchesIterator for PhraseMatchesIterator<IE, SS>
where
  IE: ImpactsEnum,
  SS: SimScorer,
{
  fn next(&mut self) -> Result<bool> {
    if !self.started {
      self.started = true;
      Ok(true)
    } else {
      self.matcher.next_match()
    }
  }

  fn start_position(&self) -> Result<i32> {
    Ok(self.matcher.start_position())
  }

  fn end_position(&self) -> i32 {
    self.matcher.end_position()
  }

  fn start_offset(&self) -> Result<i32> {
    self.matcher.start_offset()
  }

  fn end_offset(&self) -> Result<i32> {
    self.matcher.end_offset()
  }

  fn get_sub_matches(&mut self) -> Result<Option<QueryWeightMatchesIterator<'_>>> {
    // Phrases are treated as leaves.
    Ok(None)
  }

  fn get_query(&self) -> Arc<Query> {
    self.query.clone()
  }
}

pub struct PhraseWeight<S>
where
  S: PhraseWeightBase,
{
  stats: S::SimScorer,
  sub: S,
}
impl<S> PhraseWeight<S>
where
  S: PhraseWeightBase,
{
  pub(crate) fn new<IRC>(searcher: &IndexSearcher<IRC>, mut sub: S) -> Result<Self>
  where
    IRC: IndexReaderContext,
  {
    let stats = sub.get_stats(searcher)?;
    Ok(Self { stats, sub })
  }
}
impl<S, IRC> SegmentCacheable<IRC> for PhraseWeight<S>
where
  S: PhraseWeightBase,
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    Ok(true)
  }
}

impl<S, IRC> Weight<IRC> for PhraseWeight<S>
where
  S: PhraseWeightBase,
  IRC: IndexReaderContext,
  <S as PhraseWeightBase>::SimScorer: 'static,
  <S as PhraseWeightBase>::IE<IRCLeafReader<IRC>>: 'static,
{
  fn matches<'a>(
    &'a self,
    context: &'a LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    _searcher: &'a IndexSearcher<IRC>,
  ) -> Result<Option<crate::core::search::query::QueryWeightMatches<'a>>> {
    for_field(self.sub.base().field.clone(), move || {
      let Some(mut matcher) = self
        .sub
        .get_phrase_matcher(context, self.stats.clone(), true)?
      else {
        return Ok(None);
      };
      if matcher.approximation_mut().advance(doc)? != doc {
        return Ok(None);
      }
      matcher.reset()?;
      if !matcher.next_match()? {
        return Ok(None);
      }
      Ok(Some(Box::new(PhraseMatchesIterator {
        matcher,
        started: false,
        query: <Self as Weight<IRC>>::get_query(self),
      })))
    })
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    _searcher: &IndexSearcher<IRC>,
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

    let score_explanation = self.stats.explain(freq_explanation, norm)?;

    Ok(Explanation::match_(
      score_explanation.value.clone(),
      format!(
        "weight({} in {}) [{}], result of:",
        self.sub.base().query.to_string(&self.sub.base().field)?,
        doc,
        self.sub.base().similarity
      ),
      vec![score_explanation],
    ))
  }

  fn get_query(&self) -> Arc<Query> {
    self.sub.base().query.clone()
  }

  type ScorerSupplier = QueryWeightSs<IRC>;

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
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
        let scorer: PhraseWeightScorer<S, IRC> = PhraseScorer::new(
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

pub trait PhraseWeightBase {
  type SimScorer: SimScorer + Clone;
  type IE<LR: LeafReader>: ImpactsEnum;

  fn get_stats<IRC>(&mut self, searcher: &IndexSearcher<IRC>) -> Result<Self::SimScorer>
  where
    IRC: IndexReaderContext;

  fn get_phrase_matcher<LR>(
    &self,
    context: &LeafReaderContext<LR>,
    scorer: Self::SimScorer,
    expose_offsets: bool,
  ) -> PhraseMatcherResult<Self::IE<LR>, Self::SimScorer>
  where
    LR: LeafReader;
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

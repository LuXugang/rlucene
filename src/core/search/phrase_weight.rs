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
use crate::core::search::exact_phrase_matcher::ExactPhraseMatcher;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_iterator::MatchesIterator;
use crate::core::search::matches_utils::for_field;
use crate::core::search::phrase_matcher::{PhraseMatcher, PhraseMatcherEnum};
use crate::core::search::phrase_scorer::PhraseScorer;
use crate::core::search::query::QueryRef;
use crate::core::search::query::{Query, QueryWeightMatchesIterator};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::ScorerEnum2;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::{SimScorer, SimScorerEnum2, Similarity};
use crate::core::search::sloppy_phrase_matcher::SloppyPhraseMatcher;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

pub type SimScorerType<
  SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity,
> = SimScorerEnum2<<SIM as Similarity>::SimScorer, SimScorerImpl>;

pub type PhraseWeightScorer<
  S,
  IRC,
  SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity,
> = ScorerEnum2<
  ScorerEnum2<
    PhraseScorer<
      ExactPhraseMatcher<
        <S as PhraseWeightBase<SIM>>::IE<IRCLeafReader<IRC>>,
        <S as PhraseWeightBase<SIM>>::SimScorer,
        true,
      >,
      <S as PhraseWeightBase<SIM>>::SimScorer,
      LRNormNumericDocValues<IRCLeafReader<IRC>>,
      true,
    >,
    PhraseScorer<
      SloppyPhraseMatcher<
        <S as PhraseWeightBase<SIM>>::IE<IRCLeafReader<IRC>>,
        <S as PhraseWeightBase<SIM>>::SimScorer,
      >,
      <S as PhraseWeightBase<SIM>>::SimScorer,
      LRNormNumericDocValues<IRCLeafReader<IRC>>,
      true,
    >,
  >,
  ScorerEnum2<
    PhraseScorer<
      ExactPhraseMatcher<
        <S as PhraseWeightBase<SIM>>::IE<IRCLeafReader<IRC>>,
        <S as PhraseWeightBase<SIM>>::SimScorer,
        false,
      >,
      <S as PhraseWeightBase<SIM>>::SimScorer,
      LRNormNumericDocValues<IRCLeafReader<IRC>>,
      false,
    >,
    PhraseScorer<
      SloppyPhraseMatcher<
        <S as PhraseWeightBase<SIM>>::IE<IRCLeafReader<IRC>>,
        <S as PhraseWeightBase<SIM>>::SimScorer,
      >,
      <S as PhraseWeightBase<SIM>>::SimScorer,
      LRNormNumericDocValues<IRCLeafReader<IRC>>,
      false,
    >,
  >,
>;

pub type PhraseMatcherResult<IE, SS, const TOP_SCORES: bool = false> =
  Result<Option<PhraseMatcherEnum<IE, SS, TOP_SCORES>>>;

pub struct PhraseMatchesIterator<IE, SS> {
  matcher: PhraseMatcherEnum<IE, SS>,
  started: bool,
  query: Arc<Query>,
}
impl<IE, SS> MatchesIterator for PhraseMatchesIterator<IE, SS>
where
  IE: ImpactsEnum,
  SS: SimScorer,
{
  type SubMatches<'a>
    = crate::core::search::dummy::dummy_matches_iterator::DummyMatchesIterator
  where
    Self: 'a;
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

  fn end_position(&self) -> Result<i32> {
    Ok(self.matcher.end_position())
  }

  fn start_offset(&self) -> Result<i32> {
    self.matcher.start_offset()
  }

  fn end_offset(&self) -> Result<i32> {
    self.matcher.end_offset()
  }

  fn get_sub_matches(&mut self) -> Result<Option<Self::SubMatches<'_>>> {
    // Phrases are treated as leaves.
    Ok(None)
  }

  fn get_query(&self) -> Result<&Arc<Query>> {
    Ok(&self.query)
  }
}

pub struct PhraseWeight<
  S,
  SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity,
> where
  SIM: Similarity,
  S: PhraseWeightBase<SIM>,
{
  stats: S::SimScorer,
  sub: S,
}
impl<S, SIM> PhraseWeight<S, SIM>
where
  SIM: Similarity,
  S: PhraseWeightBase<SIM>,
{
  pub(crate) fn new<IRC>(searcher: &IndexSearcher<IRC, SIM>, mut sub: S) -> Result<Self>
  where
    IRC: IndexReaderContext + 'static,
    IndexSearcher<IRC, SIM>: Sync,
  {
    let stats = sub.get_stats(searcher)?;
    Ok(Self { stats, sub })
  }
}
impl<S, IRC, SIM> SegmentCacheable<IRC> for PhraseWeight<S, SIM>
where
  SIM: Similarity,
  S: PhraseWeightBase<SIM>,
  IRC: IndexReaderContext + 'static,
{
  fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    Ok(true)
  }
}

impl<S, IRC, SIM> Weight<IRC, SIM> for PhraseWeight<S, SIM>
where
  SIM: Similarity,
  S: PhraseWeightBase<SIM>,
  IRC: IndexReaderContext + 'static,
  <S as PhraseWeightBase<SIM>>::SimScorer: 'static,
  <S as PhraseWeightBase<SIM>>::IE<IRCLeafReader<IRC>>: 'static,
  PhraseWeightScorer<S, IRC, SIM>: crate::core::search::query_scorer::IntoQueryScorer<
      crate::core::search::scorer_types::LeafScorerTypes<IRCLeafReader<IRC>, SIM>,
    >,
  for<'a> crate::core::search::query_matches::FieldMatchesSupplier<'a, IRC, SIM>: From<(
    &'a PhraseWeight<S, SIM>,
    &'a LeafReaderContext<IRCLeafReader<IRC>>,
    i32,
  )>,
  for<'a> QueryWeightMatchesIterator<'a, IRCLeafReader<IRC>, SIM>:
    From<Box<PhraseMatchesIterator<S::IE<IRCLeafReader<IRC>>, S::SimScorer>>>,
{
  fn matches<'a>(
    &'a self,
    context: &'a LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    _searcher: &'a IndexSearcher<IRC, SIM>,
  ) -> Result<Option<crate::core::search::query::QueryWeightMatches<'a, IRC, SIM>>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    for_field(&self.sub.base().field, (self, context, doc).into())
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    _searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Explanation>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    let mut matcher =
      match self
        .sub
        .get_phrase_matcher::<_, false>(context, self.stats.clone(), false)?
      {
        Some(m) => m,
        None => {
          return Ok(Explanation::no_match_no_details("no matching terms"));
        },
      };

    if matcher.approx_advance(doc)? != doc {
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
        self.sub.get_query().to_string(&self.sub.base().field)?,
        doc,
        self.sub.base().similarity
      ),
      vec![score_explanation],
    ))
  }

  fn get_query(&self) -> QueryRef<'_> {
    self.sub.get_query()
  }

  type ScorerSupplier = DefaultScorerSupplier<PhraseWeightScorer<S, IRC, SIM>>;

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<Self::ScorerSupplier>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    macro_rules! scorer_supplier {
      ($top_scores:literal, $mode:ident) => {
        match self
          .sub
          .get_phrase_matcher::<_, $top_scores>(context, self.stats.clone(), false)?
        {
          Some(matcher) => {
            let norms = if self.sub.base().score_mode.needs_scores() {
              context.reader().get_norm_values(&self.sub.base().field)?
            } else {
              None
            };
            let scorer: PhraseWeightScorer<S, IRC, SIM> = ScorerEnum2::$mode(match matcher {
              PhraseMatcherEnum::Exact(m) => {
                ScorerEnum2::A(PhraseScorer::new(m, self.stats.clone(), norms))
              },
              PhraseMatcherEnum::Sloppy(m) => {
                ScorerEnum2::B(PhraseScorer::new(m, self.stats.clone(), norms))
              },
            });
            Ok(Some(DefaultScorerSupplier::new(scorer)))
          },
          None => Ok(None),
        }
      };
    }
    if self.sub.base().score_mode == ScoreMode::TopScores {
      scorer_supplier!(true, A)
    } else {
      scorer_supplier!(false, B)
    }
  }
}

pub trait PhraseWeightBase<
  SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity,
> where
  SIM: Similarity,
{
  type SimScorer: SimScorer + Clone;
  type IE<LR: LeafReader>: ImpactsEnum;

  fn get_stats<IRC>(&mut self, searcher: &IndexSearcher<IRC, SIM>) -> Result<Self::SimScorer>
  where
    IRC: IndexReaderContext + 'static;

  fn get_phrase_matcher<LR, const TOP_SCORES: bool>(
    &self,
    context: &LeafReaderContext<LR>,
    scorer: Self::SimScorer,
    expose_offsets: bool,
  ) -> PhraseMatcherResult<Self::IE<LR>, Self::SimScorer, TOP_SCORES>
  where
    LR: LeafReader;
  fn base(&self) -> &PhraseWeightMeta<SIM>;
  fn get_query(&self) -> QueryRef<'_>;
}
pub struct PhraseWeightMeta<
  SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity,
> where
  SIM: Similarity,
{
  pub(crate) field: String,
  pub(crate) score_mode: ScoreMode,
  pub(crate) similarity: Arc<SIM>,
}
impl<SIM> PhraseWeightMeta<SIM>
where
  SIM: Similarity,
{
  pub(crate) fn new(field: String, score_mode: ScoreMode, similarity: Arc<SIM>) -> Self {
    Self {
      field,
      score_mode,
      similarity,
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

impl<S: PhraseWeightBase<SIM>, SIM> PhraseWeight<S, SIM>
where
  SIM: Similarity,
{
  pub(crate) fn matches_iterator<'a, IRC: IndexReaderContext + 'static>(
    &'a self,
    context: &'a LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
  ) -> Result<
    Option<crate::core::search::query::QueryWeightMatchesIterator<'a, IRCLeafReader<IRC>, SIM>>,
  >
  where
    QueryWeightMatchesIterator<'a, IRCLeafReader<IRC>, SIM>:
      From<Box<PhraseMatchesIterator<S::IE<IRCLeafReader<IRC>>, S::SimScorer>>>,
  {
    let Some(mut matcher) =
      self
        .sub
        .get_phrase_matcher::<_, false>(context, self.stats.clone(), true)?
    else {
      return Ok(None);
    };
    if matcher.approx_advance(doc)? != doc {
      return Ok(None);
    }
    matcher.reset()?;
    if !matcher.next_match()? {
      return Ok(None);
    }
    Ok(Some(
      Box::new(PhraseMatchesIterator {
        matcher,
        started: false,
        query: Arc::new(self.sub.get_query().into()),
      })
      .into(),
    ))
  }
}

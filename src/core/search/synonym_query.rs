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
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::impact::Impact;
use crate::core::index::impacts::Impacts;
use crate::core::index::impacts_enum::{ImpactsEnum, ImpactsEnumEnum2};
use crate::core::index::impacts_source::ImpactsSource;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::{
  LRImpactsEnum, LRNormNumericDocValues, LRPosting, LRTermsEnum, LeafReader,
};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::postings_enum::{FREQS, PostingsEnum, PostingsEnumEnum2};
use crate::core::index::reader_util::ReaderUtil;
use crate::core::index::slow_impacts_enum::SlowImpactsEnum;
use crate::core::index::term::Term;
use crate::core::index::term_states;
use crate::core::index::term_states::{PrepareState, TermStates};
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::disi_priority_queue::DisiPriorityQueue;
use crate::core::search::disi_wrapper::DisiWrapper;
use crate::core::search::disjunction_disi_approximation::DisjunctionDISIApproximation;
use crate::core::search::disjunction_matches_iterator::from_terms;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, EmptyDISI};
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::impacts_disi::ImpactsDISI;
use crate::core::search::index_searcher;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::for_field;
use crate::core::search::max_score_cache::MaxScoreCache;
use crate::core::search::query::{
  Query, QueryBase, QueryWeight, QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer,
};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, ScorerEnum4, TwoPhaseState};
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::{
  SimScorer, Similarity, SimilarityEnum, SimilarityEnumSimScorer,
};
use crate::core::search::term_query::TermQuery;
use crate::core::search::term_scorer::TermScorer;
use crate::core::search::term_statistics::TermStatistics;
use crate::core::search::weight::Weight;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::UncheckedIOError;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};
use parking_lot::Mutex;
use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;

/// A query that treats multiple terms as synonyms.
///
/// For scoring purposes, this query tries to score the terms as if you had indexed them as one
/// term: it will match any of the terms but only invoke the similarity a single time, scoring the
/// sum of all term frequencies for the document.
#[derive(Clone)]
pub struct SynonymQuery {
  id: Identity,
  terms: Vec<TermAndBoost>,
  field: String,
}

impl SynonymQuery {
  fn new(terms: Vec<TermAndBoost>, field: String) -> Self {
    Self {
      id: Identity::new(),
      terms,
      field,
    }
  }

  /// Returns the terms of this [`SynonymQuery`].
  pub fn get_terms(&self) -> Vec<Term> {
    self
      .terms
      .iter()
      .map(|term| Term::new(self.field.clone(), term.term.clone()))
      .collect()
  }

  /// Returns the field name of this [`SynonymQuery`].
  pub fn get_field(&self) -> &str {
    &self.field
  }

  /// Merge impacts for multiple synonyms.
  pub(crate) fn merge_impacts<IE>(
    impacts_enums: Vec<IE>,
    boosts: Vec<f32>,
  ) -> SynonymImpactsSource<IE>
  where
    IE: ImpactsEnum,
  {
    SynonymImpactsSource::new(impacts_enums, boosts)
  }
}

impl PartialEq for SynonymQuery {
  fn eq(&self, other: &Self) -> bool {
    self.field == other.field && self.terms == other.terms
  }
}

impl Eq for SynonymQuery {}

impl Hash for SynonymQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    std::any::TypeId::of::<Self>().hash(state);
    self.field.hash(state);
    self.terms.hash(state);
  }
}

impl Debug for SynonymQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.to_string("") {
      Ok(s) => write!(f, "{}", s),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl HasIdentity for SynonymQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for SynonymQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    let mut builder = String::from("Synonym(");
    for (i, term_and_boost) in self.terms.iter().enumerate() {
      if i != 0 {
        builder.push(' ');
      }
      let term_query: Query =
        TermQuery::new(Term::new(self.field.clone(), term_and_boost.term.clone())).into();
      builder.push_str(&term_query.to_string(field)?);
      if term_and_boost.boost != 1.0 {
        builder.push('^');
        builder.push_str(&format!("{:.1}", term_and_boost.boost));
      }
    }
    builder.push(')');
    Ok(builder)
  }

  fn create_weight<IRC>(
    self,
    searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext + 'static,
    IndexSearcher<IRC>: Sync,
    Self: Sized,
  {
    if score_mode.needs_scores() {
      Ok(Box::new(SynonymWeight::new(
        self,
        searcher,
        *score_mode,
        boost,
      )?))
    } else {
      // If scores are not needed, let BooleanWeight deal with optimizing that case.
      let mut builder = boolean_query::Builder::new();
      for term in self.terms {
        builder.add(
          TermQuery::new(Term::new(self.field.clone(), term.term)),
          Occur::Should,
        )?;
      }
      searcher.rewrite(builder.build())?.create_weight(
        searcher,
        &ScoreMode::CompleteNoScores,
        boost,
      )
    }
  }

  fn rewrite<IRC>(mut self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    if self.terms.is_empty() {
      return Ok(boolean_query::Builder::new().build().into());
    }
    if self.terms.len() == 1 && self.terms[0].boost == 1.0 {
      return Ok(TermQuery::new(Term::new(self.field, self.terms.remove(0).term)).into());
    }
    Ok(self.into())
  }

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    if !visitor.accept_field(&self.field) {
      return Ok(());
    }
    let query = self.into();
    let mut visitor = visitor.get_sub_visitor(Occur::Should, query);
    visitor.consume_terms(query, &self.get_terms())
  }
}

#[derive(Clone)]
struct TermAndBoost {
  term: BytesRef<Vec<u8>>,
  boost: f32,
}

impl PartialEq for TermAndBoost {
  fn eq(&self, other: &Self) -> bool {
    self.term == other.term && self.boost.to_bits() == other.boost.to_bits()
  }
}

impl Eq for TermAndBoost {}

impl Hash for TermAndBoost {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.term.hash(state);
    self.boost.to_bits().hash(state);
  }
}

struct SynonymWeight {
  term_states: Arc<Vec<Mutex<TermStates>>>,
  similarity: Arc<SimilarityEnum>,
  sim_weight: Option<Arc<SimilarityEnumSimScorer>>,
  score_mode: ScoreMode,
  parent_query: Arc<Query>,
}

impl SynonymWeight {
  fn new<IRC>(
    query: SynonymQuery,
    searcher: &IndexSearcher<IRC>,
    score_mode: ScoreMode,
    boost: f32,
  ) -> Result<Self>
  where
    IRC: IndexReaderContext,
  {
    debug_assert!(score_mode.needs_scores());

    let collection_stats = searcher.collection_statistics(&query.field)?;
    let mut doc_freq = 0;
    let mut total_term_freq = 0;
    let mut term_states = Vec::with_capacity(query.terms.len());

    for term_and_boost in &query.terms {
      let term = Term::new(query.field.clone(), term_and_boost.term.clone());
      let ts = term_states::build(searcher, term.clone(), true)?;

      let ts_doc_freq = ts.doc_freq()?;
      if ts_doc_freq > 0 {
        let term_stats = searcher.term_statistics(term, ts_doc_freq, ts.total_term_freq()?)?;
        doc_freq = doc_freq.max(term_stats.get_doc_freq());
        total_term_freq += term_stats.get_total_term_freq();
      }
      term_states.push(ts);
    }

    let similarity = searcher.get_similarity();
    let sim_weight = if doc_freq > 0 {
      let collection_stats = collection_stats.as_ref().ok_or_else(|| {
        LuceneError::illegal_state("collection statistics are missing for matching synonym terms")
      })?;
      let pseudo_stats = TermStatistics::new(
        Term::from_text(&query.field, "synonym pseudo-term"),
        doc_freq,
        total_term_freq,
      )?;
      Some(Arc::new(similarity.scorer(
        boost,
        collection_stats,
        &[pseudo_stats],
      )?))
    } else {
      None
    };

    Ok(Self {
      term_states: Arc::new(term_states.into_iter().map(Mutex::new).collect()),
      similarity,
      sim_weight,
      score_mode,
      parent_query: Arc::new(query.into()),
    })
  }
}

impl<IRC> SegmentCacheable<IRC> for SynonymWeight
where
  IRC: IndexReaderContext + 'static,
{
  fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    Ok(true)
  }
}

impl<IRC> Weight<IRC> for SynonymWeight
where
  IRC: IndexReaderContext + 'static,
{
  fn matches<'a>(
    &'a self,
    context: &'a LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    _searcher: &'a IndexSearcher<IRC>,
  ) -> Result<Option<crate::core::search::query::QueryWeightMatches<'a>>> {
    let query = if let Query::Synonym(query) = self.parent_query.as_ref() {
      query
    } else {
      return Err(LuceneError::illegal_state(""));
    };
    if context.reader().terms(query.get_field())?.is_none() {
      return Ok(None);
    }
    let field = query.get_field().to_string();
    let terms = query.get_terms();
    for_field(field.clone(), move || {
      from_terms(
        context,
        doc,
        self.parent_query.clone(),
        &field,
        terms.clone(),
      )
    })
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    let parent_query = if let Query::Synonym(query) = self.parent_query.as_ref() {
      query
    } else {
      return Err(LuceneError::illegal_state(""));
    };

    let mut prepare_states = Vec::with_capacity(parent_query.terms.len());
    for term_states in self.term_states.iter() {
      prepare_states.push(term_states.lock().get(context)?);
    }
    let mut supplier = SynonymScorerSupplier::new(
      -1,
      self.term_states.clone(),
      prepare_states,
      parent_query.clone(),
      self.sim_weight.clone(),
      self.score_mode,
    );
    let mut scorer = supplier.get_scorer(context)?;
    if scorer.iterator_mut().advance(doc)? != doc {
      return Ok(Explanation::no_match_no_details("no matching term"));
    }

    let freq = match &mut scorer {
      SynonymScorerEnum::A(scorer) => scorer.freq()?,
      SynonymScorerEnum::B(scorer) => scorer.freq()?,
      SynonymScorerEnum::C(scorer) => scorer.freq()? as f32,
      SynonymScorerEnum::D(_) => {
        return Ok(Explanation::no_match_no_details("no matching term"));
      },
    };
    let freq_explanation = Explanation::match_no_details(freq, format!("termFreq={freq}"));

    let mut norm = 1;
    if let Some(mut norms) = context.reader().get_norm_values(&parent_query.field)?
      && norms.advance_exact(doc)?
    {
      norm = norms.long_value()?;
    }
    let sim_weight = self.sim_weight.as_ref().ok_or_else(|| {
      LuceneError::illegal_state("simWeight is missing for matching synonym terms")
    })?;
    let score_explanation = sim_weight.explain(freq_explanation, norm)?;
    Ok(Explanation::match_(
      score_explanation.value.clone(),
      format!(
        "weight({:?} in {}) [{}], result of:",
        <Self as Weight<IRC>>::get_query(self),
        doc,
        self.similarity,
      ),
      vec![score_explanation],
    ))
  }

  fn get_query(&self) -> Arc<Query> {
    self.parent_query.clone()
  }

  type ScorerSupplier = QueryWeightSs<IRC>;

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    debug_assert!(
      {
        let v = ReaderUtil::get_top_level_context(context);
        self
          .term_states
          .iter()
          .all(|term_states| term_states.lock().was_built_for(v))
      },
      "The top-reader used to create Weight is not the same as the current reader's top-reader"
    );

    let parent_query = if let Query::Synonym(query) = self.parent_query.as_ref() {
      query
    } else {
      return Err(LuceneError::illegal_state(""));
    };

    let mut prepare_states = Vec::with_capacity(parent_query.terms.len());
    for term_states in self.term_states.iter() {
      prepare_states.push(term_states.lock().get(context)?);
    }

    Ok(Some(Box::new(SynonymScorerSupplier::new(
      -1,
      self.term_states.clone(),
      prepare_states,
      parent_query.clone(),
      self.sim_weight.clone(),
      self.score_mode,
    ))))
  }

  fn count(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<i32> {
    <Self as Weight<IRC>>::default_count(self, context)
  }
}

struct SharedPostingsEnum<P> {
  inner: Rc<RefCell<P>>,
}

impl<P> Clone for SharedPostingsEnum<P> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<P> SharedPostingsEnum<P> {
  fn new(inner: P) -> Self {
    Self {
      inner: Rc::new(RefCell::new(inner)),
    }
  }
}

impl<P> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for SharedPostingsEnum<P>
where
  P: PostingsEnum,
{
}
impl<P> DocIdSetIterator for SharedPostingsEnum<P>
where
  P: PostingsEnum,
{
  fn doc_id(&self) -> i32 {
    self.inner.borrow().doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.inner.borrow_mut().next_doc()
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.inner.borrow_mut().advance(target)
  }

  fn cost(&self) -> Result<i64> {
    self.inner.borrow().cost()
  }
}

impl<P> PostingsEnum for SharedPostingsEnum<P>
where
  P: PostingsEnum,
{
  fn freq(&mut self) -> Result<i32> {
    self.inner.borrow_mut().freq()
  }

  fn next_position(&mut self) -> Result<i32> {
    self.inner.borrow_mut().next_position()
  }

  fn start_offset(&self) -> Result<i32> {
    self.inner.borrow().start_offset()
  }

  fn end_offset(&self) -> Result<i32> {
    self.inner.borrow().end_offset()
  }

  fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(
      self
        .inner
        .borrow()
        .get_payload()?
        .map(|payload| Cow::Owned(payload.into_owned())),
    )
  }
}

struct SharedImpactsEnum<I> {
  inner: Rc<RefCell<I>>,
}

impl<I> Clone for SharedImpactsEnum<I> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<I> SharedImpactsEnum<I> {
  fn new(inner: I) -> Self {
    Self {
      inner: Rc::new(RefCell::new(inner)),
    }
  }
}

impl<I> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for SharedImpactsEnum<I>
where
  I: ImpactsEnum,
{
}
impl<I> DocIdSetIterator for SharedImpactsEnum<I>
where
  I: ImpactsEnum,
{
  fn doc_id(&self) -> i32 {
    self.inner.borrow().doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.inner.borrow_mut().next_doc()
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.inner.borrow_mut().advance(target)
  }

  fn cost(&self) -> Result<i64> {
    self.inner.borrow().cost()
  }
}

impl<I> PostingsEnum for SharedImpactsEnum<I>
where
  I: ImpactsEnum,
{
  fn freq(&mut self) -> Result<i32> {
    self.inner.borrow_mut().freq()
  }

  fn next_position(&mut self) -> Result<i32> {
    self.inner.borrow_mut().next_position()
  }

  fn start_offset(&self) -> Result<i32> {
    self.inner.borrow().start_offset()
  }

  fn end_offset(&self) -> Result<i32> {
    self.inner.borrow().end_offset()
  }

  fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(
      self
        .inner
        .borrow()
        .get_payload()?
        .map(|payload| Cow::Owned(payload.into_owned())),
    )
  }
}

impl<I> ImpactsSource for SharedImpactsEnum<I>
where
  I: ImpactsEnum,
{
  fn advance_shallow(&mut self, target: i32) -> Result<()> {
    self.inner.borrow_mut().advance_shallow(target)
  }

  type Impacts<'a>
    = OwnedImpacts
  where
    Self: 'a;

  fn get_impacts(&self) -> Result<Self::Impacts<'_>> {
    let inner = self.inner.borrow();
    let impacts = inner.get_impacts()?;
    OwnedImpacts::from_impacts(&impacts)
  }
}

impl<I> ImpactsEnum for SharedImpactsEnum<I> where I: ImpactsEnum {}

#[derive(Clone)]
struct OwnedImpacts {
  doc_id_uptos: Vec<i32>,
  impacts: Vec<Vec<Impact>>,
}

impl OwnedImpacts {
  fn from_impacts<I>(impacts: &I) -> Result<Self>
  where
    I: Impacts,
  {
    let mut doc_id_uptos = Vec::with_capacity(impacts.num_levels() as usize);
    let mut impact_lists = Vec::with_capacity(impacts.num_levels() as usize);
    for level in 0..impacts.num_levels() {
      doc_id_uptos.push(impacts.get_doc_id_upto(level));
      impact_lists.push(impacts.get_impacts(level)?);
    }
    Ok(Self {
      doc_id_uptos,
      impacts: impact_lists,
    })
  }
}

impl Impacts for OwnedImpacts {
  fn num_levels(&self) -> i32 {
    self.doc_id_uptos.len() as i32
  }

  fn get_doc_id_upto(&self, level: i32) -> i32 {
    self.doc_id_uptos[level as usize]
  }

  fn get_impacts(&self, level: i32) -> Result<Vec<Impact>> {
    Ok(self.impacts[level as usize].clone())
  }
}

pub(crate) struct SynonymImpactsSource<IE> {
  impacts_enums: Vec<IE>,
  boosts: Vec<f32>,
}

impl<IE> SynonymImpactsSource<IE> {
  fn new(impacts_enums: Vec<IE>, boosts: Vec<f32>) -> Self {
    debug_assert_eq!(impacts_enums.len(), boosts.len());
    Self {
      impacts_enums,
      boosts,
    }
  }
}

impl<IE> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for SynonymImpactsSource<IE>
where
  IE: ImpactsEnum,
{
}
impl<IE> DocIdSetIterator for SynonymImpactsSource<IE>
where
  IE: ImpactsEnum,
{
  fn doc_id(&self) -> i32 {
    self
      .impacts_enums
      .iter()
      .map(|impacts_enum| impacts_enum.doc_id())
      .min()
      .unwrap_or(-1)
  }

  fn next_doc(&mut self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn cost(&self) -> Result<i64> {
    let mut cost = 0;
    for impacts_enum in &self.impacts_enums {
      cost += impacts_enum.cost()?;
    }
    Ok(cost)
  }
}

impl<IE> PostingsEnum for SynonymImpactsSource<IE>
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

impl<IE> ImpactsSource for SynonymImpactsSource<IE>
where
  IE: ImpactsEnum,
{
  fn advance_shallow(&mut self, target: i32) -> Result<()> {
    for impacts_enum in &mut self.impacts_enums {
      if impacts_enum.doc_id() < target {
        impacts_enum.advance_shallow(target)?;
      }
    }
    Ok(())
  }

  type Impacts<'a>
    = SynonymImpacts
  where
    Self: 'a;

  fn get_impacts(&self) -> Result<Self::Impacts<'_>> {
    let mut impacts = Vec::with_capacity(self.impacts_enums.len());
    let mut doc_ids = Vec::with_capacity(self.impacts_enums.len());
    let mut lead = 0;
    let mut lead_doc_id_up_to = None;

    for (i, impacts_enum) in self.impacts_enums.iter().enumerate() {
      let impact_values = impacts_enum.get_impacts()?;
      let impact_values = OwnedImpacts::from_impacts(&impact_values)?;
      let doc_id_up_to = impact_values.get_doc_id_upto(0);
      if lead_doc_id_up_to.is_none_or(|lead_doc_id| doc_id_up_to < lead_doc_id) {
        lead = i;
        lead_doc_id_up_to = Some(doc_id_up_to);
      }
      impacts.push(impact_values);
      doc_ids.push(impacts_enum.doc_id());
    }

    Ok(SynonymImpacts {
      impacts,
      doc_ids,
      boosts: self.boosts.clone(),
      lead,
    })
  }
}

impl<IE> ImpactsEnum for SynonymImpactsSource<IE> where IE: ImpactsEnum {}

pub(crate) struct SynonymImpacts {
  impacts: Vec<OwnedImpacts>,
  doc_ids: Vec<i32>,
  boosts: Vec<f32>,
  lead: usize,
}

impl SynonymImpacts {
  fn get_level(impacts: &OwnedImpacts, doc_id_up_to: i32) -> i32 {
    for level in 0..impacts.num_levels() {
      if impacts.get_doc_id_upto(level) >= doc_id_up_to {
        return level;
      }
    }
    -1
  }

  fn merge_impacts(to_merge: Vec<Vec<Impact>>) -> Result<Vec<Impact>> {
    let mut pq = PriorityQueue::new(to_merge.len(), SynonymSubIteratorCmp)?;
    pq.add_all(
      to_merge
        .into_iter()
        .map(SynonymSubIterator::new)
        .collect::<Vec<_>>(),
    )?;

    let mut merged_impacts = Vec::new();

    // Idea: merge impacts by norm. The tricky thing is that we need to
    // consider norm values that are not in the impacts too. For
    // instance if the list of impacts is [{freq=2,norm=10}, {freq=4,norm=12}],
    // there might well be a document that has a freq of 2 and a length of 11,
    // which was just not added to the list of impacts because {freq=2,norm=10}
    // is more competitive. So the way it works is that we track the sum of
    // the term freqs that we have seen so far in order to account for these
    // implicit impacts.
    let mut sum_tf = 0i64;
    let mut top = pq
      .top_mut()
      .ok_or_else(|| LuceneError::illegal_state("top is None"))?;
    loop {
      let norm = top.current()?.norm;
      loop {
        let current_freq = top.current()?.freq;
        sum_tf += (current_freq - top.previous_freq) as i64;
        top.next();
        top = pq.update_top()?;
        if top.current_opt().is_none() || top.current()?.norm != norm {
          break;
        }
      }

      let freq_upper_bound = std::cmp::min(i32::MAX as i64, sum_tf) as i32;
      match merged_impacts.last() {
        None => merged_impacts.push(Impact::new(freq_upper_bound, norm)),
        Some(prev_impact) => {
          debug_assert!((prev_impact.norm as u64) < (norm as u64));
          if freq_upper_bound > prev_impact.freq {
            merged_impacts.push(Impact::new(freq_upper_bound, norm));
          } // otherwise the previous impact is already more competitive
        },
      }

      if top.current_opt().is_none() {
        break;
      }
    }

    Ok(merged_impacts)
  }
}

impl Impacts for SynonymImpacts {
  fn num_levels(&self) -> i32 {
    self.impacts[self.lead].num_levels()
  }

  fn get_doc_id_upto(&self, level: i32) -> i32 {
    self.impacts[self.lead].get_doc_id_upto(level)
  }

  fn get_impacts(&self, level: i32) -> Result<Vec<Impact>> {
    let doc_id_up_to = self.get_doc_id_upto(level);
    let mut to_merge = Vec::new();

    for i in 0..self.impacts.len() {
      if self.doc_ids[i] <= doc_id_up_to {
        let impacts_level = Self::get_level(&self.impacts[i], doc_id_up_to);
        if impacts_level == -1 {
          // One instance doesn't have impacts that cover up to docIdUpTo.
          // Return impacts that trigger the maximum score.
          return Ok(vec![Impact::new(i32::MAX, 1)]);
        }

        let mut impact_list = self.impacts[i].get_impacts(impacts_level)?;
        if self.boosts[i] != 1.0 {
          let boost = self.boosts[i];
          for impact in &mut impact_list {
            impact.freq = ((impact.freq as f32) * boost).ceil() as i32;
          }
        }
        to_merge.push(impact_list);
      }
    }

    debug_assert!(!to_merge.is_empty());
    if to_merge.len() == 1 {
      return Ok(to_merge.remove(0));
    }

    Self::merge_impacts(to_merge)
  }
}

struct SynonymSubIterator {
  iterator: Vec<Impact>,
  current: Option<usize>,
  previous_freq: i32,
}

impl SynonymSubIterator {
  fn new(impacts: Vec<Impact>) -> Self {
    let current = if impacts.is_empty() { None } else { Some(0) };
    Self {
      iterator: impacts,
      current,
      previous_freq: 0,
    }
  }

  fn next(&mut self) {
    let Some(idx) = self.current else {
      return;
    };
    self.previous_freq = self.iterator[idx].freq;
    let next_idx = idx + 1;
    self.current = if next_idx >= self.iterator.len() {
      None
    } else {
      Some(next_idx)
    };
  }

  fn current(&self) -> Result<&Impact> {
    self
      .current_opt()
      .ok_or_else(|| LuceneError::illegal_state("current is None"))
  }

  fn current_opt(&self) -> Option<&Impact> {
    self.current.map(|idx| &self.iterator[idx])
  }
}

struct SynonymSubIteratorCmp;

impl Compare<SynonymSubIterator> for SynonymSubIteratorCmp {
  fn less_than(&self, a: &SynonymSubIterator, b: &SynonymSubIterator) -> Result<bool> {
    match (a.current_opt(), b.current_opt()) {
      (None, _) => Ok(false),
      (_, None) => Ok(true),
      (Some(a), Some(b)) => Ok((a.norm as u64) < (b.norm as u64)),
    }
  }
}

type SynonymPostingsEnum<LR> =
  PostingsEnumEnum2<SharedImpactsEnum<LRImpactsEnum<LR>>, SharedPostingsEnum<LRPosting<LR>>>;

type SynonymImpactsEnum<LR> = ImpactsEnumEnum2<
  SharedImpactsEnum<LRImpactsEnum<LR>>,
  SlowImpactsEnum<SharedPostingsEnum<LRPosting<LR>>>,
>;

type SynonymTermScorer<LR> = TermScorer<
  SynonymPostingsEnum<LR>,
  Arc<SimilarityEnumSimScorer>,
  LRNormNumericDocValues<LR>,
  SynonymImpactsEnum<LR>,
>;

type SynonymDisi<LR> = DisjunctionDISIApproximation<SynonymSubScorer<LR>>;

type SynonymImpactsDISI<LR> = ImpactsDISI<
  SynonymDisi<LR>,
  SynonymImpactsSource<SynonymImpactsEnum<LR>>,
  Arc<SimilarityEnumSimScorer>,
>;

type EmptySynonymScorer = ConstantScoreScorer<EmptyDISI, DummyTwoPhaseIterator>;

enum SynonymSubScorer<LR>
where
  LR: LeafReader,
{
  Term {
    scorer: SynonymTermScorer<LR>,
    boost: f32,
  },
}

impl<LR> SynonymSubScorer<LR>
where
  LR: LeafReader,
{
  fn freq(&mut self) -> Result<f32> {
    match self {
      SynonymSubScorer::Term { scorer, boost } => Ok(*boost * scorer.freq()? as f32),
    }
  }
}

impl<LR> FixedScore for SynonymSubScorer<LR> where LR: LeafReader {}

impl<LR> Scorable for SynonymSubScorer<LR>
where
  LR: LeafReader + 'static,
{
  fn score(&mut self) -> Result<f32> {
    match self {
      SynonymSubScorer::Term { scorer, .. } => scorer.score(),
    }
  }

  fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
    match self {
      SynonymSubScorer::Term { scorer, .. } => scorer.smoothing_score(doc_id),
    }
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    match self {
      SynonymSubScorer::Term { scorer, .. } => scorer.set_min_competitive_score(min_score),
    }
  }

  fn cost(&self) -> Result<i64> {
    match self {
      SynonymSubScorer::Term { scorer, .. } => scorer.cost(),
    }
  }
}

impl<LR> Scorer for SynonymSubScorer<LR>
where
  LR: LeafReader + 'static,
{
  fn doc_id(&mut self) -> Result<i32> {
    match self {
      SynonymSubScorer::Term { scorer, .. } => scorer.doc_id(),
    }
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    match self {
      SynonymSubScorer::Term { scorer, .. } => scorer.iterator(),
    }
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    match self {
      SynonymSubScorer::Term { scorer, .. } => scorer.iterator_mut(),
    }
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    match *self {
      SynonymSubScorer::Term { scorer, .. } => Box::new(scorer).take_iterator(),
    }
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    match self {
      SynonymSubScorer::Term { scorer, .. } => scorer.get_max_score(upto),
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

struct SynonymScorer<LR>
where
  LR: LeafReader + 'static,
{
  impacts_disi: SynonymImpactsDISI<LR>,
  sim_weight: Arc<SimilarityEnumSimScorer>,
  norms: Option<LRNormNumericDocValues<LR>>,
  score_mode: ScoreMode,
}

impl<LR> SynonymScorer<LR>
where
  LR: LeafReader + 'static,
{
  fn new(
    impacts_disi: SynonymImpactsDISI<LR>,
    sim_weight: Arc<SimilarityEnumSimScorer>,
    norms: Option<LRNormNumericDocValues<LR>>,
    score_mode: ScoreMode,
  ) -> Self {
    Self {
      impacts_disi,
      sim_weight,
      norms,
      score_mode,
    }
  }

  fn freq(&mut self) -> Result<f32> {
    let list_index = self.impacts_disi.in_.top_list_root();
    let all_scores = self.impacts_disi.in_.all_scores_mut();
    let mut freq = all_scores[list_index].scorer.freq()?;
    let mut next = all_scores[list_index].next;
    while let Some(next_index) = next {
      freq += all_scores[next_index].scorer.freq()?;
      next = all_scores[next_index].next;
    }
    Ok(freq)
  }

  fn use_impacts_disi(&self) -> bool {
    self.score_mode == ScoreMode::TopScores
  }
}

impl<LR> FixedScore for SynonymScorer<LR> where LR: LeafReader + 'static {}

impl<LR> Scorable for SynonymScorer<LR>
where
  LR: LeafReader + 'static,
{
  fn score(&mut self) -> Result<f32> {
    let mut norm = 1;
    let doc = if self.use_impacts_disi() {
      self.impacts_disi.doc_id()
    } else {
      self.impacts_disi.in_.doc_id()
    };
    if let Some(ref mut norms) = self.norms
      && norms.advance_exact(doc)?
    {
      norm = norms.long_value()?;
    }
    let freq = self.freq()?;
    Ok(self.sim_weight.score(freq, norm))
  }

  fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
    let mut norm = 1;
    if let Some(ref mut norms) = self.norms
      && norms.advance_exact(doc_id)?
    {
      norm = norms.long_value()?;
    }
    Ok(self.sim_weight.score(0.0, norm))
  }

  fn set_min_competitive_score(&mut self, _min_score: f32) -> Result<()> {
    self.impacts_disi.set_min_competitive_score(_min_score);
    Ok(())
  }

  fn cost(&self) -> Result<i64> {
    self.impacts_disi.cost()
  }
}

impl<LR> Scorer for SynonymScorer<LR>
where
  LR: LeafReader + 'static,
{
  fn doc_id(&mut self) -> Result<i32> {
    Ok(if self.use_impacts_disi() {
      self.impacts_disi.doc_id()
    } else {
      self.impacts_disi.in_.doc_id()
    })
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    if self.use_impacts_disi() {
      Box::new(&self.impacts_disi)
    } else {
      Box::new(&self.impacts_disi.in_)
    }
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    if self.use_impacts_disi() {
      Box::new(&mut self.impacts_disi)
    } else {
      Box::new(&mut self.impacts_disi.in_)
    }
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    if self.use_impacts_disi() {
      Box::new(self.impacts_disi)
    } else {
      Box::new(self.impacts_disi.in_)
    }
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    self.impacts_disi.max_score_cache.advance_shallow(target)
  }

  fn get_max_score(&mut self, _upto: i32) -> Result<f32> {
    self.impacts_disi.max_score_cache.get_max_score(_upto)
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

struct FreqBoostTermScorer<LR>
where
  LR: LeafReader + 'static,
{
  boost: f32,
  in_: SynonymTermScorer<LR>,
  scorer: Arc<SimilarityEnumSimScorer>,
  norms: Option<LRNormNumericDocValues<LR>>,
}

impl<LR> FreqBoostTermScorer<LR>
where
  LR: LeafReader + 'static,
{
  fn new(
    boost: f32,
    in_: SynonymTermScorer<LR>,
    sim_weight: Arc<SimilarityEnumSimScorer>,
    norms: Option<LRNormNumericDocValues<LR>>,
  ) -> Result<Self> {
    if boost.is_nan() || !(0.0..=1.0).contains(&boost) {
      return Err(LuceneError::illegal_argument(
        "boost must be a positive float between 0 (exclusive) and 1 (inclusive)",
      ));
    }
    Ok(Self {
      boost,
      in_,
      scorer: sim_weight,
      norms,
    })
  }

  fn freq(&mut self) -> Result<f32> {
    Ok(self.boost * self.in_.freq()? as f32)
  }
}

impl<LR> FixedScore for FreqBoostTermScorer<LR> where LR: LeafReader + 'static {}

impl<LR> Scorable for FreqBoostTermScorer<LR>
where
  LR: LeafReader + 'static,
{
  fn score(&mut self) -> Result<f32> {
    let mut norm = 1;
    let doc = self.in_.doc_id()?;
    if let Some(ref mut norms) = self.norms
      && norms.advance_exact(doc)?
    {
      norm = norms.long_value()?;
    }
    let freq = self.freq()?;
    Ok(self.scorer.score(freq, norm))
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    self.in_.set_min_competitive_score(min_score)
  }
}

impl<LR> Scorer for FreqBoostTermScorer<LR>
where
  LR: LeafReader + 'static,
{
  fn doc_id(&mut self) -> Result<i32> {
    self.in_.doc_id()
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    self.in_.iterator()
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    self.in_.iterator_mut()
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    Box::new(self.in_).take_iterator()
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    self.in_.advance_shallow(target)
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    self.in_.get_max_score(upto)
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

type SynonymScorerEnum<LR> = ScorerEnum4<
  SynonymScorer<LR>,
  FreqBoostTermScorer<LR>,
  SynonymTermScorer<LR>,
  EmptySynonymScorer,
>;

struct SynonymScorerSupplier<LR>
where
  LR: LeafReader,
{
  cost: i64,
  term_states: Arc<Vec<Mutex<TermStates>>>,
  prepare_states: Vec<Option<PrepareState<LRTermsEnum<LR>>>>,
  query: SynonymQuery,
  sim_weight: Option<Arc<SimilarityEnumSimScorer>>,
  score_mode: ScoreMode,
  iterators: Option<Vec<SynonymPostingsEnum<LR>>>,
  impacts: Option<Vec<SynonymImpactsEnum<LR>>>,
  term_boosts: Vec<f32>,
  initialized: bool,
}

impl<LR> SynonymScorerSupplier<LR>
where
  LR: LeafReader,
{
  fn new(
    cost: i64,
    term_states: Arc<Vec<Mutex<TermStates>>>,
    prepare_states: Vec<Option<PrepareState<LRTermsEnum<LR>>>>,
    query: SynonymQuery,
    sim_weight: Option<Arc<SimilarityEnumSimScorer>>,
    score_mode: ScoreMode,
  ) -> Self {
    Self {
      cost,
      term_states,
      prepare_states,
      query,
      sim_weight,
      score_mode,
      iterators: None,
      impacts: None,
      term_boosts: Vec::new(),
      initialized: false,
    }
  }

  fn init(&mut self, context: &LeafReaderContext<LR>) -> Result<()> {
    if self.initialized {
      return Ok(());
    }

    let mut iterators = Vec::new();
    let mut impacts = Vec::new();
    let mut term_boosts = Vec::new();
    let mut cost = 0;

    for i in 0..self.query.terms.len() {
      let Some(mut prepare_state) = self.prepare_states[i].take() else {
        continue;
      };
      let state = self.term_states[i].lock().resolve(&mut prepare_state)?;
      if let Some(state) = state {
        let mut terms_enum = context
          .reader()
          .terms(&self.query.field)?
          .ok_or_else(|| LuceneError::illegal_state("term should exist here"))?
          .iterator()?;
        terms_enum.seek_exact_with_state(&self.query.terms[i].term, state.as_ref())?;
        if self.score_mode == ScoreMode::TopScores {
          let impacts_enum = SharedImpactsEnum::new(terms_enum.impacts(FREQS as i32)?);
          iterators.push(PostingsEnumEnum2::A(impacts_enum.clone()));
          impacts.push(ImpactsEnumEnum2::A(impacts_enum));
        } else {
          let postings_enum =
            SharedPostingsEnum::new(terms_enum.postings_with_flags(None, FREQS as i32)?);
          iterators.push(PostingsEnumEnum2::B(postings_enum.clone()));
          impacts.push(ImpactsEnumEnum2::B(SlowImpactsEnum::new(postings_enum)));
        }
        term_boosts.push(self.query.terms[i].boost);
      }
    }

    for iterator in &iterators {
      cost += iterator.cost()?;
    }

    self.iterators = Some(iterators);
    self.impacts = Some(impacts);
    self.term_boosts = term_boosts;
    self.cost = cost;
    self.initialized = true;
    Ok(())
  }

  fn get_scorer(&mut self, context: &LeafReaderContext<LR>) -> Result<SynonymScorerEnum<LR>>
  where
    LR: 'static,
  {
    self.init(context)?;

    let Some(mut iterators) = self.iterators.take() else {
      return Err(LuceneError::illegal_state(
        "ScorerSupplier.get must be called at most once",
      ));
    };
    let Some(mut impacts) = self.impacts.take() else {
      return Err(LuceneError::illegal_state(
        "ScorerSupplier.get must be called at most once",
      ));
    };
    let mut term_boosts = std::mem::take(&mut self.term_boosts);

    if iterators.is_empty() {
      return Ok(SynonymScorerEnum::D(ConstantScoreScorer::from_disi(
        0.0,
        self.score_mode,
        EmptyDISI::new(),
      )));
    }

    let sim_weight = self.sim_weight.as_ref().cloned().ok_or_else(|| {
      LuceneError::illegal_state("simWeight is missing for matching synonym terms")
    })?;
    let norms = context.reader().get_norm_values(&self.query.field)?;

    if iterators.len() == 1 {
      let iterator = iterators.remove(0);
      let impact = impacts.remove(0);
      let boost = term_boosts.remove(0);
      return if self.score_mode == ScoreMode::CompleteNoScores || boost == 1.0 {
        let scorer = if self.score_mode == ScoreMode::TopScores {
          TermScorer::from_impacts(impact, sim_weight.clone(), norms, false)
        } else {
          TermScorer::from_postings(iterator, sim_weight.clone(), norms)
        };
        Ok(SynonymScorerEnum::C(scorer))
      } else {
        let scorer = if self.score_mode == ScoreMode::TopScores {
          TermScorer::from_impacts(impact, sim_weight.clone(), None, false)
        } else {
          TermScorer::from_postings(iterator, sim_weight.clone(), None)
        };
        Ok(SynonymScorerEnum::B(FreqBoostTermScorer::new(
          boost,
          scorer,
          sim_weight.clone(),
          norms,
        )?))
      };
    }

    let mut wrappers = Vec::with_capacity(iterators.len());
    let mut queue = DisiPriorityQueue::new(iterators.len());
    for (idx, (iterator, boost)) in iterators
      .into_iter()
      .zip(term_boosts.iter().copied())
      .enumerate()
    {
      let term_scorer = TermScorer::from_postings(iterator, sim_weight.clone(), None);
      let mut wrapper = DisiWrapper::new(SynonymSubScorer::Term {
        scorer: term_scorer,
        boost,
      })?;
      wrapper.doc = -1;
      wrappers.push(wrapper);
      queue.add(idx, &wrappers);
    }

    let iterator = DisjunctionDISIApproximation::new(queue, wrappers)?;
    let impacts_source = SynonymQuery::merge_impacts(impacts, term_boosts);
    let max_score_cache = MaxScoreCache::new(impacts_source, sim_weight.clone());
    let impacts_disi = ImpactsDISI::new(iterator, max_score_cache, true);

    Ok(SynonymScorerEnum::A(SynonymScorer::new(
      impacts_disi,
      sim_weight,
      norms,
      self.score_mode,
    )))
  }
}

impl<IRC> ScorerSupplier<IRC> for SynonymScorerSupplier<IRCLeafReader<IRC>>
where
  IRC: IndexReaderContext,
{
  type Scorer = QueryWeightSsScorer;
  type BulkScorer = QueryWeightSsBulkScorer;

  fn get(
    &mut self,
    _lead_cost: i64,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::Scorer> {
    Ok(Box::new(self.get_scorer(context)?))
  }

  fn bulk_scorer(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::BulkScorer>> {
    Ok(Some(Box::new(self.default_bulk_scorer(context, searcher)?)))
  }

  fn cost(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<i64> {
    let result: Result<i64> = (|| {
      self.init(context)?;
      Ok(self.cost)
    })();

    match result {
      Ok(v) => Ok(v),
      Err(e) => {
        let mut err = UncheckedIOError::new("");
        err.add_suppressed(e);
        Err(err.into())
      },
    }
  }
}

/// A builder for [`SynonymQuery`].
pub struct Builder {
  field: String,
  terms: Vec<TermAndBoost>,
}

impl Builder {
  /// Creates a new instance.
  pub fn new<T>(field: T) -> Self
  where
    T: Into<String>,
  {
    Self {
      field: field.into(),
      terms: Vec::new(),
    }
  }

  /// Adds the provided [`Term`] as a synonym.
  pub fn add_term(&mut self, term: Term) -> Result<&mut Self> {
    self.add_term_with_boost(term, 1.0)
  }

  /// Adds the provided [`Term`] as a synonym with a document-frequency boost.
  pub fn add_term_with_boost(&mut self, term: Term, boost: f32) -> Result<&mut Self> {
    if self.field != term.field {
      return Err(LuceneError::illegal_argument(
        "Synonyms must be across the same field",
      ));
    }
    self.add_bytes_with_boost(term.bytes, boost)
  }

  /// Adds the provided term bytes as a synonym with a document-frequency boost.
  pub fn add_bytes_with_boost(&mut self, term: BytesRef<Vec<u8>>, boost: f32) -> Result<&mut Self> {
    if boost.is_nan() || boost <= 0.0 || boost > 1.0 {
      return Err(LuceneError::illegal_argument(
        "boost must be a positive float between 0 (exclusive) and 1 (inclusive)",
      ));
    }
    if self.terms.len() >= index_searcher::get_max_clause_count() {
      return Err(index_searcher::new_nested());
    }
    self.terms.push(TermAndBoost { term, boost });
    Ok(self)
  }

  /// Builds the [`SynonymQuery`].
  pub fn build(mut self) -> SynonymQuery {
    self.terms.sort_by(|a, b| a.term.cmp(&b.term));
    SynonymQuery::new(self.terms, self.field)
  }
}

impl crate::core::util::accountable::Accountable for SynonymQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}

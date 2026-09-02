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
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::{LRNormNumericDocValues, LRPosting, LeafReader};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::postings_enum::{POSITIONS, PostingsEnum};
use crate::core::index::reader_util::ReaderUtil;
use crate::core::index::term::Term;
use crate::core::index::term_states::{TermStates, build};
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::multi_phrase_query;
use crate::core::search::phrase_query;
use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::Scorer;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::{
  SimScorer, Similarity, SimilarityEnum, SimilarityEnumSimScorer,
};
use crate::core::search::term_query::TermQuery;
use crate::core::search::term_statistics::TermStatistics;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::accountable::Accountable;
use crate::core::util::automation::automaton::{Automaton, Builder};
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::transition::Transition;
use crate::core::util::automation::transition_accessor::TransitionAccessor;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::ram_usage_estimator::{size_of_hash_map, size_of_string, size_of_vec};
use crate::sandbox::search::term_automaton_scorer::{EnumAndScorer, TermAutomatonScorer};
#[cfg(test)]
use crate::test_framework::core::search::test_term_automaton_query::CustomTermAutomatonQuery;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// A proximity query that lets you express an automaton, whose transitions are terms, to match
/// documents. This is a generalization of other proximity queries like [`PhraseQuery`](crate::core::search::phrase_query::PhraseQuery),
/// [`MultiPhraseQuery`](crate::core::search::multi_phrase_query::MultiPhraseQuery) and `SpanNearQuery`. It is likely slow, since it visits any document having
/// any of the terms (i.e. it acts like a disjunction, not a conjunction like [`PhraseQuery`](crate::core::search::phrase_query::PhraseQuery)), and
/// then it must merge-sort all positions within each document to test whether/how many times the
/// automaton matches.
///
/// After creating the query, use `create_state`, `set_accept`, `add_transition` and
/// `add_any_transition` to build up the automaton. Once you are done, call `finish` and then execute
/// the query.
///
/// This code is very new and likely has exciting bugs!
///
/// Experimental: this API follows the original Lucene experimental status.
#[derive(Clone)]
pub struct TermAutomatonQuery {
  id: Identity,
  field: String,
  builder: Builder,
  det: Option<Automaton>,
  term_to_id: HashMap<Vec<u8>, i32>,
  id_to_term: Vec<Option<BytesRef<Vec<u8>>>>,
  any_term_id: i32,
  hook: TermAutomatonQueryHook,
}

impl TermAutomatonQuery {
  pub fn new(field: impl Into<String>) -> Self {
    Self {
      id: Identity::new(),
      field: field.into(),
      builder: Builder::new(),
      det: None,
      term_to_id: HashMap::new(),
      id_to_term: Vec::new(),
      any_term_id: -1,
      hook: TermAutomatonQueryHook::Default(TermAutomatonQueryDefault),
    }
  }

  #[cfg(test)]
  pub(crate) fn with_hook(field: impl Into<String>, hook: CustomTermAutomatonQuery) -> Self {
    let mut query = Self::new(field);
    query.hook = TermAutomatonQueryHook::Custom(hook);
    query
  }

  /// Returns a new state; state 0 is always the initial state.
  pub fn create_state(&mut self) -> i32 {
    self.builder.create_state()
  }

  /// Marks the specified state as accept or not.
  pub fn set_accept(&mut self, state: i32, accept: bool) {
    self.builder.set_accept(state, accept);
  }

  /// Adds a transition to the automaton.
  pub fn add_transition(&mut self, source: i32, dest: i32, term: &str) -> Result<()> {
    self.add_transition_bytes(source, dest, &BytesRef::from(term))
  }

  /// Adds a transition to the automaton.
  pub fn add_transition_bytes(
    &mut self,
    source: i32,
    dest: i32,
    term: &BytesRef<Vec<u8>>,
  ) -> Result<()> {
    let term_id = self.get_term_id(Some(term))?;
    self.builder.add_transition_label(source, dest, term_id)
  }

  /// Adds a transition matching any term.
  pub fn add_any_transition(&mut self, source: i32, dest: i32) -> Result<()> {
    let term_id = self.get_term_id(None)?;
    self.builder.add_transition_label(source, dest, term_id)
  }

  /// Call this once you are done adding states/transitions.
  pub fn finish(&mut self) -> Result<()> {
    self.finish_with_work_limit(Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)
  }

  /// Call this once you are done adding states/transitions.
  ///
  /// `determinize_work_limit` is the maximum effort to spend determinizing the automaton. Higher
  /// numbers allow this operation to consume more memory but allow more complex automatons. Use
  /// [`Operations::DEFAULT_DETERMINIZE_WORK_LIMIT`](crate::core::util::automation::operations::Operations::DEFAULT_DETERMINIZE_WORK_LIMIT) as a decent default if you don't otherwise know
  /// what to specify.
  pub fn finish_with_work_limit(&mut self, determinize_work_limit: usize) -> Result<()> {
    let mut automaton = self.builder.finish()?;

    // println!("before det:\n{}", automaton.to_dot()?);

    let mut transition = Transition::default();

    if self.any_term_id != -1 {
      // Make sure there are no leading or trailing ANY:
      let count = automaton.init_transition(0, &mut transition);
      for _ in 0..count {
        automaton.get_next_transition(&mut transition);
        if self.any_term_id >= transition.min && self.any_term_id <= transition.max {
          return Err(LuceneError::illegal_state(
            "automaton cannot lead with an ANY transition",
          ));
        }
      }

      let num_states = automaton.get_num_states();
      for state in 0..num_states {
        let count = automaton.init_transition(state, &mut transition);
        for _ in 0..count {
          automaton.get_next_transition(&mut transition);
          if automaton.is_accept(transition.dest)
            && self.any_term_id >= transition.min
            && self.any_term_id <= transition.max
          {
            return Err(LuceneError::illegal_state(
              "automaton cannot end with an ANY transition",
            ));
          }
        }
      }

      let term_count = self.id_to_term.len() as i32;

      // We have to carefully translate these transitions so automaton
      // realizes they also match all other terms:
      let mut new_automaton = Automaton::new();
      for state in 0..num_states {
        new_automaton.create_state()?;
        new_automaton.set_accept(state, automaton.is_accept(state));
      }

      for state in 0..num_states {
        let count = automaton.init_transition(state, &mut transition);
        for _ in 0..count {
          automaton.get_next_transition(&mut transition);
          let (min, max) =
            if transition.min <= self.any_term_id && self.any_term_id <= transition.max {
              // Match any term
              (0, term_count - 1)
            } else {
              (transition.min, transition.max)
            };
          new_automaton.add_transition(transition.source, transition.dest, min, max)?;
        }
      }
      new_automaton.finish_state()?;
      automaton = new_automaton;
    }

    let deterministic = Operations::determinize(&automaton, determinize_work_limit)?.into_owned();
    let det = Operations::remove_dead_states(&deterministic)?.into_owned();

    if det.is_accept(0) {
      return Err(LuceneError::illegal_state("cannot accept the empty string"));
    }
    self.det = Some(det);
    Ok(())
  }

  fn get_term_id(&mut self, term: Option<&BytesRef<Vec<u8>>>) -> Result<i32> {
    let Some(term) = term else {
      if self.any_term_id == -1 {
        self.any_term_id = self.id_to_term.len() as i32;
        self.id_to_term.push(None);
      }
      return Ok(self.any_term_id);
    };

    let bytes = &term.bytes[term.offset..term.offset + term.length];
    if let Some(id) = self.term_to_id.get(bytes) {
      return Ok(*id);
    }

    let id = self.id_to_term.len() as i32;
    self.term_to_id.insert(bytes.to_vec(), id);
    self.id_to_term.push(Some(BytesRef::deep_copy_of(term)?));
    Ok(id)
  }

  fn check_finished(&self) {
    assert!(
      self.det.is_some(),
      "Call finish first on: {}",
      self
        .to_string(&self.field)
        .unwrap_or_else(|_| "TermAutomatonQuery".to_string())
    );
  }

  /// Returns the dot (graphviz) representation of this automaton. This is extremely useful for
  /// visualizing the automaton.
  pub fn to_dot(&self) -> Result<String> {
    let det = self
      .det
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("call finish first"))?;
    let mut builder = String::new();
    builder.push_str("digraph Automaton {\n");
    builder.push_str("  rankdir = LR\n");
    let num_states = det.get_num_states();
    if num_states > 0 {
      builder.push_str("  initial [shape=plaintext,label=\"0\"]\n");
      builder.push_str("  initial -> 0\n");
    }

    let mut transition = Transition::default();
    for state in 0..num_states {
      builder.push_str("  ");
      builder.push_str(&state.to_string());
      if det.is_accept(state) {
        builder.push_str(" [shape=doublecircle,label=\"");
      } else {
        builder.push_str(" [shape=circle,label=\"");
      }
      builder.push_str(&state.to_string());
      builder.push_str("\"]\n");
      let count = det.init_transition(state, &mut transition);
      for _ in 0..count {
        det.get_next_transition(&mut transition);
        debug_assert!(transition.max >= transition.min);
        for term_id in transition.min..=transition.max {
          builder.push_str("  ");
          builder.push_str(&state.to_string());
          builder.push_str(" -> ");
          builder.push_str(&transition.dest.to_string());
          builder.push_str(" [label=\"");
          if term_id == self.any_term_id {
            builder.push('*');
          } else {
            let term = self.id_to_term[term_id as usize]
              .as_ref()
              .ok_or_else(|| LuceneError::illegal_state("term id has no term"))?;
            builder.push_str(&term.utf8_to_string()?);
          }
          builder.push_str("\"]\n");
        }
      }
    }
    builder.push('}');
    Ok(builder)
  }
}

impl Debug for TermAutomatonQuery {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
    match self.to_string("") {
      Ok(value) => formatter.write_str(&value),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl HasIdentity for TermAutomatonQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl PartialEq for TermAutomatonQuery {
  fn eq(&self, other: &Self) -> bool {
    self.check_finished();
    other.check_finished();
    self.id == other.id
  }
}

impl Eq for TermAutomatonQuery {}

impl Hash for TermAutomatonQuery {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.check_finished();
    self.id.hash(state);
  }
}

impl Accountable for TermAutomatonQuery {
  fn ram_bytes_used(&self) -> Result<i64> {
    let mut size = self
      .id
      .ram_bytes_used()?
      .saturating_add(self.builder.ram_bytes_used()?)
      .saturating_add(size_of_string(&self.field))
      .saturating_add(size_of_hash_map(&self.term_to_id))
      .saturating_add(size_of_vec(&self.id_to_term));
    if let Some(det) = &self.det {
      size = size.saturating_add(det.ram_bytes_used()?);
    }
    for term in self.term_to_id.keys() {
      size = size.saturating_add(term.capacity() as i64);
    }
    for term in self.id_to_term.iter().flatten() {
      size = size.saturating_add(term.bytes.capacity() as i64);
    }
    Ok(size)
  }
}

pub trait TermAutomatonQueryBase {
  fn rewrite<IRC>(&self, query: TermAutomatonQuery, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TermAutomatonQueryDefault;

impl TermAutomatonQueryBase for TermAutomatonQueryDefault {
  fn rewrite<IRC>(&self, query: TermAutomatonQuery, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
  {
    TermAutomatonQueryDefaults::rewrite(query, searcher)
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum TermAutomatonQueryHook {
  Default(TermAutomatonQueryDefault),
  #[cfg(test)]
  Custom(CustomTermAutomatonQuery),
}

impl TermAutomatonQueryBase for TermAutomatonQueryHook {
  fn rewrite<IRC>(&self, query: TermAutomatonQuery, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
  {
    match self {
      Self::Default(hook) => hook.rewrite(query, searcher),
      #[cfg(test)]
      Self::Custom(hook) => hook.rewrite(query, searcher),
    }
  }
}

pub struct TermAutomatonQueryDefaults;

impl TermAutomatonQueryDefaults {
  pub fn rewrite<IRC>(query: TermAutomatonQuery, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
  {
    let det = query
      .det
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("Call finish first"))?;
    if Operations::is_empty(det) {
      return Ok(MatchNoDocsQuery::new().into());
    }

    if let Some(single) = Operations::get_singleton(det)?
      && single.length == 1
    {
      let term_id = single.ints[single.offset] as usize;
      let term = query.id_to_term[term_id]
        .as_ref()
        .ok_or_else(|| LuceneError::illegal_state("singleton term is ANY"))?;
      return Ok(TermQuery::new(Term::new(query.field, term.clone())).into());
    }

    // Try for either PhraseQuery or MultiPhraseQuery, which only works when the automaton is a
    // sausage:
    let mut multi_phrase_builder = Some(multi_phrase_query::Builder::new());
    let mut phrase_builder = Some(phrase_query::Builder::new());

    let mut transition = Transition::default();
    let mut state = 0;
    let mut position = 0;
    'query: loop {
      let count = det.init_transition(state, &mut transition);
      if count == 0 {
        if !det.is_accept(state) {
          multi_phrase_builder = None;
          phrase_builder = None;
        }
        break;
      } else if det.is_accept(state) {
        multi_phrase_builder = None;
        phrase_builder = None;
        break;
      }
      let mut dest = -1;
      let mut ranges = Vec::new();
      let mut matches_any = false;
      for transition_index in 0..count {
        det.get_next_transition(&mut transition);
        if transition_index == 0 {
          dest = transition.dest;
        } else if dest != transition.dest {
          multi_phrase_builder = None;
          phrase_builder = None;
          break 'query;
        }

        matches_any |= query.any_term_id >= transition.min && query.any_term_id <= transition.max;
        ranges.push((transition.min, transition.max));
      }
      if !matches_any {
        let mut terms = Vec::new();
        for (min, max) in ranges {
          for term_id in min..=max {
            let bytes = query.id_to_term[term_id as usize]
              .as_ref()
              .ok_or_else(|| LuceneError::illegal_state("term id has no term"))?;
            terms.push(Term::new(query.field.clone(), bytes.clone()));
          }
        }
        multi_phrase_builder
          .as_mut()
          .ok_or_else(|| LuceneError::illegal_state("multi-phrase builder is missing"))?
          .add_terms_with_position(&terms, position)?;
        if let Some(builder) = &mut phrase_builder {
          if terms.len() == 1 {
            builder.add(terms[0].clone(), position as usize)?;
          } else {
            phrase_builder = None;
          }
        }
      }
      state = dest;
      position += 1;
    }

    if let Some(builder) = phrase_builder {
      return Ok(builder.build()?.into());
    }
    if let Some(builder) = multi_phrase_builder {
      return Ok(builder.build().into());
    }

    Ok(query.into())
  }
}

impl QueryBase for TermAutomatonQuery {
  fn to_string(&self, _field: &str) -> Result<String> {
    let mut value = format!("TermAutomatonQuery(field={}", self.field);
    if let Some(det) = &self.det {
      value.push_str(" numStates=");
      value.push_str(&det.get_num_states().to_string());
    }
    value.push(')');
    Ok(value)
  }

  fn create_weight<IRC>(
    self,
    searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
  {
    Ok(Box::new(TermAutomatonWeight::new(
      self, searcher, score_mode, boost,
    )?))
  }

  fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
  {
    let hook = self.hook.clone();
    hook.rewrite(self, searcher)
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
    let terms = self
      .id_to_term
      .iter()
      .flatten()
      .map(|term| Term::new(self.field.clone(), term.clone()))
      .collect::<Vec<_>>();
    visitor.consume_terms(query, &terms)
  }
}

struct TermAutomatonWeight {
  automaton: Automaton,
  term_states: Vec<Option<Mutex<TermStates>>>,
  stats: Option<Arc<SimilarityEnumSimScorer>>,
  #[allow(dead_code)]
  // Mirrors Java's retained similarity field, which is only read during construction.
  similarity: Arc<SimilarityEnum>,
  parent_query: Arc<Query>,
}

impl Debug for TermAutomatonWeight {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
    match self.parent_query.to_string("") {
      Ok(value) => write!(formatter, "weight({value})"),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl TermAutomatonWeight {
  fn new<IRC>(
    query: TermAutomatonQuery,
    searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<Self>
  where
    IRC: IndexReaderContext,
  {
    let automaton = query
      .det
      .clone()
      .ok_or_else(|| LuceneError::illegal_state("Call finish first"))?;
    let mut term_states = Vec::with_capacity(query.id_to_term.len());
    let mut all_term_stats = Vec::<TermStatistics>::new();
    for term in &query.id_to_term {
      if let Some(term) = term {
        let index_term = Arc::new(Term::new(query.field.clone(), term.clone()));
        let states = build(searcher, index_term.clone(), score_mode.needs_scores())?;
        if states.doc_freq()? > 0 {
          all_term_stats.push(searcher.term_statistics(
            index_term,
            states.doc_freq()?,
            states.total_term_freq()?,
          )?);
        }
        term_states.push(Some(Mutex::new(states)));
      } else {
        term_states.push(None);
      }
    }

    let similarity = searcher.get_similarity();
    let stats = if all_term_stats.is_empty() {
      None // no terms matched at all, will not use sim
    } else {
      let collection_stats = searcher
        .collection_statistics(&query.field)?
        .ok_or_else(|| LuceneError::illegal_state("collection statistics are missing"))?;
      Some(Arc::new(similarity.scorer(
        boost,
        &collection_stats,
        &all_term_stats,
      )?))
    };

    Ok(Self {
      automaton,
      term_states,
      stats,
      similarity,
      parent_query: Arc::new(query.into()),
    })
  }

  fn query(&self) -> Result<&TermAutomatonQuery> {
    match self.parent_query.as_ref() {
      Query::TermAutomaton(query) => Ok(query),
      _ => Err(LuceneError::illegal_state(
        "TermAutomatonWeight has the wrong parent query",
      )),
    }
  }

  #[allow(clippy::type_complexity)] // Preserve the concrete generic scorer types without erasure.
  fn build_scorer<LR>(
    &self,
    context: &LeafReaderContext<LR>,
  ) -> Result<
    Option<TermAutomatonScorer<LRPosting<LR>, SimilarityEnumSimScorer, LRNormNumericDocValues<LR>>>,
  >
  where
    LR: LeafReader,
  {
    let query = self.query()?;
    let field_terms = match context.reader().terms(&query.field)? {
      Some(terms) => terms,
      None => return Ok(None),
    };
    let mut subs = Vec::new();
    for (term_id, states) in self.term_states.iter().enumerate() {
      let Some(states) = states else {
        continue;
      };
      debug_assert!(
        states
          .lock()
          .was_built_for(ReaderUtil::get_top_level_context(context)),
        "The top-reader used to create Weight is not the same as the current reader's top-reader"
      );
      let mut states = states.lock();
      let mut supplier = states.get(context)?;
      let state = match supplier {
        Some(ref mut supplier) => states.resolve(supplier)?,
        None => None,
      };
      let Some(state) = state else {
        continue;
      };
      let term = query.id_to_term[term_id]
        .as_ref()
        .ok_or_else(|| LuceneError::illegal_state("term states exist for ANY"))?;
      let mut terms_enum = field_terms.iterator()?;
      terms_enum.seek_exact_with_state(term, state.as_ref())?;
      subs.push(EnumAndScorer::new(
        term_id as i32,
        terms_enum.postings_with_flags(None, POSITIONS as i32)?,
      ));
    }

    if subs.is_empty() {
      return Ok(None);
    }
    let norms = context.reader().get_norm_values(&query.field)?;
    let stats = self
      .stats
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("similarity scorer is missing"))?
      .clone();
    Ok(Some(TermAutomatonScorer::new(
      self.automaton.clone(),
      subs,
      query.id_to_term.len(),
      query.any_term_id,
      stats,
      norms,
    )?))
  }
}

impl<IRC> SegmentCacheable<IRC> for TermAutomatonWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    Ok(true)
  }
}

impl<IRC> Weight<IRC> for TermAutomatonWeight
where
  IRC: IndexReaderContext,
{
  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    let Some(mut scorer) = self.build_scorer(context)? else {
      return Ok(Explanation::no_match_no_details(
        "No matching terms in the document",
      ));
    };

    let advanced_doc = scorer.iterator_mut().advance(doc)?;
    if advanced_doc != doc {
      return Ok(Explanation::no_match_no_details(
        "No matching terms in the document",
      ));
    }

    let score = scorer.score()?;
    let query = self.query()?;

    let mut norms = context.reader().get_norm_values(&query.field)?;
    let mut norm = 1i64;
    if let Some(norms) = &mut norms
      && norms.advance_exact(doc)?
    {
      norm = norms.long_value()?;
    }

    let stats = self
      .stats
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("similarity scorer is missing"))?;
    let mut term_explanations = Vec::new();
    for sub in scorer.original_subs_on_doc() {
      if sub.pos_enum.doc_id() == doc {
        let frequency = sub.pos_enum.freq()?;
        let term_score = stats.score(frequency as f32, norm);
        let term = query.id_to_term[sub.term_id as usize]
          .as_ref()
          .ok_or_else(|| LuceneError::illegal_state("term id has no term"))?;
        term_explanations.push(Explanation::match_(
          frequency,
          "term frequency in the document",
          vec![Explanation::match_no_details(
            term_score,
            format!("score for term: {}", term.utf8_to_string()?),
          )],
        ));
      }
    }

    if term_explanations.is_empty() {
      return Ok(Explanation::no_match_no_details(
        "No matching terms in the document",
      ));
    }

    let freq_explanation =
      Explanation::match_(score, "TermAutomatonQuery, sum of:", term_explanations);
    stats.explain(freq_explanation, norm)
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
    let Some(scorer) = self.build_scorer(context)? else {
      return Ok(None);
    };
    Ok(Some(Box::new(DefaultScorerSupplier::new(scorer))))
  }
}

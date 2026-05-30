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
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::{LRPosting, LeafReader};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::index::slow_impacts_enum::SlowImpactsEnum;
use crate::core::index::term::Term;
use crate::core::index::term_states::{TermStates, build};
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::search::exact_phrase_matcher::ExactPhraseMatcher;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::phrase_matcher::PhraseMatcherEnum;
use crate::core::search::phrase_query::{PostingsAndFreq, term_positions_cost};
use crate::core::search::phrase_weight::{
  PhraseWeight, PhraseWeightBase, PhraseWeightMeta, SimScorerImpl, SimScorerType,
};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::sloppy_phrase_matcher::SloppyPhraseMatcher;
use crate::core::search::term_query::TermQuery;
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// A generalized version of `PhraseQuery`, with the possibility of adding
/// more than one term at the same position that are treated as a disjunction (OR).
#[derive(Debug, Clone)]
pub struct MultiPhraseQuery {
  id: Identity,
  slop: i32,
  field: String,
  term_arrays: Arc<Vec<Vec<Term>>>,
  positions: Arc<Vec<i32>>,
}

impl PartialEq for MultiPhraseQuery {
  fn eq(&self, other: &Self) -> bool {
    self.slop == other.slop
      && term_arrays_equals(&self.term_arrays, &other.term_arrays)
      && self.positions.as_ref() == other.positions.as_ref()
  }
}

impl Eq for MultiPhraseQuery {}

impl Hash for MultiPhraseQuery {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.slop.hash(state);
    for term_array in self.term_arrays.iter() {
      term_array.hash(state);
    }
    self.positions.hash(state);
  }
}

fn term_arrays_equals(a: &[Vec<Term>], b: &[Vec<Term>]) -> bool {
  if a.len() != b.len() {
    return false;
  }
  for i in 0..a.len() {
    if a[i] != b[i] {
      return false;
    }
  }
  true
}

impl MultiPhraseQuery {
  pub fn get_slop(&self) -> i32 {
    self.slop
  }

  pub fn get_term_arrays(&self) -> &Vec<Vec<Term>> {
    &self.term_arrays
  }

  pub fn get_positions(&self) -> &Vec<i32> {
    &self.positions
  }

  pub fn builder() -> Builder {
    Builder::new()
  }
}

impl HasIdentity for MultiPhraseQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

/// A builder for multi-phrase queries.
#[derive(Debug, Clone)]
pub struct Builder {
  field: Option<String>,
  term_arrays: Vec<Vec<Term>>,
  positions: Vec<i32>,
  slop: i32,
}

impl Builder {
  pub fn new() -> Self {
    Builder {
      field: None,
      term_arrays: Vec::new(),
      positions: Vec::new(),
      slop: 0,
    }
  }

  pub fn from_query(query: &MultiPhraseQuery) -> Self {
    Builder {
      field: Some(query.field.clone()),
      term_arrays: query.term_arrays.to_vec(),
      positions: query.positions.to_vec(),
      slop: query.slop,
    }
  }

  pub fn set_slop(&mut self, s: i32) -> &mut Self {
    if s < 0 {
      panic!("slop value cannot be negative");
    }
    self.slop = s;
    self
  }

  pub fn add_term(self, term: Term) -> Self {
    self.add_terms(&[term])
  }

  pub fn add_terms(self, terms: &[Term]) -> Self {
    let position = if self.positions.is_empty() {
      0
    } else {
      self.positions[self.positions.len() - 1] + 1
    };
    self.add_terms_at(terms, position)
  }

  pub fn add_terms_at(mut self, terms: &[Term], position: i32) -> Self {
    assert!(!terms.is_empty(), "Term array must not be null");
    if self.term_arrays.is_empty() {
      self.field = Some(terms[0].field().to_string());
    }
    for term in terms {
      if term.field() != self.field.as_ref().unwrap() {
        panic!(
          "All phrase terms must be in the same field ({}): {}",
          self.field.as_ref().unwrap(),
          term
        );
      }
    }
    self.term_arrays.push(terms.to_vec());
    self.positions.push(position);
    self
  }

  pub fn build(self) -> MultiPhraseQuery {
    let field = self.field.unwrap_or_default();
    MultiPhraseQuery {
      id: Identity::new(),
      slop: self.slop,
      field,
      term_arrays: Arc::new(self.term_arrays),
      positions: Arc::new(self.positions),
    }
  }
}

impl QueryBase for MultiPhraseQuery {
  fn as_string(&self, f: &str) -> Result<String> {
    let mut buffer = String::new();
    if self.field != f {
      buffer.push_str(&self.field);
      buffer.push(':');
    }
    buffer.push('"');
    let mut last_pos: i32 = -1;
    for i in 0..self.term_arrays.len() {
      let terms = &self.term_arrays[i];
      let position = self.positions[i];
      if i != 0 {
        buffer.push(' ');
        for _j in 1..(position - last_pos) {
          buffer.push_str("? ");
        }
      }
      if terms.len() > 1 {
        buffer.push('(');
        for (j, term) in terms.iter().enumerate() {
          if j > 0 {
            buffer.push(' ');
          }
          buffer.push_str(&term.text().unwrap_or_else(|_| "None".to_string()));
        }
        buffer.push(')');
      } else {
        buffer.push_str(&terms[0].text().unwrap_or_else(|_| "None".to_string()));
      }
      last_pos = position;
    }
    buffer.push('"');
    if self.slop != 0 {
      buffer.push('~');
      buffer.push_str(&self.slop.to_string());
    }
    Ok(buffer)
  }

  fn create_weight<IRC>(
    self,
    searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    let similarity = searcher.get_similarity();
    let field = self.field.clone();
    let base = PhraseWeightMeta::new(field, *score_mode, similarity, self.clone().into());
    let sub = MultiPhraseQueryWeightBase::new(self, boost, base);
    let weight = PhraseWeight::new(searcher, sub)?;
    Ok(Box::new(weight))
  }

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    if self.term_arrays.is_empty() {
      Ok(MatchNoDocsQuery::with_reason("empty MultiPhraseQuery").into())
    } else if self.term_arrays.len() == 1 {
      let mut builder = BooleanQueryBuilder::new();
      for term in &self.term_arrays[0] {
        builder.add(TermQuery::new(term.clone()), Occur::Should)?;
      }
      Ok(builder.build().into())
    } else {
      Ok(self.into())
    }
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
    todo!()
  }
}

pub struct MultiPhraseQueryWeightBase {
  query: Arc<MultiPhraseQuery>,
  term_states: Mutex<HashMap<Term, TermStates>>,
  boost: f32,
  base: PhraseWeightMeta,
}

impl MultiPhraseQueryWeightBase {
  pub(crate) fn new(query: MultiPhraseQuery, boost: f32, base: PhraseWeightMeta) -> Self {
    Self {
      query: Arc::new(query),
      term_states: Mutex::new(HashMap::new()),
      boost,
      base,
    }
  }
}

impl PhraseWeightBase for MultiPhraseQueryWeightBase {
  type SimScorer = Arc<SimScorerType>;
  type IE<LR: LeafReader> = SlowImpactsEnum<UnionPostingsEnum<LRPosting<LR>>>;

  fn get_stats<IRC>(&mut self, searcher: &IndexSearcher<IRC>) -> Result<Self::SimScorer>
  where
    IRC: IndexReaderContext,
  {
    let mut all_term_stats = Vec::new();

    for terms in &*self.query.term_arrays {
      for term in terms {
        let mut ts_map = self.term_states.lock();
        if !ts_map.contains_key(term) {
          let ts = build(
            searcher,
            Arc::new(term.clone()),
            self.base.score_mode.needs_scores(),
          )?;
          ts_map.insert(term.clone(), ts);
        }
        if self.base.score_mode.needs_scores() {
          let ts = ts_map.get(term).unwrap();
          if ts.doc_freq()? > 0 {
            let stats = searcher.term_statistics(
              Arc::new(term.clone()),
              ts.doc_freq()?,
              ts.total_term_freq()?,
            )?;
            all_term_stats.push(stats);
          }
        }
      }
    }

    if all_term_stats.is_empty() {
      Ok(Arc::new(SimScorerType::B(SimScorerImpl)))
    } else {
      let collection_stats = searcher
        .collection_statistics(&self.base.field)?
        .ok_or_else(|| LuceneError::illegal_state("could not get collection stats"))?;
      Ok(Arc::new(SimScorerType::A(self.base.similarity.scorer(
        self.boost,
        &collection_stats,
        &all_term_stats,
      )?)))
    }
  }

  fn get_phrase_matcher<LR>(
    &self,
    context: &LeafReaderContext<LR>,
    scorer: Self::SimScorer,
    expose_offsets: bool,
  ) -> Result<Option<PhraseMatcherEnum<Self::IE<LR>, Self::SimScorer>>>
  where
    LR: LeafReader,
  {
    debug_assert!(!self.query.term_arrays.is_empty());
    let reader = context.reader();

    let field_terms = match reader.terms(&self.base.field)? {
      Some(t) => t,
      None => return Ok(None),
    };

    if !field_terms.has_positions() {
      return Err(LuceneError::illegal_state(format!(
        "field \"{}\" was indexed without position data; cannot run MultiPhraseQuery (phrase={})",
        self.base.field,
        self.query.as_string(&self.base.field)?
      )));
    }

    let mut te = field_terms.iterator()?;
    let mut total_match_cost: f32 = 0.0;

    let mut postings_freqs: Vec<PostingsAndFreq<Self::IE<LR>>> =
      Vec::with_capacity(self.query.term_arrays.len());

    let postings_flags = if expose_offsets {
      crate::core::index::postings_enum::ALL as i32
    } else {
      crate::core::index::postings_enum::POSITIONS as i32
    };

    let ts_map = self.term_states.lock();

    for pos in 0..self.query.term_arrays.len() {
      let terms = &self.query.term_arrays[pos];
      let mut posting_enums: Vec<LRPosting<LR>> = Vec::new();

      for term in terms {
        let mut ts = match ts_map.get(term) {
          Some(ts) => ts.clone(),
          None => continue,
        };

        let mut supplier = ts.get(context)?;
        let state = match supplier {
          None => None,
          Some(ref mut s) => ts.resolve(s)?,
        };

        let state = match state {
          None => continue,
          Some(s) => s,
        };

        te.seek_exact_with_state(term.bytes(), state.as_ref())?;

        let pe = te.postings_with_flags(None, postings_flags)?;
        posting_enums.push(pe);
        total_match_cost += term_positions_cost(&mut te)?;
      }

      if posting_enums.is_empty() {
        return Ok(None);
      }

      let union_pe = UnionPostingsEnum::new(posting_enums);
      let ie = SlowImpactsEnum::new(union_pe);

      postings_freqs.push(PostingsAndFreq::new(ie, pos, terms));
    }

    let v = if self.query.slop == 0 {
      postings_freqs.sort();
      PhraseMatcherEnum::Exact(ExactPhraseMatcher::new(
        postings_freqs,
        self.base.score_mode,
        scorer,
        total_match_cost,
      )?)
    } else {
      PhraseMatcherEnum::Sloppy(SloppyPhraseMatcher::new(
        postings_freqs,
        self.query.slop as usize,
        scorer,
        total_match_cost,
        expose_offsets,
      )?)
    };
    Ok(Some(v))
  }

  fn base(&self) -> &PhraseWeightMeta {
    &self.base
  }
}

/// Takes the logical union of multiple PostingsEnum iterators.
///
/// Note: positions are merged during freq()
pub struct UnionPostingsEnum<P>
where
  P: PostingsEnum,
{
  subs: Vec<P>,
  pos_queue: PositionsQueue,
  pos_queue_doc: i32,
}

impl<P> UnionPostingsEnum<P>
where
  P: PostingsEnum,
{
  pub fn new(subs: Vec<P>) -> Self {
    UnionPostingsEnum {
      subs,
      pos_queue: PositionsQueue::new(),
      pos_queue_doc: -2,
    }
  }
}

impl<P> DocIdSetIterator for UnionPostingsEnum<P>
where
  P: PostingsEnum,
{
  fn doc_id(&self) -> i32 {
    self
      .subs
      .iter()
      .map(|s| s.doc_id())
      .min()
      .unwrap_or(NO_MORE_DOCS)
  }

  fn next_doc(&mut self) -> Result<i32> {
    let current_doc = self
      .subs
      .iter()
      .map(|s| s.doc_id())
      .min()
      .unwrap_or(NO_MORE_DOCS);
    if current_doc == NO_MORE_DOCS {
      return Ok(NO_MORE_DOCS);
    }
    for sub in &mut self.subs {
      if sub.doc_id() == current_doc {
        sub.next_doc()?;
      }
    }
    Ok(
      self
        .subs
        .iter()
        .map(|s| s.doc_id())
        .min()
        .unwrap_or(NO_MORE_DOCS),
    )
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    for sub in &mut self.subs {
      if sub.doc_id() < target {
        sub.advance(target)?;
      }
    }
    Ok(
      self
        .subs
        .iter()
        .map(|s| s.doc_id())
        .min()
        .unwrap_or(NO_MORE_DOCS),
    )
  }

  fn cost(&self) -> Result<i64> {
    let mut sum: i64 = 0;
    for sub in &self.subs {
      sum += sub.cost()?;
    }
    Ok(sum)
  }
}

impl<P> PostingsEnum for UnionPostingsEnum<P>
where
  P: PostingsEnum,
{
  fn freq(&mut self) -> Result<i32> {
    let doc = self.doc_id();
    if doc != self.pos_queue_doc {
      self.pos_queue.clear();
      for sub in &mut self.subs {
        if sub.doc_id() == doc {
          let freq = sub.freq()?;
          for _ in 0..freq {
            let pos = sub.next_position()?;
            self.pos_queue.add(pos);
          }
        }
      }
      self.pos_queue.sort();
      self.pos_queue_doc = doc;
    }
    Ok(self.pos_queue.size() as i32)
  }

  fn next_position(&mut self) -> Result<i32> {
    Ok(self.pos_queue.next())
  }

  fn start_offset(&self) -> Result<i32> {
    Ok(-1)
  }

  fn end_offset(&self) -> Result<i32> {
    Ok(-1)
  }

  fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(None)
  }
}

/// Queue of terms for a single document.
/// It's a sorted array of all the positions from all the postings.
struct PositionsQueue {
  array: Vec<i32>,
  index: usize,
}

impl PositionsQueue {
  fn new() -> Self {
    PositionsQueue {
      array: Vec::with_capacity(16),
      index: 0,
    }
  }

  fn add(&mut self, i: i32) {
    self.array.push(i);
  }

  fn next(&mut self) -> i32 {
    let val = self.array[self.index];
    self.index += 1;
    val
  }

  fn sort(&mut self) {
    self.array.sort();
  }

  fn clear(&mut self) {
    self.index = 0;
    self.array.clear();
  }

  fn size(&self) -> usize {
    self.array.len()
  }
}

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
use crate::core::index::postings_enum::{PostingsEnum, PostingsEnumEnum2};
use crate::core::index::slow_impacts_enum::SlowImpactsEnum;
use crate::core::index::term::Term;
use crate::core::index::term_states::{TermStates, build};
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
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
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};
use crate::core::util::{HasIdentity, SliceCopyOps};
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

impl Default for Builder {
  fn default() -> Self {
    Self::new()
  }
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

  pub fn set_slop(&mut self, s: i32) -> Result<&mut Self> {
    if s < 0 {
      return Err(LuceneError::illegal_argument(
        "slop value cannot be negative",
      ));
    }
    self.slop = s;
    Ok(self)
  }

  pub fn add_term(&mut self, term: Term) -> Result<&mut Self> {
    self.add_terms(&[term])
  }

  pub fn add_terms(&mut self, terms: &[Term]) -> Result<&mut Self> {
    let position = if self.positions.is_empty() {
      0
    } else {
      self.positions[self.positions.len() - 1] + 1
    };
    self.add_terms_with_position(terms, position)
  }

  pub fn add_terms_with_position(&mut self, terms: &[Term], position: i32) -> Result<&mut Self> {
    if self.term_arrays.is_empty() {
      let first_term = terms
        .first()
        .ok_or_else(|| LuceneError::array_index_out_of_bounds("Term array must not be empty"))?;
      self.field = Some(first_term.field().to_string());
    }
    let field = self
      .field
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("field is not set"))?;
    for term in terms {
      if term.field() != field {
        return Err(LuceneError::illegal_argument(format!(
          "All phrase terms must be in the same field ({}): {}",
          field, term
        )));
      }
    }
    self.term_arrays.push(terms.to_vec());
    self.positions.push(position);
    Ok(self)
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
  fn to_string(&self, f: &str) -> Result<String> {
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

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    if !visitor.accept_field(&self.field) {
      return Ok(());
    }
    let query = self.into();
    let mut visitor = visitor.get_sub_visitor(Occur::Must, query);
    for terms in self.term_arrays.iter() {
      let mut sub_visitor = visitor.get_sub_visitor(Occur::Should, query);
      sub_visitor.consume_terms(query, terms)?;
    }
    Ok(())
  }
}

pub struct MultiPhraseQueryWeightBase {
  query: Arc<MultiPhraseQuery>,
  term_states: HashMap<Term, TermStates>,
  boost: f32,
  base: PhraseWeightMeta,
}

impl MultiPhraseQueryWeightBase {
  pub(crate) fn new(query: MultiPhraseQuery, boost: f32, base: PhraseWeightMeta) -> Self {
    Self {
      query: Arc::new(query),
      term_states: HashMap::new(),
      boost,
      base,
    }
  }
}

impl PhraseWeightBase for MultiPhraseQueryWeightBase {
  type SimScorer = Arc<SimScorerType>;
  type IE<LR: LeafReader> =
    SlowImpactsEnum<PostingsEnumEnum2<LRPosting<LR>, UnionPE<LRPosting<LR>>>>;

  fn get_stats<IRC>(&mut self, searcher: &IndexSearcher<IRC>) -> Result<Self::SimScorer>
  where
    IRC: IndexReaderContext,
  {
    let mut all_term_stats = Vec::new();

    for terms in &*self.query.term_arrays {
      for term in terms {
        if !self.term_states.contains_key(term) {
          let ts = build(
            searcher,
            Arc::new(term.clone()),
            self.base.score_mode.needs_scores(),
          )?;
          self.term_states.insert(term.clone(), ts);
        }
        if self.base.score_mode.needs_scores() {
          let ts = self
            .term_states
            .get(term)
            .ok_or_else(|| LuceneError::illegal_state("term state should have been built"))?;
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
        self.query.to_string(&self.base.field)?
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

    for pos in 0..self.query.term_arrays.len() {
      let terms = &self.query.term_arrays[pos];
      let mut posting_enums: Vec<LRPosting<LR>> = Vec::new();

      for term in terms {
        let mut ts = match self.term_states.get(term) {
          Some(ts) => ts.clone(),
          None => continue,
        };

        let mut supplier = ts.get(context)?;
        let term_state = match supplier {
          None => None,
          Some(ref mut s) => ts.resolve(s)?,
        };

        let terms_state = match term_state {
          None => continue,
          Some(s) => s,
        };

        te.seek_exact_with_state(term.bytes(), terms_state.as_ref())?;

        let pe = te.postings_with_flags(None, postings_flags)?;
        posting_enums.push(pe);
        total_match_cost += term_positions_cost(&mut te)?;
      }

      if posting_enums.is_empty() {
        return Ok(None);
      }

      let postings_enum = if posting_enums.len() == 1 {
        PostingsEnumEnum2::A(posting_enums.remove(0))
      } else {
        let union_pe = if expose_offsets {
          PostingsEnumEnum2::A(UnionFullPostingsEnum::new(posting_enums)?)
        } else {
          PostingsEnumEnum2::B(UnionPostingsEnum::new(posting_enums)?)
        };
        PostingsEnumEnum2::B(union_pe)
      };

      postings_freqs.push(PostingsAndFreq::new(
        SlowImpactsEnum::new(postings_enum),
        self.query.positions[pos],
        terms,
      ));
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
  docs_queue: PriorityQueue<usize, DocsQueueCmp<P>>,
  cost: i64,
  pos_queue: PositionsQueue,
  pos_queue_doc: i32,
}

impl<P> UnionPostingsEnum<P>
where
  P: PostingsEnum,
{
  pub fn new(subs: Vec<P>) -> Result<Self> {
    // subs should never be empty
    if subs.is_empty() {
      return Err(LuceneError::illegal_argument("subs must not be empty"));
    }
    let size = subs.len();
    let mut cost = 0;
    let cmp = DocsQueueCmp::new(subs);
    let mut docs_queue = PriorityQueue::new(size, cmp)?;
    for pe in docs_queue.compare.subs.iter() {
      cost += pe.cost()?;
    }
    for i in 0..size {
      docs_queue.add(i)?;
    }
    Ok(UnionPostingsEnum {
      docs_queue,
      cost,
      pos_queue: PositionsQueue::new(),
      pos_queue_doc: -2,
    })
  }
}

impl<P> DocIdSetIterator for UnionPostingsEnum<P>
where
  P: PostingsEnum,
{
  fn doc_id(&self) -> i32 {
    // docs_queue is nerver empty or pop so it is safe to unwrap
    let index = self.docs_queue.top().expect("docs_queue is never empty");
    self.docs_queue.compare.subs[*index].doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    let mut top = *self
      .docs_queue
      .top()
      .ok_or_else(|| LuceneError::illegal_state("docs_queue is never empty"))?;
    let doc = self.docs_queue.compare.subs[top].doc_id();
    loop {
      self.docs_queue.compare.subs[top].next_doc()?;
      top = *self.docs_queue.update_top()?;
      if self.docs_queue.compare.subs[top].doc_id() != doc {
        return Ok(self.docs_queue.compare.subs[top].doc_id());
      }
    }
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    let mut top = *self
      .docs_queue
      .top()
      .ok_or_else(|| LuceneError::illegal_state("docs_queue is never empty"))?;
    loop {
      self.docs_queue.compare.subs[top].advance(target)?;
      top = *self.docs_queue.update_top()?;
      let doc = self.docs_queue.compare.subs[top].doc_id();
      if doc >= target {
        return Ok(doc);
      }
    }
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.cost)
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
      for sub in &mut self.docs_queue.compare.subs {
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

struct DocsQueueCmp<P>
where
  P: PostingsEnum,
{
  subs: Vec<P>,
}
impl<P> DocsQueueCmp<P>
where
  P: PostingsEnum,
{
  pub fn new(subs: Vec<P>) -> Self {
    Self { subs }
  }
}
impl<P> Compare<usize> for DocsQueueCmp<P>
where
  P: PostingsEnum,
{
  fn less_than(&self, a: &usize, b: &usize) -> Result<bool> {
    let a_doc = self.subs[*a].doc_id();
    let b_doc = self.subs[*b].doc_id();
    Ok(a_doc < b_doc)
  }
}

/// Queue of terms for a single document.
/// It's a sorted array of all the positions from all the postings.
struct PositionsQueue {
  array_size: usize,
  index: usize,
  size: usize,
  array: Vec<i32>,
}

impl PositionsQueue {
  fn new() -> Self {
    let array_size = 16;
    Self {
      array_size,
      index: 0,
      size: 0,
      array: vec![0; array_size],
    }
  }

  fn add(&mut self, i: i32) {
    if self.size == self.array_size {
      self.grow_array();
    }

    self.array[self.size] = i;
    self.size += 1;
  }

  fn next(&mut self) -> i32 {
    let val = self.array[self.index];
    self.index += 1;
    val
  }

  fn sort(&mut self) {
    self.array[self.index..self.size].sort();
  }

  fn clear(&mut self) {
    self.index = 0;
    self.size = 0;
  }

  fn size(&self) -> usize {
    self.size
  }

  fn grow_array(&mut self) {
    let mut new_array = vec![0; self.array_size * 2];
    new_array.copy_from(&self.array[..self.array_size], 0);
    self.array = new_array;
    self.array_size *= 2;
  }
}
#[derive(Clone)]
struct PostingsAndPosition {
  pe: usize,
  pos: i32,
  upto: i32,
}

impl PostingsAndPosition {
  fn new(pe: usize) -> Self {
    Self {
      pe,
      pos: 0,
      upto: 0,
    }
  }
}

struct PosQueueCmp;

impl Compare<PostingsAndPosition> for PosQueueCmp {
  fn less_than(&self, a: &PostingsAndPosition, b: &PostingsAndPosition) -> Result<bool> {
    Ok(a.pos < b.pos)
  }
}

/// Slower version of UnionPostingsEnum that delegates offsets and positions.
pub struct UnionFullPostingsEnum<P>
where
  P: PostingsEnum,
{
  base: UnionPostingsEnum<P>,
  freq: i32,
  started: bool,
  pos_queue: PriorityQueue<PostingsAndPosition, PosQueueCmp>,
  subs: Vec<PostingsAndPosition>,
}

impl<P> UnionFullPostingsEnum<P>
where
  P: PostingsEnum,
{
  pub fn new(subs: Vec<P>) -> Result<Self> {
    let size = subs.len();

    let base = UnionPostingsEnum::new(subs)?;

    let mut postings = Vec::with_capacity(size);
    for i in 0..size {
      postings.push(PostingsAndPosition::new(i));
    }

    Ok(Self {
      base,
      freq: -1,
      started: false,
      pos_queue: PriorityQueue::new(size, PosQueueCmp)?,
      subs: postings,
    })
  }
}

impl<P> DocIdSetIterator for UnionFullPostingsEnum<P>
where
  P: PostingsEnum,
{
  fn doc_id(&self) -> i32 {
    self.base.doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.base.next_doc()
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.base.advance(target)
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.base.advance(target)
  }

  fn cost(&self) -> Result<i64> {
    self.base.cost()
  }
}

impl<P> PostingsEnum for UnionFullPostingsEnum<P>
where
  P: PostingsEnum,
{
  fn freq(&mut self) -> Result<i32> {
    let doc = self.doc_id();
    if doc == self.base.pos_queue_doc {
      return Ok(self.freq);
    }

    self.freq = 0;
    self.started = false;
    self.pos_queue.clear();
    for pp in &mut self.subs {
      let pe = &mut self.base.docs_queue.compare.subs[pp.pe];
      if pe.doc_id() == doc {
        pp.pos = pe.next_position()?;
        pp.upto = pe.freq()?;
        self.pos_queue.add(pp.clone())?;
        self.freq += pp.upto;
      }
    }
    Ok(self.freq)
  }

  fn next_position(&mut self) -> Result<i32> {
    if !self.started {
      self.started = true;
      let top = self
        .pos_queue
        .top()
        .ok_or_else(|| LuceneError::illegal_state("pos_queue is empty"))?;
      return Ok(top.pos);
    }

    let top = self
      .pos_queue
      .top_mut()
      .ok_or_else(|| LuceneError::illegal_state("pos_queue is empty"))?;

    if top.upto == 1 {
      self.pos_queue.pop()?;
      let top = self
        .pos_queue
        .top()
        .ok_or_else(|| LuceneError::illegal_state("pos_queue is empty"))?;
      return Ok(top.pos);
    }

    top.pos = self.base.docs_queue.compare.subs[top.pe].next_position()?;
    top.upto -= 1;
    self.pos_queue.update_top()?;

    let top = self
      .pos_queue
      .top()
      .ok_or_else(|| LuceneError::illegal_state("pos_queue is empty"))?;
    Ok(top.pos)
  }

  fn start_offset(&self) -> Result<i32> {
    let top = self
      .pos_queue
      .top()
      .ok_or_else(|| LuceneError::illegal_state("pos_queue is empty"))?;
    self.base.docs_queue.compare.subs[top.pe].start_offset()
  }

  fn end_offset(&self) -> Result<i32> {
    let top = self
      .pos_queue
      .top()
      .ok_or_else(|| LuceneError::illegal_state("pos_queue is empty"))?;
    self.base.docs_queue.compare.subs[top.pe].end_offset()
  }
  fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    let top = self
      .pos_queue
      .top()
      .ok_or_else(|| LuceneError::illegal_state("pos_queue is empty"))?;
    self.base.docs_queue.compare.subs[top.pe].get_payload()
  }
}
pub type UnionPE<P> = PostingsEnumEnum2<UnionFullPostingsEnum<P>, UnionPostingsEnum<P>>;

impl crate::core::util::accountable::Accountable for MultiPhraseQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}

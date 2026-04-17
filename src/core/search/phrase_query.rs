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
use crate::core::index::impacts_enum::{ImpactsEnum, ImpactsEnumEnum2};
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::postings_enum::{OFFSETS, POSITIONS};
use crate::core::index::slow_impacts_enum::SlowImpactsEnum;
use crate::core::index::term::Term;
use crate::core::index::term_states::{TermStates, build};
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::exact_phrase_matcher::ExactPhraseMatcher;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::phrase_matcher::{DefaultPhraseMatcherEnum, PhraseMatcherEnum};
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
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PhraseQuery {
  id: Identity,
  slop: usize,
  terms: Arc<Vec<Term>>,
  positions: Arc<Vec<usize>>,
  field: Option<String>,
}
impl PhraseQuery {
  /// Create a phrase query which will match documents that contain the given
  /// list of terms at consecutive positions in `field`, and at a maximum edit
  /// distance of `slop`.
  ///
  /// For more complicated use-cases, use [PhraseQuery::builder](Builder).
  ///
  /// # See also
  ///
  /// - [`PhraseQuery::get_slop`]
  pub fn from_terms(slop: usize, field: &str, terms: &[&str]) -> Result<Self> {
    let terms = to_terms(field, terms);
    let positions = incremental_positions(terms.len());
    PhraseQuery::new(slop, terms, positions)
  }

  /// Create a phrase query which will match documents that contain the given
  /// list of terms at consecutive positions in `field`.
  pub fn from_terms_no_slop(field: &str, terms: &[&str]) -> Result<Self> {
    Self::from_terms(0, field, terms)
  }

  /// Create a phrase query which will match documents that contain the given
  /// list of terms at consecutive positions in `field`, and at a maximum edit
  /// distance of `slop`.
  ///
  /// For more complicated use-cases, use [`PhraseQuery::builder`](Builder).
  ///
  /// # See also
  ///
  /// - [`PhraseQuery::get_slop`]
  pub fn from_bytes(slop: usize, field: &str, terms: Vec<BytesRef<Vec<u8>>>) -> Result<Self> {
    let terms = to_terms_from_bytes(field, terms);
    let positions = incremental_positions(terms.len());
    PhraseQuery::new(slop, terms, positions)
  }

  /// Create a phrase query which will match documents that contain the given
  /// list of terms at consecutive positions in `field`.
  pub fn from_bytes_no_slop(field: &str, terms: Vec<BytesRef<Vec<u8>>>) -> Result<Self> {
    Self::from_bytes(0, field, terms)
  }

  /// Return the slop for this `PhraseQuery`.
  ///
  /// The slop is an edit distance between the respective positions of terms as
  /// defined in this `PhraseQuery` and the actual positions of these terms in
  /// a document.
  ///
  /// For instance, when searching for `"quick fox"`, it is expected that the
  /// difference between the positions of `fox` and `quick` is `1`. So
  /// `"a quick brown fox"` would be at an edit distance of `1` since the
  /// difference of the positions of `fox` and `quick` is `2`. Similarly,
  /// `"the fox is quick"` would be at an edit distance of `3` since the
  /// difference of the positions of `fox` and `quick` is `-2`.
  ///
  /// The slop defines the maximum edit distance for a document to match this
  /// phrase query.
  ///
  /// More exact matches are scored higher than sloppier matches, so search
  /// results are ordered by exactness.
  pub fn get_slop(&self) -> usize {
    self.slop
  }

  /// Returns the field this query applies to.
  ///
  /// If the query contains no terms, this returns `None`. Otherwise, it
  /// returns the field shared by all terms in this phrase query.
  pub fn get_field(&self) -> Option<&str> {
    self.field.as_deref()
  }

  /// Returns the list of terms in this phrase.
  ///
  /// The returned slice preserves the order in which terms were added to the
  /// phrase query. All terms are guaranteed to belong to the same field.
  pub fn get_terms(&self) -> &[Term] {
    &self.terms
  }

  /// Returns the relative positions of terms in this phrase.
  ///
  /// The returned slice has the same length as [`get_terms`](Self::get_terms),
  /// and each position corresponds to the term at the same index.
  pub fn get_positions(&self) -> &[usize] {
    &self.positions
  }

  fn new(slop: usize, terms: Vec<Term>, positions: Vec<usize>) -> Result<Self> {
    if terms.len() != positions.len() {
      return Err(LuceneError::illegal_argument(
        "Must have as many terms as positions".to_string(),
      ));
    }
    if terms.len() > 1 {
      let field = terms[0].field();
      for term in &terms[1..] {
        if term.field() != field {
          return Err(LuceneError::illegal_argument(
            "All terms should have the same field".to_string(),
          ));
        }
      }
    }

    for i in 1..positions.len() {
      if positions[i] < positions[i - 1] {
        return Err(LuceneError::illegal_argument(format!(
          "Positions should not go backwards, got {} before {}",
          positions[i - 1],
          positions[i]
        )));
      }
    }

    let field = terms.first().map(|t| t.field().to_string());

    Ok(Self {
      id: Identity::new(),
      slop,
      terms: Arc::new(terms),
      positions: Arc::new(positions),
      field,
    })
  }
}

impl Hash for PhraseQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.slop.hash(state);
    self.terms.hash(state);
    self.positions.hash(state);
  }
}
impl Eq for PhraseQuery {}
impl PartialEq for PhraseQuery {
  fn eq(&self, other: &Self) -> bool {
    self.slop == other.slop && self.terms == other.terms && self.positions == other.positions
  }
}

impl HasIdentity for PhraseQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for PhraseQuery {
  fn as_string(&self, f: &str) -> Result<String> {
    let mut buffer = String::new();

    if let Some(field) = &self.field
      && field != f
    {
      buffer.push_str(field);
      buffer.push(':');
    }

    buffer.push('"');

    let max_position = self.positions.last().copied();

    let mut pieces: Vec<Option<String>> = match max_position {
      None => Vec::new(),
      Some(max) => vec![None; max + 1],
    };

    for (term, &pos) in self.terms.iter().zip(self.positions.iter()) {
      let text = term.text().unwrap_or_else(|_| "None".to_string());
      match &mut pieces[pos] {
        None => {
          pieces[pos] = Some(text.to_string());
        },
        Some(existing) => {
          existing.push('|');
          existing.push_str(&text);
        },
      }
    }

    for (i, piece) in pieces.iter().enumerate() {
      if i > 0 {
        buffer.push(' ');
      }
      match piece {
        None => buffer.push('?'),
        Some(s) => buffer.push_str(s),
      }
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
    let query = self.clone();
    let field = self
      .field
      .clone()
      .ok_or_else(|| LuceneError::illegal_state("field is None"))?;
    let base = PhraseWeightMeta::new(field, *score_mode, similarity, query.into());
    let sub = PhraseQueryWeightBase::new(self, boost, base);
    let weight = PhraseWeight::new(searcher, sub)?;
    Ok(Box::new(weight))
  }

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    let len = self.terms.len();
    if len == 0 {
      Ok(MatchNoDocsQuery::with_reason("empty PhraseQuery").into())
    } else if len == 1 {
      Ok(TermQuery::new(self.terms[0].clone()).into())
    } else if let Some(&first_pos) = self.positions.first() {
      if first_pos != 0 {
        let mut new_positions = Vec::with_capacity(self.positions.len());
        for &p in self.positions.iter() {
          new_positions.push(p - first_pos);
        }
        Ok(PhraseQuery::new(self.slop, (*self.terms).clone(), new_positions)?.into())
      } else {
        Ok(self.into())
      }
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

/// A builder for phrase queries
pub struct Builder {
  slop: usize,
  terms: Vec<Term>,
  positions: Vec<usize>,
}
#[cfg(test)]
impl Clone for Builder {
  fn clone(&self) -> Self {
    Self {
      slop: self.slop,
      terms: self.terms.clone(),
      positions: self.positions.clone(),
    }
  }
}

impl Default for Builder {
  fn default() -> Self {
    Self::new()
  }
}
impl Builder {
  pub fn new() -> Self {
    Self {
      slop: 0,
      terms: Vec::new(),
      positions: Vec::new(),
    }
  }
  pub fn set_slop(&mut self, slop: usize) -> &mut Self {
    self.slop = slop;
    self
  }
  /// Adds a term to the end of the query phrase.
  ///
  /// The relative position of the term is the one immediately after the last
  /// term added.
  pub fn add_term(&mut self, term: Term) -> Result<&mut Self> {
    let position = match self.positions.last() {
      None => 0,
      Some(&last) => last + 1,
    };
    self.add(term, position)
  }
  /// Adds a term to the end of the query phrase.
  ///
  /// The relative position of the term within the phrase is specified explicitly,
  /// but must be greater than or equal to that of the previously added term.
  /// A greater position allows phrases with gaps (e.g. in connection with
  /// stopwords).
  ///
  /// If the position is equal, you most likely should be using
  /// `MultiPhraseQuery` instead, which only requires one term at each position
  /// to match; this class requires all of them.
  pub fn add(&mut self, term: Term, position: usize) -> Result<&mut Self> {
    if let Some(&last_position) = self.positions.last()
      && position < last_position
    {
      return Err(LuceneError::illegal_argument(format!(
        "Positions must be added in order, got {} after {}",
        position, last_position
      )))?;
    }

    if let Some(first_term) = self.terms.first()
      && term.field() != first_term.field()
    {
      return Err(LuceneError::illegal_argument(format!(
        "All terms must be on the same field, got {} and {}",
        term.field(),
        first_term.field()
      )))?;
    }
    self.terms.push(term);
    self.positions.push(position);
    Ok(self)
  }
  /// Build a phrase query based on the terms that have been added.
  pub fn build(self) -> Result<PhraseQuery> {
    PhraseQuery::new(self.slop, self.terms, self.positions)
  }
}

fn incremental_positions(length: usize) -> Vec<usize> {
  (0..length).collect()
}

fn to_terms(field: &str, term_strings: &[&str]) -> Vec<Term> {
  let mut terms = Vec::with_capacity(term_strings.len());
  for &s in term_strings {
    terms.push(Term::from_text(field, s));
  }
  terms
}

fn to_terms_from_bytes(field: &str, term_bytes: Vec<BytesRef<Vec<u8>>>) -> Vec<Term> {
  let mut terms = Vec::with_capacity(term_bytes.len());
  for b in term_bytes {
    terms.push(Term::new(field, b));
  }
  terms
}
/// A guess of the average number of simple operations for the initial seek and buffer refill per
/// document for the positions of a term. See also
/// [`Lucene101PostingsReader::BlockPostingsEnum::next_position`](crate::core::codecs::lucene101::lucene101_postings_reader::BlockPostingsEnum::next_position).
///
/// Aside: Instead of being constant this could depend among others on
/// [`Lucene101PostingsFormat::BLOCK_SIZE`](crate::core::codecs::lucene101::lucene101_postings_format::Lucene101PostingsFormat::BLOCK_SIZE), [`TermsEnum::doc_freq`], [`TermsEnum::total_term_freq`],
/// [`DocIdSetIterator::cost`](crate::core::search::doc_id_set_iterator::DocIdSetIterator::cost) (expected number of matching docs), [`LeafReader::max_doc`] (total
/// number of docs in the segment), and the seek time and block size of the device storing the
/// index.
pub(crate) const TERM_POSNS_SEEK_OPS_PER_DOC: i32 = 128;

/// Number of simple operations in [`Lucene101PostingsReader::BlockPostingsEnum::next_position`](crate::core::codecs::lucene101::lucene101_postings_reader::BlockPostingsEnum::next_position)
/// when no seek or buffer refill is done.
pub(crate) const TERM_OPS_PER_POS: i32 = 7;

pub fn term_positions_cost<TE>(terms_enum: &mut TE) -> Result<f32>
where
  TE: TermsEnum,
{
  let doc_freq = terms_enum.doc_freq()?;
  debug_assert!(doc_freq > 0);

  let total_term_freq = terms_enum.total_term_freq()?;

  let exp_occurrences_in_matching_doc = total_term_freq as f32 / doc_freq as f32;

  Ok(TERM_POSNS_SEEK_OPS_PER_DOC as f32 + exp_occurrences_in_matching_doc * TERM_OPS_PER_POS as f32)
}
pub struct PhraseQueryWeightBase {
  query: Arc<PhraseQuery>,
  states: Vec<Mutex<TermStates>>,
  boost: f32,
  base: PhraseWeightMeta,
}
impl PhraseQueryWeightBase {
  pub(crate) fn new(query: PhraseQuery, boost: f32, base: PhraseWeightMeta) -> Self {
    Self {
      query: Arc::new(query),
      states: Vec::new(),
      boost,
      base,
    }
  }
  #[cfg(debug_assertions)]
  fn term_not_in_reader<LR>(reader: &LR, term: &Term) -> Result<bool>
  where
    LR: LeafReader,
  {
    Ok(LeafReader::doc_freq(reader, term)? == 0)
  }
}

impl PhraseWeightBase for PhraseQueryWeightBase {
  type SimScorer = Arc<SimScorerType>;

  fn get_stats<IRC>(&mut self, searcher: &IndexSearcher<IRC>) -> Result<Self::SimScorer>
  where
    IRC: IndexReaderContext,
  {
    let positions = &self.query.positions;

    if positions.len() < 2 {
      return Err(LuceneError::illegal_state(
        "PhraseWeight does not support less than 2 terms, call rewrite first",
      ));
    } else if positions[0] != 0 {
      return Err(LuceneError::illegal_state(
        "PhraseWeight requires that the first position is 0, call rewrite first",
      ));
    }

    self.states = Vec::with_capacity(self.query.terms.len());

    let mut term_stats = Vec::with_capacity(self.query.terms.len());
    let mut term_up_to = 0usize;

    for term in &*self.query.terms {
      let term = Arc::new(term.clone());
      let ts = build(searcher, term.clone(), self.base.score_mode.needs_scores())?;

      if self.base.score_mode.needs_scores() && ts.doc_freq()? > 0 {
        let stats =
          searcher.term_statistics(term.clone(), ts.doc_freq()?, ts.total_term_freq()?)?;
        term_stats.push(stats);
        term_up_to += 1;
      }

      self.states.push(Mutex::new(ts));
    }

    let v = if term_up_to > 0 {
      let collection_stats = searcher
        .collection_statistics(&self.base.field)?
        .ok_or_else(|| LuceneError::illegal_state("could not get collection stats"))?;

      SimScorerType::A(self.base.similarity.scorer(
        self.boost,
        &collection_stats,
        term_stats[..term_up_to].as_ref(),
      )?)
    } else {
      // no terms at all, we won't use similarity
      SimScorerType::B(SimScorerImpl)
    };
    Ok(Arc::new(v))
  }

  fn get_phrase_matcher<LR>(
    &self,
    context: &LeafReaderContext<LR>,
    scorer: Self::SimScorer,
    expose_offsets: bool,
  ) -> Result<Option<DefaultPhraseMatcherEnum<LR, Self::SimScorer>>>
  where
    LR: LeafReader,
  {
    debug_assert!(!self.query.terms.is_empty());
    let reader = context.reader();

    let field_terms = match reader.terms(&self.base.field)? {
      Some(t) => t,
      None => return Ok(None),
    };

    if !field_terms.has_positions() {
      return Err(LuceneError::illegal_state(format!(
        "field \"{}\" was indexed without position data; cannot run PhraseQuery (phrase={})",
        self.base.field,
        self.query.as_string(&self.base.field)?
      )));
    }

    let mut te = field_terms.iterator()?;
    let mut total_match_cost: f32 = 0.0;

    let mut postings_freqs = Vec::with_capacity(self.query.terms.len());

    for i in 0..self.query.terms.len() {
      let t = &self.query.terms[i];

      let mut supplier = self.states[i].lock().get(context)?;
      let state = match supplier {
        None => None,
        Some(ref mut s) => self.states[i].lock().resolve(s)?,
      };

      let state = match state {
        None => {
          #[cfg(debug_assertions)]
          {
            debug_assert!(
              Self::term_not_in_reader(reader, t)?,
              "no termstate found but term exists in reader"
            );
          }
          return Ok(None);
        },
        Some(s) => s,
      };

      te.seek_exact_with_state(t.bytes(), state.as_ref())?;

      let impacts_enum = if self.base.score_mode == ScoreMode::TopScores {
        let impacts = te.impacts(if expose_offsets {
          OFFSETS as i32
        } else {
          POSITIONS as i32
        })?;
        ImpactsEnumEnum2::A(impacts)
      } else {
        let postings = te.postings_with_flags(
          None,
          if expose_offsets {
            OFFSETS as i32
          } else {
            POSITIONS as i32
          },
        )?;
        ImpactsEnumEnum2::B(SlowImpactsEnum::new(postings))
      };

      postings_freqs.push(PostingsAndFreq::new(
        impacts_enum,
        self.query.positions[i],
        std::slice::from_ref(t),
      ));

      total_match_cost += term_positions_cost(&mut te)?;
    }

    // sort by increasing docFreq order
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
        self.query.slop,
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

pub struct PostingsAndFreq<IE>
where
  IE: ImpactsEnum,
{
  pub(crate) postings: IE,
  pub(crate) position: usize,
  pub(crate) terms: Option<Vec<Term>>,
  pub(crate) n_terms: usize, // for faster comparisons
}
impl<IE> PostingsAndFreq<IE>
where
  IE: ImpactsEnum,
{
  pub fn new(postings: IE, position: usize, terms: &[Term]) -> Self {
    let n_terms = terms.len();

    let terms_vec = if n_terms == 0 {
      None
    } else if n_terms == 1 {
      Some(vec![terms[0].clone()])
    } else {
      let mut v = terms.to_vec();
      v.sort();
      Some(v)
    };

    Self {
      postings,
      position,
      terms: terms_vec,
      n_terms,
    }
  }
}
impl<IE> Ord for PostingsAndFreq<IE>
where
  IE: ImpactsEnum,
{
  fn cmp(&self, other: &Self) -> Ordering {
    match self.position.cmp(&other.position) {
      Ordering::Equal => {},
      ord => return ord,
    }

    match self.n_terms.cmp(&other.n_terms) {
      Ordering::Equal => {},
      ord => return ord,
    }

    if self.n_terms == 0 {
      return Ordering::Equal;
    }

    let a = self.terms.as_ref().unwrap();
    let b = other.terms.as_ref().unwrap();

    for i in 0..a.len() {
      let ord = a[i].cmp(&b[i]);
      if ord != Ordering::Equal {
        return ord;
      }
    }

    Ordering::Equal
  }
}

impl<IE> PartialOrd for PostingsAndFreq<IE>
where
  IE: ImpactsEnum,
{
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}
impl<IE> PartialEq for PostingsAndFreq<IE>
where
  IE: ImpactsEnum,
{
  fn eq(&self, other: &Self) -> bool {
    if self.position != other.position {
      return false;
    }

    match (&self.terms, &other.terms) {
      (None, None) => true,
      (Some(a), Some(b)) => a == b,
      _ => false,
    }
  }
}

impl<IE> Eq for PostingsAndFreq<IE> where IE: ImpactsEnum {}

#[cfg(test)]
mod tests {
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::index::BytesRef;
  use crate::core::index::impact::Impact;
  use crate::core::index::impacts::Impacts;
  use crate::core::index::impacts_enum::ImpactsEnum;
  use crate::core::index::impacts_source::ImpactsSource;
  use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
  use crate::core::index::postings_enum::PostingsEnum;
  use crate::core::index::term::Term;
  use crate::core::search::boolean_clause::Occur;
  use crate::core::search::boolean_query::Builder;
  use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
  use crate::core::search::exact_phrase_matcher::merge_impacts_from_ie;
  use crate::core::search::phrase_query::PhraseQuery;
  use crate::core::search::query::{Query, QueryBase};
  use crate::core::search::similarities_impl::classic_similarity::ClassicSimilarity;
  use crate::core::search::term_query::TermQuery;
  use crate::core::search::top_docs::TopDocsLike;
  use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
  use crate::core::util::error::lucene_error::{LuceneError, Result};
  use crate::test::core::index::random_index_writer::RandomIndexWriter;
  use crate::test::core::search::check_hits::CheckHits;
  use crate::test::core::search::query_utils::QueryUtils;
  use crate::test::core::util::DefaultIndexSearchCR;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_index_writer_config, new_log_merge_policy, new_searcher_with_reader,
    new_text_field, random,
  };
  use rand::Rng;
  use rand::prelude::SliceRandom;
  use std::borrow::Cow;
  use std::collections::HashMap;
  use std::rc::Rc;

  #[allow(dead_code)]
  struct TestPhraseQuery;
  pub const SCORE_COMP_THRESH: f32 = 1e-6;

  fn before_class<R>(random: &mut R) -> Result<DefaultIndexSearchCR>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    // TODO IMPORTANT 这里需要自定义分词器
    let writer = RandomIndexWriter::new(random, dir.clone());
    let mut field_to_type = HashMap::new();
    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      "field",
      "one two three four five",
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(new_text_field(
      random,
      "repeated",
      "this is a repeated field - first part",
      Store::Yes,
      &mut field_to_type,
    )?);
    let repeated_field = new_text_field(
      random,
      "repeated",
      "second part of a repeated field",
      Store::Yes,
      &mut field_to_type,
    )?;
    doc.add(repeated_field);
    doc.add(new_text_field(
      random,
      "palindrome",
      "one two three two one",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      "nonexist",
      "phrase exist notexist exist found",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      "nonexist",
      "phrase exist notexist exist found",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;

    let reader = writer.get_reader()?;
    writer.close()?;

    let searcher = new_searcher_with_reader(reader)?;

    Ok(searcher)
  }
  #[test]
  fn test_not_close_enough() -> Result<()> {
    let mut random = random();
    let searcher = before_class(&mut random)?;
    let query = PhraseQuery::from_terms(2, "field", &["one", "five"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(0, hits.len());
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    Ok(())
  }

  #[test]
  fn test_barely_close_enough() -> Result<()> {
    let mut random = random();
    let searcher = before_class(&mut random)?;
    let query = PhraseQuery::from_terms(3, "field", &["one", "five"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len());

    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;
    Ok(())
  }

  /// Ensures slop of 0 works for exact matches, but not reversed
  #[test]
  fn test_exact() -> Result<()> {
    let mut random = random();
    let searcher = before_class(&mut random)?;
    // slop is zero by default
    let query = PhraseQuery::from_terms(0, "field", &["four", "five"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len(), "exact match");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    let query = PhraseQuery::from_terms(0, "field", &["two", "one"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(0, hits.len(), "reverse not exact");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    Ok(())
  }

  #[test]
  fn test_slop1() -> Result<()> {
    let mut random = random();
    let searcher = before_class(&mut random)?;

    // Ensures slop of 1 works with terms in order.
    let query = PhraseQuery::from_terms(1, "field", &["one", "two"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len(), "in order");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    // Ensures slop of 1 does not work for phrases out of order;
    // must be at least 2.
    let query = PhraseQuery::from_terms(1, "field", &["two", "one"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(0, hits.len(), "reversed, slop not 2 or more");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    Ok(())
  }

  /// As long as slop is at least 2, terms can be reversed
  #[test]
  fn test_order_doesnt_matter() -> Result<()> {
    let mut random = random();
    let searcher = before_class(&mut random)?;

    // must be at least two for reverse order match
    let query = PhraseQuery::from_terms(2, "field", &["two", "one"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len(), "just sloppy enough");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    let query = PhraseQuery::from_terms(2, "field", &["three", "one"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(0, hits.len(), "not sloppy enough");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    Ok(())
  }

  /// slop is the total number of positional moves allowed to line up a phrase
  #[test]
  fn test_multiple_terms() -> Result<()> {
    let mut random = random();
    let searcher = before_class(&mut random)?;

    let query = PhraseQuery::from_terms(2, "field", &["one", "three", "five"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len(), "two total moves");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    // it takes six moves to match this phrase
    let query = PhraseQuery::from_terms(5, "field", &["five", "three", "one"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(0, hits.len(), "slop of 5 not close enough");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    let query = PhraseQuery::from_terms(6, "field", &["five", "three", "one"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len(), "slop of 6 just right");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    Ok(())
  }
  #[test]
  fn test_phrase_query_with_stop_analyzer() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    // TODO  这里需要自定义分词器
    let writer = RandomIndexWriter::new(&mut random, dir.clone());
    let mut field_to_type = HashMap::new();

    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "field",
      "the stop words are here",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;

    let reader = writer.get_reader()?;
    writer.close()?;

    let searcher = new_searcher_with_reader(reader)?;

    // valid exact phrase query
    let query = PhraseQuery::from_terms(0, "field", &["stop", "words"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len());

    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    Ok(())
  }

  #[test]
  fn test_phrase_query_in_conjunction_scorer() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mut field_to_type = HashMap::new();
    {
      let writer = RandomIndexWriter::new(&mut random, dir.clone());

      let mut doc = Document::new();
      doc.add(new_text_field(
        &mut random,
        "source",
        "marketing info",
        Store::Yes,
        &mut field_to_type,
      )?);
      writer.add_document(doc)?;

      let mut doc = Document::new();
      doc.add(new_text_field(
        &mut random,
        "contents",
        "foobar",
        Store::Yes,
        &mut field_to_type,
      )?);
      doc.add(new_text_field(
        &mut random,
        "source",
        "marketing info",
        Store::Yes,
        &mut field_to_type,
      )?);
      writer.add_document(doc)?;

      let reader = writer.get_reader()?;
      writer.close()?;

      let searcher = new_searcher_with_reader(reader)?;

      let phrase_query = PhraseQuery::from_terms(0, "source", &["marketing", "info"])?;
      let top_docs = searcher.search(phrase_query.clone(), 1000)?;
      let hits = top_docs.score_docs();
      assert_eq!(2, hits.len());
      QueryUtils::check_from_searcher(&mut random, phrase_query.clone(), &searcher)?;

      let term_query: Query = TermQuery::new(Term::from_text("contents", "foobar")).into();

      let mut b = Builder::new();
      b.add(term_query.clone(), Occur::Must)?;
      b.add(phrase_query.clone(), Occur::Must)?;
      let boolean_query: Query = b.build().into();

      let top_docs = searcher.search(boolean_query, 1000)?;
      let hits = top_docs.score_docs();
      assert_eq!(1, hits.len());
      QueryUtils::check_from_searcher(&mut random, term_query, &searcher)?;
    }

    {
      let writer = RandomIndexWriter::new(&mut random, dir.clone());

      let mut doc = Document::new();
      doc.add(new_text_field(
        &mut random,
        "contents",
        "map entry woo",
        Store::Yes,
        &mut field_to_type,
      )?);
      writer.add_document(doc)?;

      let mut doc = Document::new();
      doc.add(new_text_field(
        &mut random,
        "contents",
        "woo map entry",
        Store::Yes,
        &mut field_to_type,
      )?);
      writer.add_document(doc)?;

      let mut doc = Document::new();
      doc.add(new_text_field(
        &mut random,
        "contents",
        "map foobarword entry woo",
        Store::Yes,
        &mut field_to_type,
      )?);
      writer.add_document(doc)?;

      let reader = writer.get_reader()?;
      writer.close()?;

      let searcher = new_searcher_with_reader(reader)?;

      let term_query: Query = TermQuery::new(Term::from_text("contents", "woo")).into();
      let phrase_query = PhraseQuery::from_terms(0, "contents", &["map", "entry"])?;

      let top_docs = searcher.search(term_query.clone(), 1000)?;
      let hits = top_docs.score_docs();
      assert_eq!(3, hits.len());

      let top_docs = searcher.search(phrase_query.clone(), 1000)?;
      let hits = top_docs.score_docs();
      assert_eq!(2, hits.len());

      let mut b = Builder::new();
      b.add(term_query.clone(), Occur::Must)?;
      b.add(phrase_query.clone(), Occur::Must)?;
      let boolean_query1: Query = b.build().into();
      let top_docs = searcher.search(boolean_query1, 1000)?;
      let hits = top_docs.score_docs();
      assert_eq!(2, hits.len());

      let mut b = Builder::new();
      b.add(phrase_query.clone(), Occur::Must)?;
      b.add(term_query.clone(), Occur::Must)?;
      let boolean_query2: Query = b.build().into();
      let top_docs = searcher.search(boolean_query2.clone(), 1000)?;
      let hits = top_docs.score_docs();
      assert_eq!(2, hits.len());

      QueryUtils::check_from_searcher(&mut random, boolean_query2, &searcher)?;
    }

    Ok(())
  }
  #[test]
  fn test_slop_scoring() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
    let writer = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

    let mut field_to_type = HashMap::new();

    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "field",
      "foo firstname lastname foo",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;

    let mut doc2 = Document::new();
    doc2.add(new_text_field(
      &mut random,
      "field",
      "foo firstname zzz lastname foo",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(doc2)?;

    let mut doc3 = Document::new();
    doc3.add(new_text_field(
      &mut random,
      "field",
      "foo firstname zzz yyy lastname foo",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(doc3)?;

    let reader = writer.get_reader()?;
    writer.close()?;

    let mut searcher = new_searcher_with_reader(reader)?;
    searcher.set_similarity(ClassicSimilarity::new());

    let query = PhraseQuery::from_terms(i32::MAX as usize, "field", &["firstname", "lastname"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(3, hits.len());

    assert!((hits[0].score - 1.0).abs() <= 0.01);
    assert_eq!(0, hits[0].doc);

    assert!((hits[1].score - 0.63).abs() <= 0.01);
    assert_eq!(1, hits[1].doc);

    assert!((hits[2].score - 0.47).abs() <= 0.01);
    assert_eq!(2, hits[2].doc);

    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;
    Ok(())
  }
  #[test]
  fn test_to_string() -> Result<()> {
    let q = PhraseQuery::from_terms(0, "field", &[])?;
    assert_eq!("\"\"", q.as_string("")?);

    // single term at position 1
    let mut builder = crate::core::search::phrase_query::Builder::new();
    builder.add(Term::from_text("field", "hi"), 1)?;
    let q = builder.build()?;
    assert_eq!("field:\"? hi\"", q.as_string("")?);

    // two terms with gap
    let mut builder = crate::core::search::phrase_query::Builder::new();
    builder.add(Term::from_text("field", "hi"), 1)?;
    builder.add(Term::from_text("field", "test"), 5)?;
    let q = builder.build()?;
    assert_eq!("field:\"? hi ? ? ? test\"", q.as_string("")?);

    // multi-term at same position
    let mut builder = crate::core::search::phrase_query::Builder::new();
    builder.add(Term::from_text("field", "hi"), 1)?;
    builder.add(Term::from_text("field", "hello"), 1)?;
    builder.add(Term::from_text("field", "test"), 5)?;
    let q = builder.build()?;
    assert_eq!("field:\"? hi|hello ? ? ? test\"", q.as_string("")?);

    // with slop
    let mut builder = crate::core::search::phrase_query::Builder::new();
    builder.add(Term::from_text("field", "hi"), 1)?;
    builder.add(Term::from_text("field", "hello"), 1)?;
    builder.add(Term::from_text("field", "test"), 5)?;
    builder.set_slop(5);
    let q = builder.build()?;
    assert_eq!("field:\"? hi|hello ? ? ? test\"~5", q.as_string("")?);

    Ok(())
  }
  #[test]
  fn test_wrapped_phrase() -> Result<()> {
    // TODO IMPORTANT 这里before_class中的自定义分词器 导致这个测试不能成功
    // let mut random = random();
    // let searcher = before_class(&mut random)?;
    //
    // let query = PhraseQuery::from_terms(
    //     100,
    //     "repeated",
    //     &["first", "part", "second", "part"],
    // )?;
    // let top_docs = searcher.search(query.clone(), 1000)?;
    // let hits = top_docs.score_docs();
    // assert_eq!(1, hits.len(), "slop of 100 just right");
    // QueryUtils::check_from_searcher(&mut random, query, &searcher)?;
    //
    // let query = PhraseQuery::from_terms(
    //     99,
    //     "repeated",
    //     &["first", "part", "second", "part"],
    // )?;
    // let top_docs = searcher.search(query.clone(), 1000)?;
    // let hits = top_docs.score_docs();
    // assert_eq!(0, hits.len(), "slop of 99 not enough");
    // QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    Ok(())
  }
  #[test]
  fn test_non_existing_phrase() -> Result<()> {
    let mut random = random();
    let searcher = before_class(&mut random)?;

    // phrase without repetitions that exists in 2 docs
    let query = PhraseQuery::from_terms(2, "nonexist", &["phrase", "notexist", "found"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(2, hits.len(), "phrase without repetitions exists in 2 docs");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    // phrase with repetitions that exists in 2 docs
    let query = PhraseQuery::from_terms(1, "nonexist", &["phrase", "exist", "exist"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(2, hits.len(), "phrase with repetitions exists in two docs");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    // phrase I with repetitions that does not exist in any doc
    let query = PhraseQuery::from_terms(1000, "nonexist", &["phrase", "notexist", "phrase"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(
      0,
      hits.len(),
      "nonexisting phrase with repetitions does not exist in any doc"
    );
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    // phrase II with repetitions that does not exist in any doc
    let query = PhraseQuery::from_terms(1000, "nonexist", &["phrase", "exist", "exist", "exist"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(
      0,
      hits.len(),
      "nonexisting phrase with repetitions does not exist in any doc"
    );
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    Ok(())
  }
  #[test]
  fn test_palyndrome2() -> Result<()> {
    let mut random = random();
    let searcher = before_class(&mut random)?;

    // search on non palyndrome, find phrase with no slop, using exact phrase scorer
    let query = PhraseQuery::from_terms(0, "field", &["two", "three"])?; // to use exact phrase scorer
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len(), "phrase found with exact phrase scorer");
    let score0 = hits[0].score;
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    // search on non palyndrome, find phrase with slop 2, though no slop required here.
    let query = PhraseQuery::from_terms(2, "field", &["two", "three"])?; // to use sloppy scorer
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len(), "just sloppy enough");
    let score1 = hits[0].score;
    assert!(
      (score0 - score1).abs() <= SCORE_COMP_THRESH,
      "exact scorer and sloppy scorer score the same when slop does not matter"
    );
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    // search ordered in palyndrome, find it twice
    let query = PhraseQuery::from_terms(2, "palindrome", &["two", "three"])?; // must be at least two for both ordered and reversed to match
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len(), "just sloppy enough");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    // search reveresed in palyndrome, find it twice
    let query = PhraseQuery::from_terms(2, "palindrome", &["three", "two"])?; // must be at least two for both ordered and reversed to match
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len(), "just sloppy enough");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    Ok(())
  }
  #[test]
  fn test_palyndrome3() -> Result<()> {
    let mut random = random();
    let searcher = before_class(&mut random)?;

    // search on non palyndrome, find phrase with no slop, using exact phrase scorer
    // slop=0 to use exact phrase scorer
    let query = PhraseQuery::from_terms(0, "field", &["one", "two", "three"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len(), "phrase found with exact phrase scorer");
    let score0 = hits[0].score;
    QueryUtils::check_from_searcher(&mut random, query.clone(), &searcher)?;

    // just make sure no exc:
    searcher.explain(query.clone(), 0)?;

    // search on non palyndrome, find phrase with slop 3, though no slop required here.
    // slop=4 to use sloppy scorer
    let query = PhraseQuery::from_terms(4, "field", &["one", "two", "three"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len(), "just sloppy enough");
    let score1 = hits[0].score;
    assert!(
      (score0 - score1).abs() <= SCORE_COMP_THRESH,
      "exact scorer and sloppy scorer score the same when slop does not matter"
    );
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    // search ordered in palyndrome, find it twice
    // slop must be at least four for both ordered and reversed to match
    let query = PhraseQuery::from_terms(4, "palindrome", &["one", "two", "three"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();

    // just make sure no exc:
    let _ = searcher.explain(query.clone(), 0)?;

    assert_eq!(1, hits.len(), "just sloppy enough");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    // search reveresed in palyndrome, find it twice
    // must be at least four for both ordered and reversed to match
    let query = PhraseQuery::from_terms(4, "palindrome", &["three", "two", "one"])?;
    let top_docs = searcher.search(query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len(), "just sloppy enough");
    QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

    Ok(())
  }
  #[test]
  fn test_empty_phrase_query() -> Result<()> {
    let mut b = Builder::new();
    b.add(PhraseQuery::from_terms(0, "field", &[])?, Occur::Must)?;
    let q: Query = b.build().into();
    let _ = q.as_string("");
    Ok(())
  }

  #[test]
  fn test_rewrite() -> Result<()> {
    let mut random = random();
    let searcher = before_class(&mut random)?;

    let pq: Query = PhraseQuery::from_terms(0, "foo", &["bar"])?.into();
    let rewritten = pq.rewrite(&searcher)?;

    assert!(matches!(rewritten, Query::Term(_)));
    Ok(())
  }
  #[test]
  fn test_zero_pos_incr() -> Result<()> {
    // TODO Token 未实现
    Ok(())
  }
  #[test]
  fn test_random_phrases() -> Result<()> {
    // TODO IMPORTANT
    Ok(())
  }
  #[test]
  fn test_negative_slop() {
    // this test is not required in Rust Lucene
  }
  #[test]
  fn test_negative_position() {
    // this test is not required in Rust Lucene
  }
  #[test]
  fn test_backward_positions() -> Result<()> {
    let mut builder = crate::core::search::phrase_query::Builder::new();
    builder.add(Term::from_text("field", "one"), 1)?;
    builder.add(Term::from_text("field", "two"), 5)?;

    let result = builder.add(Term::from_text("field", "three"), 4);

    assert!(result.is_err());
    Ok(())
  }
  static DOCS: [&str; 6] = [
    "a b c d e f g h",
    "b c b",
    "c d d d e f g b",
    "c b a b c",
    "a a b b c c d d",
    "a b c d a b c d a b c d",
  ];

  #[test]
  fn test_top_phrases() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let writer = RandomIndexWriter::new(&mut random, dir.clone());
    let mut field_to_type = HashMap::new();

    let mut docs = DOCS.to_vec();
    docs.shuffle(&mut random);

    for value in docs {
      let mut doc = Document::new();
      doc.add(new_text_field(
        &mut random,
        "f",
        value,
        Store::No,
        &mut field_to_type,
      )?);
      writer.add_document(doc)?;
    }

    let reader = writer.get_reader()?;
    writer.close()?;

    let searcher = new_searcher_with_reader(reader)?;

    let queries: Vec<Query> = vec![
      PhraseQuery::from_terms(0, "f", &["b", "c"])?.into(), // common phrase
      PhraseQuery::from_terms(0, "f", &["e", "f"])?.into(), // always appear next to each other
      PhraseQuery::from_terms(0, "f", &["d", "d"])?.into(), // repeated term
    ];

    for query in queries {
      for top_n in 1..=2 {
        let collector_manager = TopScoreDocCollectorManager::new(top_n, i32::MAX as usize)?;
        let top_docs1 =
          searcher.search_with_collector_manager(query.clone(), &collector_manager)?;
        let hits1 = top_docs1.score_docs();

        let collector_manager = TopScoreDocCollectorManager::new(top_n, 1)?;
        let top_docs2 =
          searcher.search_with_collector_manager(query.clone(), &collector_manager)?;
        let hits2 = top_docs2.score_docs();

        assert!(!hits1.is_empty(), "{}", query.as_string("")?);
        CheckHits::check_equal(&query, hits1, hits2)?;
      }
    }

    Ok(())
  }

  #[test]
  fn test_merge_impacts() -> Result<()> {
    let impacts1 = DummyImpactsEnum::new(1000);
    let impacts2 = DummyImpactsEnum::new(2000);

    let mut merged_impacts = merge_impacts_from_ie(vec![impacts1, impacts2])?;

    merged_impacts.impacts_enums.all_disi[0].reset(
      vec![
        vec![Impact::new(3, 10), Impact::new(5, 12), Impact::new(8, 13)],
        vec![
          Impact::new(3, 10),
          Impact::new(5, 11),
          Impact::new(8, 13),
          Impact::new(12, 14),
        ],
      ],
      vec![110, 945],
    );

    // Merge with empty impacts
    merged_impacts.impacts_enums.all_disi[1].reset(vec![], vec![]);
    assert_impacts_eq(
      vec![
        vec![Impact::new(3, 10), Impact::new(5, 12), Impact::new(8, 13)],
        vec![
          Impact::new(3, 10),
          Impact::new(5, 11),
          Impact::new(8, 13),
          Impact::new(12, 14),
        ],
      ],
      vec![110, 945],
      &merged_impacts.get_impacts()?,
    )?;

    // Merge with dummy impacts
    merged_impacts.impacts_enums.all_disi[1]
      .reset(vec![vec![Impact::new(i32::MAX, 1)]], vec![5000]);
    assert_impacts_eq(
      vec![
        vec![Impact::new(3, 10), Impact::new(5, 12), Impact::new(8, 13)],
        vec![
          Impact::new(3, 10),
          Impact::new(5, 11),
          Impact::new(8, 13),
          Impact::new(12, 14),
        ],
      ],
      vec![110, 945],
      &merged_impacts.get_impacts()?,
    )?;

    // Merge with dummy impacts that we don't special case
    merged_impacts.impacts_enums.all_disi[1]
      .reset(vec![vec![Impact::new(i32::MAX, 2)]], vec![5000]);
    assert_impacts_eq(
      vec![
        vec![Impact::new(3, 10), Impact::new(5, 12), Impact::new(8, 13)],
        vec![
          Impact::new(3, 10),
          Impact::new(5, 11),
          Impact::new(8, 13),
          Impact::new(12, 14),
        ],
      ],
      vec![110, 945],
      &merged_impacts.get_impacts()?,
    )?;

    // First level of impacts2 doesn't cover the first level of impacts1
    merged_impacts.impacts_enums.all_disi[1].reset(
      vec![
        vec![Impact::new(2, 10), Impact::new(6, 13)],
        vec![Impact::new(3, 9), Impact::new(5, 11), Impact::new(7, 13)],
      ],
      vec![90, 1000],
    );
    assert_impacts_eq(
      vec![
        vec![Impact::new(3, 10), Impact::new(5, 12), Impact::new(7, 13)],
        vec![Impact::new(3, 10), Impact::new(5, 11), Impact::new(7, 13)],
      ],
      vec![110, 945],
      &merged_impacts.get_impacts()?,
    )?;

    // First level of impacts2 doesn't cover the first level of impacts1
    merged_impacts.impacts_enums.all_disi[1].reset(
      vec![
        vec![Impact::new(2, 10), Impact::new(6, 11)],
        vec![Impact::new(3, 9), Impact::new(5, 11), Impact::new(7, 13)],
      ],
      vec![150, 900],
    );
    assert_impacts_eq(
      vec![
        vec![
          Impact::new(2, 10),
          Impact::new(3, 11),
          Impact::new(5, 12),
          Impact::new(6, 13),
        ],
        vec![
          Impact::new(3, 10),
          Impact::new(5, 11),
          Impact::new(8, 13),
          Impact::new(12, 14),
        ],
      ],
      vec![110, 945],
      &merged_impacts.get_impacts()?,
    )?;

    merged_impacts.impacts_enums.all_disi[1].reset(
      vec![
        vec![Impact::new(4, 10), Impact::new(9, 13)],
        vec![
          Impact::new(1, 1),
          Impact::new(4, 10),
          Impact::new(5, 11),
          Impact::new(8, 13),
          Impact::new(12, 14),
          Impact::new(13, 15),
        ],
      ],
      vec![113, 950],
    );
    assert_impacts_eq(
      vec![
        vec![Impact::new(3, 10), Impact::new(4, 12), Impact::new(8, 13)],
        vec![
          Impact::new(3, 10),
          Impact::new(5, 11),
          Impact::new(8, 13),
          Impact::new(12, 14),
        ],
      ],
      vec![110, 945],
      &merged_impacts.get_impacts()?,
    )?;

    // Make sure negative norms are treated as unsigned
    merged_impacts.impacts_enums.all_disi[0].reset(
      vec![
        vec![Impact::new(3, 10), Impact::new(5, -10), Impact::new(8, -5)],
        vec![
          Impact::new(3, 10),
          Impact::new(5, -15),
          Impact::new(8, -5),
          Impact::new(12, -3),
        ],
      ],
      vec![110, 945],
    );

    merged_impacts.impacts_enums.all_disi[1].reset(
      vec![
        vec![Impact::new(2, 10), Impact::new(12, -4)],
        vec![Impact::new(3, 9), Impact::new(12, -4), Impact::new(20, -1)],
      ],
      vec![150, 960],
    );

    assert_impacts_eq(
      vec![
        vec![Impact::new(2, 10), Impact::new(8, -4)],
        vec![Impact::new(3, 10), Impact::new(8, -4), Impact::new(12, -3)],
      ],
      vec![110, 945],
      &merged_impacts.get_impacts()?,
    )?;

    Ok(())
  }
  fn assert_impacts_eq(
    impacts: Vec<Vec<Impact>>,
    doc_id_upto: Vec<i32>,
    actual: &impl Impacts,
  ) -> Result<()> {
    assert_eq!(impacts.len(), actual.num_levels() as usize);

    for i in 0..impacts.len() {
      assert_eq!(doc_id_upto[i], actual.get_doc_id_upto(i as i32));

      let actual_impacts = actual.get_impacts(i as i32)?;
      let expect = impacts[i].as_slice();
      assert_eq!(expect, actual_impacts.as_slice());
    }
    Ok(())
  }

  struct DummyImpactsEnum {
    cost: i64,
    impacts: Rc<Vec<Vec<Impact>>>,
    doc_id_upto: Rc<Vec<i32>>,
  }
  impl DummyImpactsEnum {
    fn new(cost: i64) -> Self {
      Self {
        cost,
        impacts: Rc::new(vec![vec![]]),
        doc_id_upto: Rc::new(vec![]),
      }
    }

    fn reset(&mut self, impacts: Vec<Vec<Impact>>, doc_id_upto: Vec<i32>) {
      self.impacts = Rc::new(impacts);
      self.doc_id_upto = Rc::new(doc_id_upto);
    }
  }

  impl PostingsEnum for DummyImpactsEnum {
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

  impl DocIdSetIterator for DummyImpactsEnum {
    fn doc_id(&self) -> i32 {
      unreachable!("")
    }

    fn next_doc(&mut self) -> Result<i32> {
      Err(LuceneError::unsupported_operation(""))
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
      Err(LuceneError::unsupported_operation(""))
    }

    fn cost(&self) -> Result<i64> {
      Ok(self.cost)
    }
  }

  impl ImpactsSource for DummyImpactsEnum {
    fn advance_shallow(&mut self, _target: i32) -> Result<()> {
      Err(LuceneError::unsupported_operation(""))
    }

    type Impacts<'a>
      = ImpactsImpl
    where
      Self: 'a;

    fn get_impacts(&self) -> Result<Self::Impacts<'_>> {
      Ok(ImpactsImpl::new(
        self.impacts.clone(),
        self.doc_id_upto.clone(),
      ))
    }
  }

  impl ImpactsEnum for DummyImpactsEnum {}
  struct ImpactsImpl {
    impacts: Rc<Vec<Vec<Impact>>>,
    doc_id_upto: Rc<Vec<i32>>,
  }
  impl ImpactsImpl {
    fn new(impacts: Rc<Vec<Vec<Impact>>>, doc_id_upto: Rc<Vec<i32>>) -> Self {
      Self {
        impacts,
        doc_id_upto,
      }
    }
  }
  impl Impacts for ImpactsImpl {
    fn num_levels(&self) -> i32 {
      self.impacts.len() as i32
    }

    fn get_doc_id_upto(&self, level: i32) -> i32 {
      self.doc_id_upto[level as usize]
    }

    fn get_impacts(&self, level: i32) -> Result<Vec<Impact>> {
      Ok(self.impacts[level as usize].clone())
    }
  }
}

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
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::term::Term;
use crate::core::index::term_states;
use crate::core::index::term_states::TermStates;
use crate::core::search::boolean_query;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::{
  Similarity, SimilarityEnum, SimilarityEnumSimScorer,
};
use crate::core::search::term_query::TermQuery;
use crate::core::search::term_statistics::TermStatistics;
use crate::core::search::weight::Weight;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// A query that treats multiple terms as synonyms.
///
/// For scoring purposes Lucene scores this query as if all synonyms had been indexed as one
/// term. This port preserves the public query behavior and delegates scorer construction to the
/// existing boolean/term query machinery until the specialized synonym scorer is available.
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
    _searcher: &IndexSearcher<IRC>,
    _score_mode: &ScoreMode,
    _boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext + 'static,
    Self: Sized,
  {
    todo!()
  }

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    if self.terms.is_empty() {
      return Ok(boolean_query::Builder::new().build().into());
    }
    if self.terms.len() == 1 && self.terms[0].boost == 1.0 {
      return Ok(
        TermQuery::new(Term::new(
          self.field,
          self.terms.into_iter().next().unwrap().term,
        ))
        .into(),
      );
    }
    Ok(self.into())
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
    todo!()
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
  term_states: Vec<TermStates>,
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
      term_states,
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
  type Matches = MatchWithNoTerms;

  fn matches(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _doc: i32,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::Matches>> {
    todo!()
  }

  fn explain(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _doc: i32,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    todo!()
  }

  fn get_query(&self) -> Arc<Query> {
    self.parent_query.clone()
  }

  type ScorerSupplier = QueryWeightSs<IRC>;

  fn scorer_supplier(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    todo!()
  }

  fn count(&self, _context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
    todo!()
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

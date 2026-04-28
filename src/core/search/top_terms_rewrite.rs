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
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::term::Term;
use crate::core::index::term_states::TermStates;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::MultiTermQuery;
use crate::core::search::query::Query;
use crate::core::search::term_collecting_rewrite::{TermCollectingRewrite, TermCollector};
use crate::core::util::attribute_source::AttributeSource;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};
use std::collections::HashMap;

pub trait TopTermsRewrite: TermCollectingRewrite {
  /// return the maximum priority queue size
  fn get_size(&self) -> usize;
  /// Return the maximum size of the priority queue (for boolean rewrites this is
  /// [`BooleanQuery::get_max_clause_count`]).
  fn get_max_size(&self) -> usize;
  fn default_rewrite<IRC, Q>(self, index_searcher: &IndexSearcher<IRC>, query: &Q) -> Result<Query>
  where
    Q: MultiTermQuery,
    IRC: IndexReaderContext,
    Self: Sized,
  {
    let max_size = std::cmp::min(self.get_size(), self.get_max_size());
    let mut builder = self.get_top_level_builder()?;
    let mut collector = TermCollectorImpl::new(max_size)?;
    self.collect_terms(
      index_searcher.get_top_reader_context(),
      query,
      &mut collector,
    )?;
    let keys = collector.st_queue.take_heap_array();

    let mut visited_terms = collector.st_queue.compare.visited_terms;

    let mut score_terms = Vec::with_capacity(keys.len());
    for key in keys {
      let st = visited_terms
        .remove(&key)
        .ok_or_else(|| LuceneError::illegal_state("term not found in visited_terms"))?;
      score_terms.push((key, st));
    }

    score_terms.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (bytes, st) in score_terms {
      let term = Term::new(query.get_field(), bytes);

      self.add_clause_with_states(
        &mut builder,
        term,
        st.term_state.doc_freq()?,
        st.boost.max(0.0),
        Some(st.term_state),
      )?;
    }
    self.build(builder)
  }
}
struct ScoreTerm {
  pub boost: f32,
  pub term_state: TermStates,
}

impl ScoreTerm {
  pub fn new(term_state: TermStates) -> Self {
    Self {
      boost: 0.0,
      term_state,
    }
  }
}
struct ScoreTermCmp {
  visited_terms: HashMap<BytesRef<Vec<u8>>, ScoreTerm>,
}
impl ScoreTermCmp {
  pub fn new() -> Self {
    Self {
      visited_terms: HashMap::new(),
    }
  }
}
impl Compare<BytesRef<Vec<u8>>> for ScoreTermCmp {
  fn less_than(&self, a: &BytesRef<Vec<u8>>, b: &BytesRef<Vec<u8>>) -> Result<bool> {
    let l = self
      .visited_terms
      .get(a)
      .ok_or_else(|| LuceneError::illegal_state("term not found in visited_terms"))?;
    let r = self
      .visited_terms
      .get(b)
      .ok_or_else(|| LuceneError::illegal_state("term not found in visited_terms"))?;
    if l.boost < r.boost {
      Ok(true)
    } else if l.boost > r.boost {
      Ok(false)
    } else {
      Ok(b < a)
    }
  }
}

pub(crate) struct TermCollectorImpl {
  last_term: Option<BytesRefBuilder<Vec<u8>>>,
  st_queue: PriorityQueue<BytesRef<Vec<u8>>, ScoreTermCmp>,
  max_size: usize,
  ord: usize,
}
impl TermCollectorImpl {
  pub(crate) fn new(max_size: usize) -> Result<Self> {
    let cmp = ScoreTermCmp::new();
    let st_queue = PriorityQueue::new(max_size + 1, cmp)?;
    Ok(Self {
      last_term: None,
      st_queue,
      max_size,
      ord: 0,
    })
  }
}
impl TermCollectorImpl {
  #[cfg(debug_assertions)]
  fn compare_to_last_term(&mut self, t: Option<&BytesRef<Vec<u8>>>) -> bool {
    match (&mut self.last_term, t) {
      (None, Some(t)) => {
        let mut v = BytesRefBuilder::new();
        v.append(t);
        self.last_term = Some(v);
      },
      (_, None) => {
        self.last_term = None;
      },
      (Some(last_term), Some(t)) => {
        debug_assert!(last_term.get_bytes_ref().cmp(t).is_lt());
        last_term.copy_bytes_from_ref(t)
      },
    }

    true
  }
}

impl TermCollector for TermCollectorImpl {
  fn set_reader_context<IRC>(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
  ) -> Result<()>
  where
    IRC: IndexReaderContext,
  {
    self.ord = context.ord;
    Ok(())
  }

  fn collect<TE, IRC>(
    &mut self,
    bytes: BytesRef<Vec<u8>>,
    terms_enum: &mut TE,
    top_reader_context: &IRC,
  ) -> Result<bool>
  where
    TE: TermsEnum,
    IRC: IndexReaderContext,
  {
    let boost = terms_enum.attributes()?.get_boost()?;

    debug_assert!(self.compare_to_last_term(Some(&bytes)));

    if self.st_queue.size() == self.max_size {
      let key = self
        .st_queue
        .top()
        .ok_or_else(|| LuceneError::illegal_state("PriorityQueue is empty"))?;
      let t = self
        .st_queue
        .compare
        .visited_terms
        .get(key)
        .ok_or_else(|| LuceneError::illegal_state("term not found in visited_terms"))?;
      if boost < t.boost {
        return Ok(true);
      }
      if boost == t.boost && bytes.cmp(key) == std::cmp::Ordering::Greater {
        return Ok(true);
      }
    }

    let state = terms_enum.term_state()?;
    if let Some(t) = self.st_queue.compare.visited_terms.get_mut(&bytes) {
      debug_assert!(t.boost == boost);
      t.term_state.register_with_stats(
        state,
        self.ord,
        terms_enum.doc_freq()?,
        terms_enum.total_term_freq()?,
      );
    } else {
      let mut st = ScoreTerm::new(TermStates::new(top_reader_context)?);
      st.boost = boost;
      debug_assert!(st.term_state.doc_freq()? == 0);
      st.term_state.register_with_stats(
        state,
        self.ord,
        terms_enum.doc_freq()?,
        terms_enum.total_term_freq()?,
      );
      self
        .st_queue
        .compare
        .visited_terms
        .insert(bytes.clone(), st);
      self.st_queue.add(bytes)?;

      if self.st_queue.size() > self.max_size {
        let dropped = self
          .st_queue
          .pop()?
          .ok_or_else(|| LuceneError::illegal_state("PriorityQueue is empty"))?;
        self.st_queue.compare.visited_terms.remove(&dropped);
      }

      debug_assert!(self.st_queue.size() <= self.max_size);

      if self.st_queue.size() == self.max_size {
        let key = self
          .st_queue
          .top()
          .ok_or_else(|| LuceneError::illegal_state("PriorityQueue is empty"))?;
        let t = self
          .st_queue
          .compare
          .visited_terms
          .get(key)
          .ok_or_else(|| LuceneError::illegal_state("term not found in visited_terms"))?;
        let mut attr = terms_enum.attributes_mut()?;
        attr.set_max_non_competitive_boost(t.boost)?;
        attr.set_competitive_term(Some(key.clone()))?;
      }
    }

    Ok(true)
  }

  fn set_next_enum<TE>(&mut self, _terms_enum: &mut TE) -> Result<()>
  where
    TE: TermsEnum,
  {
    debug_assert!(self.compare_to_last_term(None));
    Ok(())
  }
}

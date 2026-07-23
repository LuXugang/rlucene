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
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::test_framework::core::util::lucene_test_case::{is_night_mode, random};

use crate::core::index::index_reader_context::IRCLeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::disi_priority_queue::DisiPriorityQueue;
use crate::core::search::disi_wrapper::DisiWrapper;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryWeightSsScorer};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::Scorer;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::{DummyCR, dummy_directory, dummy_index_searcher};
use rand::Rng;
use rand::RngExt;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

#[allow(dead_code)] // for quick search
struct TestDisiPriorityQueue;

#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let size = random.random_range(1..if is_night_mode() { 1000 } else { 10 });
  let mut all = Vec::with_capacity(size);
  let dummy_s = dummy_index_searcher(dummy_directory()?)?;

  for _ in 0..size {
    let it = random_disi(&mut random);
    let w = wrapper(it, &dummy_s)?;
    all.push(w);
  }

  let mut pq = DisiPriorityQueue::new(size);
  if random.random_bool(0.5) {
    for idx in 0..all.len() {
      pq.add(idx, &all);
    }
  } else if random.random_range(0..10) < 2 && size > 1 {
    let len = random.random_range(1..size);
    let mut v = vec![];
    for i in 0..len {
      pq.add(i, &all);
      v.push(i)
    }
    for idx in len..size {
      v.push(idx)
    }
    pq.add_all(v.as_slice(), len, size - len, &all)?;
  } else {
    let mut v = vec![];
    for idx in 0..size {
      v.push(idx)
    }
    pq.add_all(v.as_slice(), 0, size, &all)?;
  }
  let mut sorted_docs = vec![0; all.len()];
  while pq.size() > 0 {
    for (doc, wrapper) in sorted_docs.iter_mut().zip(&all) {
      *doc = wrapper.doc;
    }
    sorted_docs.sort_unstable();

    let top = all.get_mut(*pq.top().as_ref().unwrap()).unwrap();
    assert_eq!(sorted_docs[0], top.doc);

    let next = top.scorer.iterator_mut().next_doc()?;
    top.doc = next;
    if next == NO_MORE_DOCS {
      pq.pop(&all);
    } else {
      pq.update_top(&all);
    }
  }

  Ok(())
}

pub fn wrapper(
  iterator: DocIdSetIteratorImpl,
  dummy_s: &IndexSearcher<CompositeReaderContext<DummyCR>>,
) -> Result<DisiWrapper<QueryWeightSsScorer>>
{
  let q = DummyQueryImpl::new(iterator);
  let weight = q.weight();
  let lrc = &dummy_s.get_leaf_contexts()?[0];
  let s = weight.scorer(lrc, dummy_s)?.unwrap();
  DisiWrapper::new(s)
}
fn random_disi<R>(random: &mut R) -> DocIdSetIteratorImpl
where
  R: Rng + ?Sized,
{
  let max_size = random.random_range(0..50);
  let upper_exclusive = NO_MORE_DOCS - 1;
  let mut v: Vec<i32> = (0..max_size)
    .map(|_| random.random_range(0..upper_exclusive))
    .collect();
  v.sort_unstable();
  v.dedup();
  let int_vec_iter = IntVecIterator::new(v);
  DocIdSetIteratorImpl::new(int_vec_iter, max_size)
}

static COUNTER: AtomicI32 = AtomicI32::new(0);
#[derive(Debug, Clone)]
pub struct DummyQueryImpl {
  id: i32,
  disi: DocIdSetIteratorImpl,
}
impl DummyQueryImpl {
  pub fn new(disi: DocIdSetIteratorImpl) -> Self {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    Self { id, disi }
  }
  fn weight(self) -> DummyQueryImplWeight {
    DummyQueryImplWeight::new(self, ScoreMode::CompleteNoScores)
  }
}
pub struct DummyQueryImplWeight {
  score_mode: ScoreMode,
  query: DummyQueryImpl,
}
impl DummyQueryImplWeight {
  fn new(query: DummyQueryImpl, score_mode: ScoreMode) -> Self {
    Self { score_mode, query }
  }
}

impl SegmentCacheable<CompositeReaderContext<DummyCR>> for DummyQueryImplWeight {
  fn is_cacheable(
    &self,
    _ctx: &LeafReaderContext<IRCLeafReader<CompositeReaderContext<DummyCR>>>,
  ) -> Result<bool> {
    Ok(true)
  }
}

impl Weight<CompositeReaderContext<DummyCR>> for DummyQueryImplWeight {
  type Matches = MatchWithNoTerms;

  fn matches(
    &self,
    context: &LeafReaderContext<IRCLeafReader<CompositeReaderContext<DummyCR>>>,
    _doc: i32,
    _searcher: &IndexSearcher<CompositeReaderContext<DummyCR>>,
  ) -> Result<Option<Self::Matches>> {
    self.default_matches(context, _doc, _searcher)
  }

  fn explain(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<CompositeReaderContext<DummyCR>>>,
    _doc: i32,
    _searcher: &IndexSearcher<CompositeReaderContext<DummyCR>>,
  ) -> Result<Explanation> {
    unreachable!()
  }

  fn get_query(&self) -> Arc<Query> {
    unreachable!()
  }

  type ScorerSupplier =
    DefaultScorerSupplier<ConstantScoreScorer<DocIdSetIteratorImpl, DummyTwoPhaseIterator>>;

  fn scorer_supplier(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<CompositeReaderContext<DummyCR>>>,
    _searcher: &IndexSearcher<CompositeReaderContext<DummyCR>>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    let v = ConstantScoreScorer::from_disi(1.0f32, self.score_mode, self.query.disi.clone());
    Ok(Some(DefaultScorerSupplier::new(v)))
  }
}
#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct DocIdSetIteratorImpl {
  doc: i32,
  random_ints: IntVecIterator,
  max_size: i32,
}
impl DocIdSetIteratorImpl {
  fn new(random_ints: IntVecIterator, max_size: i32) -> Self {
    Self {
      doc: -1,
      random_ints,
      max_size,
    }
  }
}
impl DocIdSetIterator for DocIdSetIteratorImpl {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    if self.random_ints.has_next() {
      self.doc = self.random_ints.next_int().unwrap();
    } else {
      self.doc = NO_MORE_DOCS;
    }
    Ok(self.doc)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    while self.doc < target {
      self.next_doc()?;
    }
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.max_size as i64)
  }
}
#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct IntVecIterator {
  data: Vec<i32>,
  index: usize,
}

impl IntVecIterator {
  pub fn new(data: Vec<i32>) -> Self {
    Self { data, index: 0 }
  }

  pub fn has_next(&self) -> bool {
    self.index < self.data.len()
  }

  pub fn next_int(&mut self) -> Option<i32> {
    if self.has_next() {
      let v = self.data[self.index];
      self.index += 1;
      Some(v)
    } else {
      None
    }
  }
}

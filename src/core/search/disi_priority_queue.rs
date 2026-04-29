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
use crate::core::search::disi_wrapper::DisiWrapper;
use crate::core::search::scorer::Scorer;
use crate::core::util::error::lucene_error::Result;
/// A priority queue of `DocIdSetIterator`s that orders by the current doc ID.
#[derive(Default)] // for std::mem::take
pub struct DisiPriorityQueue {
  size: usize,
  pub(crate) heap: Vec<Option<usize>>,
}
impl DisiPriorityQueue {
  pub fn new(max_size: usize) -> Self {
    Self {
      size: 0,
      heap: vec![None; max_size],
    }
  }

  pub(crate) fn left_node(node: usize) -> usize {
    ((node + 1) << 1) - 1
  }

  pub(crate) fn right_node(node: usize) -> usize {
    node + 1
  }

  pub(crate) fn parent_node(node: usize) -> usize {
    (node - 1) >> 1
  }

  pub fn size(&self) -> usize {
    self.size
  }

  pub fn top(&self) -> Option<usize> {
    self.heap[0]
  }
  /// Return the 2nd least value in this heap, or null if the heap contains less than 2 values
  pub fn top2<S>(&self, wrappers: &[DisiWrapper<S>]) -> Option<usize>
  where
    S: Scorer,
  {
    match self.size() {
      0 | 1 => None,
      2 => self.heap[1],
      _ => {
        let left = self.heap[1];
        let right = self.heap[2];
        if wrappers[*left.as_ref().unwrap()].doc <= wrappers[*right.as_ref().unwrap()].doc {
          left
        } else {
          right
        }
      },
    }
  }
  /// Get the list of scorers which are on the current doc.
  pub fn top_list_root<S>(&self, wrappers: &mut [DisiWrapper<S>]) -> usize
  where
    S: Scorer,
  {
    let heap = &self.heap;
    let mut list_index = heap[0].expect("top element missing");
    wrappers[list_index].next = None;

    if self.size >= 3 {
      list_index = self.top_list(list_index, heap, wrappers, self.size, 1);
      list_index = self.top_list(list_index, heap, wrappers, self.size, 2);
    } else if self.size == 2 {
      let child = heap[1].as_ref().unwrap();
      if wrappers[*child].doc == wrappers[list_index].doc {
        list_index = self.prepend(*child, list_index, wrappers);
      }
    }

    list_index
  }
  fn prepend<S>(&self, w1_index: usize, w2_index: usize, wrappers: &mut [DisiWrapper<S>]) -> usize
  where
    S: Scorer,
  {
    wrappers[w1_index].next = Some(w2_index);
    w1_index
  }
  pub fn top_list<S>(
    &self,
    mut list: usize,
    heap: &[Option<usize>],
    wrappers: &mut [DisiWrapper<S>],
    size: usize,
    i: usize,
  ) -> usize
  where
    S: Scorer,
  {
    let w_index = heap[i].expect("heap element missing");

    if wrappers[w_index].doc == wrappers[list].doc {
      list = self.prepend(w_index, list, wrappers);

      let left = Self::left_node(i);
      let right = left + 1;

      if right < size {
        list = self.top_list(list, heap, wrappers, size, left);
        list = self.top_list(list, heap, wrappers, size, right);
      } else if left < size {
        let left_index = heap[left].expect("left leaf missing");
        if wrappers[left_index].doc == wrappers[list].doc {
          list = self.prepend(left_index, list, wrappers);
        }
      }
    }

    list
  }

  pub fn add<S>(&mut self, entry: usize, wrappers: &[DisiWrapper<S>]) -> usize
  where
    S: Scorer,
  {
    self.heap[self.size] = Some(entry);
    self.up_heap(self.size, wrappers);
    self.size += 1;
    self.heap[0].expect("top element missing after add")
  }
  pub fn add_all<S>(
    &mut self,
    entries: &[usize],
    offset: usize,
    len: usize,
    wrappers: &[DisiWrapper<S>],
  ) -> Result<()>
  where
    S: Scorer,
  {
    // Nothing to do if empty:
    if len == 0 {
      return Ok(());
    }
    // Fail early if we're going to over-fill:
    if self.size + len > self.heap.len() {
      unreachable!(
        "Cannot add {} elements to a queue with remaining capacity {}",
        len,
        self.heap.len() - self.size
      );
    }
    // Copy the entries over to our heap array:
    for (idx, entry) in entries[offset..offset + len].iter().enumerate() {
      self.heap[self.size + idx] = Some(*entry);
    }
    self.size += len;
    // Heapify in bulk:
    let first_leaf_index = self.size >> 1;

    for root_index in (0..first_leaf_index).rev() {
      let mut parent_index = root_index;
      let parent = self.heap[parent_index].expect("parent missing");
      let parent_doc = wrappers[parent].doc;

      while parent_index < first_leaf_index {
        let mut child_index = Self::left_node(parent_index);
        let right_child_index = Self::right_node(child_index);

        let mut child = self.heap[child_index].expect("child missing");

        if right_child_index < self.size {
          let right_child = self.heap[right_child_index].expect("right child missing");
          if wrappers[right_child].doc < wrappers[child].doc {
            child = right_child;
            child_index = right_child_index;
          }
        }

        if wrappers[child].doc >= parent_doc {
          break;
        }

        self.heap[parent_index] = Some(child);
        parent_index = child_index;
      }

      self.heap[parent_index] = Some(parent);
    }
    Ok(())
  }
  pub fn pop<S>(&mut self, wrappers: &[DisiWrapper<S>]) -> usize
  where
    S: Scorer,
  {
    let result = self.heap[0].expect("pop called on empty heap");
    self.size -= 1;
    let i = self.size;
    self.heap[0] = self.heap[i];
    self.heap[i] = None;
    self.down_heap(i, wrappers);
    result
  }
  pub fn update_top<S>(&mut self, wrappers: &[DisiWrapper<S>]) -> usize
  where
    S: Scorer,
  {
    self.down_heap(self.size, wrappers);
    self.heap[0].expect("top element missing after update")
  }
  pub(crate) fn update_top_with<S>(
    &mut self,
    top_replacement: usize,
    wrappers: &[DisiWrapper<S>],
  ) -> usize
  where
    S: Scorer,
  {
    self.heap[0] = Some(top_replacement);
    self.update_top(wrappers)
  }
  /// Clear the heap.
  pub fn clear(&mut self) {
    for v in self.heap.iter_mut() {
      *v = None;
    }
    self.size = 0;
  }
  pub(crate) fn up_heap<S>(&mut self, mut i: usize, wrappers: &[DisiWrapper<S>])
  where
    S: Scorer,
  {
    let node_index = self.heap[i].expect("node missing");
    let node_doc = wrappers[node_index].doc;

    let mut j = Self::parent_node(i);

    while i > 0 && node_doc < wrappers[self.heap[j].expect("parent missing")].doc {
      self.heap[i] = self.heap[j];
      i = j;
      j = Self::parent_node(j);
    }

    self.heap[i] = Some(node_index);
  }
  pub fn down_heap<S>(&mut self, size: usize, wrappers: &[DisiWrapper<S>])
  where
    S: Scorer,
  {
    if size == 0 {
      return;
    }
    let mut i = 0;
    let node = self.heap[0].expect("node missing at root");
    let mut j = Self::left_node(i);

    if j < size {
      let mut k = Self::right_node(j);

      if k < size
        && wrappers[self.heap[k].expect("right child missing")].doc
          < wrappers[self.heap[j].expect("left child missing")].doc
      {
        j = k;
      }

      if wrappers[self.heap[j].expect("child missing")].doc < wrappers[node].doc {
        loop {
          self.heap[i] = self.heap[j];
          i = j;
          j = Self::left_node(i);
          k = Self::right_node(j);
          if k < size
            && wrappers[self.heap[k].expect("right child missing")].doc
              < wrappers[self.heap[j].expect("left child missing")].doc
          {
            j = k;
          }
          if j >= size || wrappers[self.heap[j].expect("child missing")].doc >= wrappers[node].doc {
            break;
          }
        }
        self.heap[i] = Some(node);
      }
    }
  }
  pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
    self.heap[..self.size]
      .iter()
      .cloned()
      .map(|v| v.expect("heap element missing during iteration"))
  }
}

#[cfg(test)]
pub mod tests {
  use crate::core::index::composite_reader_context::CompositeReaderContext;
  use crate::core::index::dummy::dummy_composite_reader::DummyCompositeReader;
  use crate::core::index::dummy::dummy_leaf_reader::DummyLeafReader;
  use crate::core::index::index_reader_context::IRCLeafReader;
  use crate::core::index::leaf_reader_context::{LeafReaderContext, TopParentMeta};
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
  use crate::test::core::util::dummy_index_searcher;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{is_night_mode, random};
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
    let size = random.random_range(1..if is_night_mode() { 1000 } else { 100 });
    let mut all = Vec::with_capacity(size);

    for _ in 0..size {
      let it = random_disi(&mut random);
      let w = wrapper(it)?;
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
    while pq.size() > 0 {
      let mut sorted_docs: Vec<i32> = all.iter().map(|w| w.doc).collect();
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

  pub fn wrapper(iterator: DocIdSetIteratorImpl) -> Result<DisiWrapper<QueryWeightSsScorer>>
where {
    let q = DummyQueryImpl::new(iterator);
    let weight = q.weight();
    let reader = DummyLeafReader;
    let lrc = LeafReaderContext::new(reader, 0, 0, 0, 0, TopParentMeta::default());
    let dummy_s = dummy_index_searcher()?;
    let s = weight.scorer(&lrc, &dummy_s)?.unwrap();
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

  impl SegmentCacheable<CompositeReaderContext<DummyCompositeReader<DummyLeafReader>>>
    for DummyQueryImplWeight
  {
    fn is_cacheable(
      &self,
      _ctx: &LeafReaderContext<
        IRCLeafReader<CompositeReaderContext<DummyCompositeReader<DummyLeafReader>>>,
      >,
    ) -> Result<bool> {
      Ok(true)
    }
  }

  impl Weight<CompositeReaderContext<DummyCompositeReader<DummyLeafReader>>>
    for DummyQueryImplWeight
  {
    type Matches = MatchWithNoTerms;

    fn matches(
      &self,
      context: &LeafReaderContext<
        IRCLeafReader<CompositeReaderContext<DummyCompositeReader<DummyLeafReader>>>,
      >,
      _doc: i32,
      _searcher: &IndexSearcher<CompositeReaderContext<DummyCompositeReader<DummyLeafReader>>>,
    ) -> Result<Option<Self::Matches>> {
      self.default_matches(context, _doc, _searcher)
    }

    fn explain(
      &self,
      _context: &LeafReaderContext<
        IRCLeafReader<CompositeReaderContext<DummyCompositeReader<DummyLeafReader>>>,
      >,
      _doc: i32,
      _searcher: &IndexSearcher<CompositeReaderContext<DummyCompositeReader<DummyLeafReader>>>,
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
      _context: &LeafReaderContext<
        IRCLeafReader<CompositeReaderContext<DummyCompositeReader<DummyLeafReader>>>,
      >,
      _searcher: &IndexSearcher<CompositeReaderContext<DummyCompositeReader<DummyLeafReader>>>,
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
}

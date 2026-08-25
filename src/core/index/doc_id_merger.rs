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

use crate::core::index::merge_state::DocMap;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};

/// Reuse API, currently only used by postings during merge
pub trait DocIDMerger<S> {
  /// Reuse API, currently only used by postings during merge
  fn reset(&mut self) -> Result<()>;

  /// Returns `None` when done.
  /// # NOTE:
  /// after the iterator has exhausted you should not call this method,
  /// as it may result in unpredicted behavior.
  fn next(&mut self) -> Result<Option<usize>>;
}

pub(crate) struct SequentialDocIDMerger<S> {
  subs: Vec<Sub<S>>,
  current: Option<usize>,
  next_index: usize,
}
impl<S> SequentialDocIDMerger<S>
where
  S: SubBase,
{
  pub fn new(subs: Vec<Sub<S>>) -> Result<Self> {
    let mut doc_id_merger = Self {
      subs,
      current: None,
      next_index: 0,
    };
    doc_id_merger.reset()?;
    Ok(doc_id_merger)
  }
}

impl<S> DocIDMerger<S> for SequentialDocIDMerger<S>
where
  S: SubBase,
{
  fn reset(&mut self) -> Result<()> {
    if !self.subs.is_empty() {
      self.current = Some(0);
      self.next_index = 1;
    } else {
      self.current = None;
      self.next_index = 0;
    }
    Ok(())
  }

  fn next(&mut self) -> Result<Option<usize>> {
    loop {
      match self.current {
        Some(current) => {
          let next_mapped_doc = {
            let current = &mut self.subs[current];
            current.next_mapped_doc()?
          };
          if next_mapped_doc != NO_MORE_DOCS {
            return Ok(Some(current));
          }
          if self.next_index == self.subs.len() {
            self.current = None;
            return Ok(None);
          }

          self.current = Some(self.next_index);
          self.next_index += 1;
        },
        None => return Err(LuceneError::illegal_state("current is None")),
      }
    }
  }
}

pub(crate) struct SortedDocIDMerger<S> {
  current: Option<usize>,
  queue: PriorityQueue<usize, SubCompare<S>>,
  queue_min_doc_id: i32,
}
impl<S> SortedDocIDMerger<S>
where
  S: SubBase,
{
  fn new(subs: Vec<Sub<S>>, max_count: usize) -> Result<Self> {
    if max_count <= 1 {
      return Err(LuceneError::illegal_argument(""));
    }
    let sub_compare = SubCompare::new(subs);
    let queue = PriorityQueue::new(max_count, sub_compare)?;
    let mut merger = Self {
      current: None,
      queue,
      queue_min_doc_id: 0,
    };
    merger.reset()?;
    Ok(merger)
  }
  fn set_queue_min_doc_id(&mut self) -> Result<()> {
    if self.queue.size() > 0 {
      let idx = self
        .queue
        .top()
        .ok_or_else(|| LuceneError::illegal_state("no top available"))?;
      let v = self.queue.compare.subs[*idx].mapped_doc_id;
      self.queue_min_doc_id = v;
    } else {
      self.queue_min_doc_id = NO_MORE_DOCS;
    }
    Ok(())
  }
}
impl<S> DocIDMerger<S> for SortedDocIDMerger<S>
where
  S: SubBase,
{
  fn reset(&mut self) -> Result<()> {
    // caller may not have fully consumed the queue:
    self.queue.clear();
    self.current = None;
    let mut to_add = Vec::new();
    if !self.queue.compare.subs.is_empty() {
      // by setting mappedDocID = -1, this entry is guaranteed to be
      // the top of the queue so the first call to
      // next() will advance it
      self.queue.compare.subs[0].mapped_doc_id = -1;
      self.current = Some(0);

      let mut i = 1;
      while i < self.queue.compare.subs.len() {
        let next_mapped_doc = self.queue.compare.subs[i].next_mapped_doc()?;
        if next_mapped_doc != NO_MORE_DOCS {
          to_add.push(i);
        } // else all docs in this sub were deleted; do not add it to the
        // queue!
        i += 1;
      }
    }
    for i in to_add {
      self.queue.add(i)?;
    }

    self.set_queue_min_doc_id()?;
    Ok(())
  }

  fn next(&mut self) -> Result<Option<usize>> {
    let (next_doc, current) = match self.current {
      Some(current) => (self.queue.compare.subs[current].next_mapped_doc()?, current),
      None => {
        return Err(LuceneError::illegal_state("current is None"))?;
      },
    };

    if next_doc < self.queue_min_doc_id {
      // This should be the common case when index sorting is either
      // disabled, or enabled on a low-cardinality field, or
      // enabled on a field that correlates with index order.
      return Ok(Some(current));
    }

    if next_doc == NO_MORE_DOCS {
      if self.queue.size() == 0 {
        self.current = None;
      } else {
        self.current = self.queue.pop()?;
      }
    } else if self.queue.size() > 0 {
      debug_assert!({
        let top_idx = **self.queue.top().as_ref().unwrap();
        let top = self.queue.compare.subs[top_idx].mapped_doc_id;
        self.queue_min_doc_id == top
      });
      debug_assert!(next_doc > self.queue_min_doc_id);
      let new_current_idx = *self
        .queue
        .top()
        .ok_or_else(|| LuceneError::illegal_state("no top available"))?;
      self
        .queue
        .update_top_with_new_top(self.current.take().unwrap())?;
      self.current = Some(new_current_idx);
    }

    self.set_queue_min_doc_id()?;
    match self.current {
      Some(current) => Ok(Some(current)),
      None => Ok(None),
    }
  }
}
pub(crate) enum DocIDMergerEnum<S> {
  Sequential(SequentialDocIDMerger<S>),
  Sorted(SortedDocIDMerger<S>),
}
impl<S> DocIDMergerEnum<S>
where
  S: SubBase,
{
  pub(crate) fn get_subs_mut(&mut self) -> &mut [Sub<S>] {
    match self {
      DocIDMergerEnum::Sequential(merger) => &mut merger.subs,
      DocIDMergerEnum::Sorted(merger) => &mut merger.queue.compare.subs,
    }
  }
  pub(crate) fn get_subs_vec(&mut self) -> &mut Vec<Sub<S>> {
    match self {
      DocIDMergerEnum::Sequential(merger) => &mut merger.subs,
      DocIDMergerEnum::Sorted(merger) => &mut merger.queue.compare.subs,
    }
  }
  pub(crate) fn clear_subs(&mut self) {
    match self {
      DocIDMergerEnum::Sequential(merger) => merger.subs.clear(),
      DocIDMergerEnum::Sorted(merger) => merger.queue.compare.subs.clear(),
    }
  }

  pub(crate) fn get_subs(&self) -> &[Sub<S>] {
    match self {
      DocIDMergerEnum::Sequential(merger) => &merger.subs,
      DocIDMergerEnum::Sorted(merger) => &merger.queue.compare.subs,
    }
  }
}

impl<S> DocIDMerger<S> for DocIDMergerEnum<S>
where
  S: SubBase,
{
  fn reset(&mut self) -> Result<()> {
    match self {
      DocIDMergerEnum::Sequential(merger) => merger.reset(),
      DocIDMergerEnum::Sorted(merger) => merger.reset(),
    }
  }

  fn next(&mut self) -> Result<Option<usize>> {
    match self {
      DocIDMergerEnum::Sequential(merger) => merger.next(),
      DocIDMergerEnum::Sorted(merger) => merger.next(),
    }
  }
}

/// Represents one sub-reader being merged
pub struct Sub<S> {
  /// Mapped doc ID
  pub(crate) sub: S,
  pub mapped_doc_id: i32,
}
impl<S> Sub<S> {
  pub fn new(sub: S) -> Self {
    Self {
      sub,
      mapped_doc_id: 0,
    }
  }
}

impl<S> Sub<S>
where
  S: SubBase,
{
  /// Like `next_doc()` but skips over unmapped docs and returns the next
  /// mapped doc ID, or [`NO_MORE_DOCS`](crate::core::search::doc_id_set_iterator::NO_MORE_DOCS) when exhausted.
  /// This method sets `mapped_doc_id` as a side effect.
  fn next_mapped_doc(&mut self) -> Result<i32> {
    loop {
      let doc = self.sub.next_doc()?;
      if doc == NO_MORE_DOCS {
        self.mapped_doc_id = NO_MORE_DOCS;
        return Ok(NO_MORE_DOCS);
      }
      let mapped_doc = self.sub.get_doc_map()?.get(doc)?;
      if mapped_doc != -1 {
        self.mapped_doc_id = mapped_doc;
        return Ok(mapped_doc);
      }
    }
  }
}
pub trait SubBase {
  /// Returns the next document ID from this sub reader,
  /// and [`NO_MORE_DOCS`] when done
  fn next_doc(&mut self) -> Result<i32>;
  type DocMap: DocMap;
  fn get_doc_map(&self) -> Result<&Self::DocMap>;
}

struct SubCompare<S> {
  subs: Vec<Sub<S>>,
}
impl<S> SubCompare<S> {
  fn new(subs: Vec<Sub<S>>) -> Self {
    Self { subs }
  }
}
impl<S> Compare<usize> for SubCompare<S>
where
  S: SubBase,
{
  fn less_than(&self, a: &usize, b: &usize) -> Result<bool> {
    debug_assert!(self.subs[*a].mapped_doc_id != self.subs[*b].mapped_doc_id);
    Ok(self.subs[*a].mapped_doc_id < self.subs[*b].mapped_doc_id)
  }
}

/// Construct this from the provided subs, specifying the maximum sub count.
pub(crate) fn of_with_max_count<S>(
  subs: Vec<Sub<S>>,
  max_count: usize,
  index_is_sorted: bool,
) -> Result<DocIDMergerEnum<S>>
where
  S: SubBase,
{
  if index_is_sorted && max_count > 1 {
    Ok(DocIDMergerEnum::Sorted(SortedDocIDMerger::new(
      subs, max_count,
    )?))
  } else {
    Ok(DocIDMergerEnum::Sequential(SequentialDocIDMerger::new(
      subs,
    )?))
  }
}
/// Construct this from the provided subs.
pub(crate) fn of<S>(subs: Vec<Sub<S>>, index_is_sorted: bool) -> Result<DocIDMergerEnum<S>>
where
  S: SubBase,
{
  let max_count = subs.len();
  of_with_max_count(subs, max_count, index_is_sorted)
}

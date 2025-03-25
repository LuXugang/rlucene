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
use crate::index::merge_state::{DocMap, DocMapEnum};
use crate::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::priority_queue::{Compare, PriorityQueue};
use std::cell::RefCell;
use std::rc::Rc;

/// Reuse API, currently only used by postings during merge
pub trait DocIDMerger<S>
where
    S: SubBase + Default,
{
    /// Reuse API, currently only used by postings during merge
    fn reset(&mut self) -> Result<()>;

    /// Returns `None` when done.
    /// # NOTE:
    /// after the iterator has exhausted you should not call this method,
    /// as it may result in unpredicted behavior.
    fn next(&mut self) -> Result<Option<Rc<RefCell<Sub<S>>>>>;
    /// Construct this from the provided subs, specifying the maximum sub count.
    fn of_with_max_count(
        subs: Vec<Rc<RefCell<Sub<S>>>>,
        max_count: i32,
        index_is_sorted: bool,
    ) -> Result<DocIDMergerEnum<S>> {
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
    fn of(subs: Vec<Rc<RefCell<Sub<S>>>>, index_is_sorted: bool) -> Result<DocIDMergerEnum<S>> {
        let max_count = subs.len() as i32;
        Self::of_with_max_count(subs, max_count, index_is_sorted)
    }
}
pub(crate) struct SequentialDocIDMerger<S>
where
    S: SubBase + Default,
{
    subs: Vec<Rc<RefCell<Sub<S>>>>,
    current: Option<Rc<RefCell<Sub<S>>>>,
    next_index: i32,
}
impl<S> SequentialDocIDMerger<S>
where
    S: SubBase + Default,
{
    pub fn new(subs: Vec<Rc<RefCell<Sub<S>>>>) -> Result<Self> {
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
    S: SubBase + Default,
{
    fn reset(&mut self) -> Result<()> {
        if !self.subs.is_empty() {
            self.current = Some(self.subs[0].clone());
            self.next_index = 1;
        } else {
            self.current = None;
            self.next_index = 0;
        }
        Ok(())
    }

    fn next(&mut self) -> Result<Option<Rc<RefCell<Sub<S>>>>> {
        loop {
            if let Some(ref current_sub) = self.current {
                if current_sub.borrow_mut().next_mapped_doc()? != NO_MORE_DOCS {
                    return Ok(Some(Rc::clone(current_sub)));
                }
            }

            if self.next_index as usize == self.subs.len() {
                self.current = None;
                return Ok(None);
            }

            self.current = Some(Rc::clone(&self.subs[self.next_index as usize]));
            self.next_index += 1;
        }
    }
}

pub(crate) struct SortedDocIDMerger<S>
where
    S: SubBase + Default,
{
    subs: Vec<Rc<RefCell<Sub<S>>>>,
    current: Option<Rc<RefCell<Sub<S>>>>,
    queue: PriorityQueue<Rc<RefCell<Sub<S>>>, SubCompare>,
    queue_min_doc_id: i32,
}
impl<S> SortedDocIDMerger<S>
where
    S: SubBase + Default,
{
    fn new(subs: Vec<Rc<RefCell<Sub<S>>>>, max_count: i32) -> Result<Self> {
        if max_count <= 1 {
            return Err(LuceneError::illegal_argument(""));
        }
        let queue = PriorityQueue::new(max_count, SubCompare)?;
        let mut merger = Self {
            subs,
            current: None,
            queue,
            queue_min_doc_id: 0,
        };
        merger.reset()?;
        Ok(merger)
    }
    fn set_queue_min_doc_id(&mut self) {
        if self.queue.size() > 0 {
            self.queue_min_doc_id = self.queue.top().borrow().mapped_doc_id;
        } else {
            self.queue_min_doc_id = NO_MORE_DOCS;
        }
    }
}
impl<S> DocIDMerger<S> for SortedDocIDMerger<S>
where
    S: SubBase + Default,
{
    fn reset(&mut self) -> Result<()> {
        // caller may not have fully consumed the queue:
        self.queue.clear();
        self.current = None;
        let mut first = true;
        for sub in &self.subs {
            let mut sub_mut = sub.borrow_mut();
            if first {
                // by setting mappedDocID = -1, this entry is guaranteed to be the top of the queue
                // so the first call to next() will advance it
                sub_mut.mapped_doc_id = -1;
                self.current = Some(Rc::clone(sub));
                first = false;
            } else if sub_mut.next_mapped_doc()? != NO_MORE_DOCS {
                self.queue.add(sub.clone());
            } // else all docs in this sub were deleted; do not add it to the queue!
        }

        self.set_queue_min_doc_id();
        Ok(())
    }

    fn next(&mut self) -> Result<Option<Rc<RefCell<Sub<S>>>>> {
        let next_doc = {
            if let Some(ref current) = self.current {
                current.borrow_mut().next_mapped_doc()?
            } else {
                return Err(LuceneError::unreachable("should not be here"))?;
            }
        };

        if next_doc < self.queue_min_doc_id {
            // This should be the common case when index sorting is either disabled, or enabled on a
            // low-cardinality field, or enabled on a field that correlates with index order.
            return Ok(self.current.clone());
        }

        if next_doc == NO_MORE_DOCS {
            if self.queue.size() == 0 {
                self.current = None;
            } else {
                self.current = self.queue.pop();
            }
        } else if self.queue.size() > 0 {
            debug_assert!(!self.queue_min_doc_id == self.queue.top().borrow().mapped_doc_id);
            debug_assert!(next_doc > self.queue_min_doc_id);
            let new_current = self.queue.top().clone();
            self.queue
                .update_top_with_new_top(self.current.take().unwrap());
            self.current = Some(new_current);
        }

        self.set_queue_min_doc_id();
        Ok(self.current.clone())
    }
}
pub(crate) enum DocIDMergerEnum<S>
where
    S: SubBase + Default,
{
    Sequential(SequentialDocIDMerger<S>),
    Sorted(SortedDocIDMerger<S>),
}
impl<S> DocIDMerger<S> for DocIDMergerEnum<S>
where
    S: SubBase + Default,
{
    fn reset(&mut self) -> Result<()> {
        match self {
            DocIDMergerEnum::Sequential(merger) => merger.reset(),
            DocIDMergerEnum::Sorted(merger) => merger.reset(),
        }
    }

    fn next(&mut self) -> Result<Option<Rc<RefCell<Sub<S>>>>> {
        match self {
            DocIDMergerEnum::Sequential(merger) => merger.next(),
            DocIDMergerEnum::Sorted(merger) => merger.next(),
        }
    }
}

/// Represents one sub-reader being merged
#[derive(Default)]
pub struct Sub<S>
where
    S: SubBase + Default,
{
    /// Mapped doc ID
    sub: S,
    mapped_doc_id: i32,
    doc_map: Rc<DocMapEnum>,
}
impl<S> Sub<S>
where
    S: SubBase + Default,
{
    pub fn new(sub: S, doc_map: Rc<DocMapEnum>) -> Self {
        Self {
            sub,
            mapped_doc_id: 0,
            doc_map,
        }
    }
    /// Like `next_doc()` but skips over unmapped docs and returns the next mapped doc ID,
    /// or `DocIdSetIterator::NO_MORE_DOCS` when exhausted.
    /// This method sets `mapped_doc_id` as a side effect.
    fn next_mapped_doc(&mut self) -> Result<i32> {
        loop {
            let doc = self.sub.next_doc()?;
            if doc == NO_MORE_DOCS {
                self.mapped_doc_id = NO_MORE_DOCS;
                return Ok(NO_MORE_DOCS);
            }
            let mapped_doc = self.doc_map.get(doc);
            if mapped_doc != -1 {
                self.mapped_doc_id = mapped_doc;
                return Ok(mapped_doc);
            }
        }
    }
}
pub trait SubBase {
    /// Returns the next document ID from this sub reader,
    /// and `DocIdSetIterator::NO_MORE_DOCS` when done
    fn next_doc(&mut self) -> Result<i32>;
}

struct SubCompare;
impl<S> Compare<Rc<RefCell<Sub<S>>>> for SubCompare
where
    S: SubBase + Default,
{
    fn less_than(&self, a: &Rc<RefCell<Sub<S>>>, b: &Rc<RefCell<Sub<S>>>) -> bool {
        debug_assert!(a.borrow().mapped_doc_id != b.borrow().mapped_doc_id);
        a.borrow().mapped_doc_id < b.borrow().mapped_doc_id
    }
}

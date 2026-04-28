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
use crate::core::codecs::block_term_state::TermStateEnum;
use crate::core::index::index_reader::Identity;
use crate::core::index::multi_postings_enum::{EnumWithSlice, MultiPostingsEnum};
use crate::core::index::reader_slice::ReaderSlice;
use crate::core::index::slow_impacts_enum::SlowImpactsEnum;
use crate::core::index::terms_enum::{EmptyTermsEnum, SeekStatus, TermsEnum, TermsEnumEnum2};
use crate::core::index::terms_enum_index::TermsEnumIndex;
use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::dummy::dummy_attribute_source::DummyAttributeSource;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};
use crate::core::util::{Comparator, ToInt, TryIntoInt};
use std::borrow::Cow;
use std::rc::Rc;

/// Exposes [`TermsEnum`] API, merged from [`TermsEnum`] API of sub-segments. This does a
/// merge sort, by term text, of the sub-readers.
pub struct MultiTermsEnum<TE>
where
  TE: TermsEnum,
{
  queue: TermMergeQueue<TE>,
  /// All subs (one per sub-reader), stored as indices
  subs: Vec<usize>,
  /// Current subs that have at least one term for this field
  current_subs: Vec<usize>,
  top: Vec<usize>,
  /// Last seek term
  last_seek: Option<BytesRef<Vec<u8>>>,
  sub_docs: Vec<EnumWithSlice>,
  last_seek_exact: bool,
  last_seek_scratch: BytesRefBuilder<Vec<u8>>,
  num_top: usize,
  num_subs: i32,
  current: Option<BytesRef<Vec<u8>>>,
  parent: Identity,
}
impl<TE> MultiTermsEnum<TE>
where
  TE: TermsEnum,
{
  pub fn new(slices: Vec<Rc<ReaderSlice>>) -> Result<Self> {
    let len = slices.len();
    let mut subs = vec![0usize; len];
    let current_subs = vec![0usize; len];
    let top = vec![0usize; len];
    let mut sub_docs = Vec::with_capacity(len);
    let mut all_terms_enum_with_slice = Vec::with_capacity(len);
    for (i, slice) in slices.into_iter().enumerate() {
      all_terms_enum_with_slice.push(TermsEnumWithSlice::new(i, slice.clone()));
      sub_docs.push(EnumWithSlice::with_slice(slice));
      subs[i] = i;
    }
    let queue = TermMergeQueue::new(len, all_terms_enum_with_slice)?;
    Ok(Self {
      queue,
      subs,
      current_subs,
      top,
      last_seek: None,
      sub_docs,
      last_seek_exact: false,
      last_seek_scratch: BytesRefBuilder::new(),
      num_top: 0,
      num_subs: 0,
      current: None,
      parent: Identity::new(),
    })
  }

  /// The terms array must be newly created TermsEnum, ie [`TermsEnum.next`](TermsEnum::next) has not yet been called.
  pub fn reset(
    mut self,
    terms_enums_index: Vec<TermsEnumIndex<TE>>,
  ) -> Result<MultiTermsEnumType<TE>> {
    debug_assert!(terms_enums_index.len() <= self.top.len());

    self.num_subs = 0;
    self.num_top = 0;
    self.queue.q.clear();

    for mut terms_enum_index in terms_enums_index.into_iter() {
      if (terms_enum_index.next()?).is_some() {
        let sub_idx = terms_enum_index.sub_index;
        let entry_idx = self.subs[sub_idx];
        let entry = &mut self.queue.q.compare.all_terms_enum_with_slice[entry_idx];
        entry.base.reset(terms_enum_index);
        self.queue.q.add(entry_idx)?;
        self.current_subs[self.num_subs as usize] = entry_idx;
        self.num_subs += 1;
      } else {
        // field has no terms
      }
    }

    if self.queue.q.size() == 0 {
      Ok(TermsEnumEnum2::B(EmptyTermsEnum))
    } else {
      Ok(TermsEnumEnum2::A(self))
    }
  }
  fn pull_top(&mut self) -> Result<()> {
    // extract all subs from the queue that have the same
    // top term
    debug_assert_eq!(self.num_top, 0);

    self.num_top = self.queue.fill_top(&mut self.top)?.try_convert()?;

    let top0_idx = self.top[0];
    let top0 = &self.queue.q.compare.all_terms_enum_with_slice[top0_idx];
    self.current = top0.base.term().cloned();

    Ok(())
  }
  fn push_top(&mut self) -> Result<()> {
    // call next() on each top, and reorder queue
    for _ in 0..self.num_top {
      let top_idx = *self
        .queue
        .q
        .top()
        .ok_or_else(|| LuceneError::illegal_state("top() returned None"))?;
      let top = &mut self.queue.q.compare.all_terms_enum_with_slice[top_idx];

      if top.base.next()?.is_none() {
        self.queue.q.pop()?;
      } else {
        self.queue.q.update_top()?;
      }
    }
    self.num_top = 0;
    Ok(())
  }
}

impl<TE> BytesRefIterator for MultiTermsEnum<TE>
where
  TE: TermsEnum,
{
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    if self.last_seek_exact {
      // Must seekCeil at this point, so those subs that
      // didn't have the term can find the following term.
      // NOTE: we could save some CPU by only seekCeil the
      // subs that didn't match the last exact seek... but
      // most impls short-circuit if you seekCeil to term
      // they are already on.
      let current = self.current.clone();
      let cur = current
        .as_ref()
        .ok_or_else(|| LuceneError::illegal_state("current is None but last_seek_exact=true"))?;
      let status = self.seek_ceil(cur)?;
      debug_assert_eq!(status, SeekStatus::Found);
      self.last_seek_exact = false;
    }

    self.last_seek = None;

    // restore queue
    self.push_top()?;

    // gather equal top fields
    if self.queue.q.size() > 0 {
      self.pull_top()?;
    } else {
      self.current = None;
    }
    match self.current {
      None => Ok(None),
      Some(ref v) => Ok(Some(Cow::Borrowed(v))),
    }
  }
}

impl<TE> TermsEnum for MultiTermsEnum<TE>
where
  TE: TermsEnum,
{
  type AttributeSource<'a>
    = &'a DummyAttributeSource
  where
    Self: 'a;
  type AttributeSourceMut<'a>
    = &'a mut DummyAttributeSource
  where
    Self: 'a;

  fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
    self.queue.q.clear();
    self.num_top = 0;

    let mut seek_opt = false;
    if let Some(ref last) = self.last_seek
      && last.cmp(term).to_int() <= 0
    {
      seek_opt = true;
    }

    self.last_seek = None;
    self.last_seek_exact = true;

    for i in 0..(self.num_subs as usize) {
      let entry_idx = self.current_subs[i];
      let status: bool;

      // LUCENE-2130: if we had just seek'd already, prior
      // to this seek, and the new seek term is after the
      // previous one, don't try to re-seek this sub if its
      // current term is already beyond this new seek term.
      // Doing so is a waste because this sub will simply
      // seek to the same spot.
      if seek_opt {
        let entry = &mut self.queue.q.compare.all_terms_enum_with_slice[entry_idx];
        let cur_term = entry.base.term();

        if let Some(cur) = cur_term {
          let cmp = term.cmp(cur).to_int();
          if cmp == 0 {
            status = true;
          } else if cmp < 0 {
            status = false;
          } else {
            status = entry.base.seek_exact(term)?;
          }
        } else {
          status = false;
        }
      } else {
        let entry = &mut self.queue.q.compare.all_terms_enum_with_slice[entry_idx];
        status = entry.base.seek_exact(term)?;
      }

      if status {
        self.top[self.num_top] = entry_idx;
        self.num_top += 1;

        let cur = {
          let entry = &mut self.queue.q.compare.all_terms_enum_with_slice[entry_idx];
          entry.base.term()
        };
        self.current = cur.cloned();

        debug_assert!({
          let t = {
            let entry = &mut self.queue.q.compare.all_terms_enum_with_slice[entry_idx];
            entry.base.term()
          };
          match t {
            Some(v) => term == v,
            None => false,
          }
        });
      }
    }
    // if at least one sub had exact match to the requested
    // term then we found match
    Ok(self.num_top > 0)
  }

  fn prepare_seek_exact(&mut self, _text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_prepare_seek_exact_status(&mut self, _target: &BytesRef<Vec<u8>>) -> Result<bool> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    self.queue.q.clear();
    self.num_top = 0;
    self.last_seek_exact = false;

    let mut seek_opt = false;
    if let Some(ref last) = self.last_seek
      && last.cmp(term).to_int() <= 0
    {
      seek_opt = true;
    }

    self.last_seek_scratch.copy_bytes_from_ref(term);
    self.last_seek = Some(self.last_seek_scratch.get_bytes_owner());

    for i in 0..(self.num_subs as usize) {
      let entry_idx = self.current_subs[i];
      let status: SeekStatus;

      // LUCENE-2130: if we had just seek'd already, prior
      // to this seek, and the new seek term is after the
      // previous one, don't try to re-seek this sub if its
      // current term is already beyond this new seek term.
      // Doing so is a waste because this sub will simply
      // seek to the same spot.
      if seek_opt {
        let entry = &mut self.queue.q.compare.all_terms_enum_with_slice[entry_idx];
        let cur_term = entry.base.term();

        if let Some(cur) = cur_term {
          let cmp = term.cmp(cur).to_int();
          if cmp == 0 {
            status = SeekStatus::Found;
          } else if cmp < 0 {
            status = SeekStatus::NotFound;
          } else {
            status = entry.base.seek_ceil(term)?;
          }
        } else {
          status = SeekStatus::End;
        }
      } else {
        let entry = &mut self.queue.q.compare.all_terms_enum_with_slice[entry_idx];
        status = entry.base.seek_ceil(term)?;
      }

      if status == SeekStatus::Found {
        self.top[self.num_top] = entry_idx;
        self.num_top += 1;

        let cur = {
          let entry = &mut self.queue.q.compare.all_terms_enum_with_slice[entry_idx];
          entry.base.term()
        };
        self.current = cur.cloned();

        self.queue.q.add(entry_idx)?;
      } else if status == SeekStatus::NotFound {
        debug_assert!({
          let entry = &mut self.queue.q.compare.all_terms_enum_with_slice[entry_idx];
          entry.base.term().is_some()
        });
        self.queue.q.add(entry_idx)?;
      } else {
        debug_assert_eq!(status, SeekStatus::End);
      }
    }

    if self.num_top > 0 {
      // at least one sub had exact match to the requested term
      Ok(SeekStatus::Found)
    } else if self.queue.q.size() > 0 {
      // no sub had exact match, but at least one sub found
      // a term after the requested term -- advance to that
      // next term:
      self.pull_top()?;
      Ok(SeekStatus::NotFound)
    } else {
      Ok(SeekStatus::End)
    }
  }

  fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_exact_with_state(
    &mut self,
    term: &BytesRef<Vec<u8>>,
    _state: &TermStateEnum,
  ) -> Result<()> {
    if !self.seek_exact(term)? {
      return Err(LuceneError::illegal_state(format!(
        "term {} does not exist",
        term
      )));
    }
    Ok(())
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    match self.current {
      None => Err(LuceneError::illegal_state("current is None in term() call")),
      Some(ref v) => Ok(Cow::Borrowed(v)),
    }
  }

  fn ord(&self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn doc_freq(&mut self) -> Result<i32> {
    let mut sum: i32 = 0;
    for i in 0..(self.num_top) {
      let idx = self.top[i];
      let entry = &mut self.queue.q.compare.all_terms_enum_with_slice[idx];
      match entry.base.terms_enum {
        Some(ref mut terms_enum) => {
          sum += terms_enum.doc_freq()?;
        },
        None => return Err(LuceneError::illegal_state("terms_enum is None")),
      }
    }
    Ok(sum)
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    let mut sum: i64 = 0;
    for i in 0..(self.num_top) {
      let idx = self.top[i];
      let entry = &mut self.queue.q.compare.all_terms_enum_with_slice[idx];
      match entry.base.terms_enum {
        Some(ref mut terms_enum) => {
          let v = terms_enum.total_term_freq()?;
          debug_assert!(v != -1);
          sum += v;
        },
        None => return Err(LuceneError::illegal_state("terms_enum is None")),
      }
    }
    Ok(sum)
  }

  type PostingsEnum = MultiPostingsEnum<TE::PostingsEnum>;

  fn postings_with_flags(
    &mut self,
    reuse: Option<Self::PostingsEnum>,
    flags: i32,
  ) -> Result<Self::PostingsEnum> {
    let mut docs_enum = match reuse {
      Some(reuse) => {
        if reuse.can_reuse(&self.parent) {
          reuse
        } else {
          MultiPostingsEnum::new(self.parent.clone(), self.subs.len())
        }
      },
      None => MultiPostingsEnum::new(self.parent.clone(), self.subs.len()),
    };
    let mut upto: usize = 0;
    let cmp =
      TopTermsEnumWithSliceCmp::new(self.queue.q.compare.all_terms_enum_with_slice.as_slice());
    ArrayUtil::do_tim_sort(self.top.as_mut(), 0, self.num_top, cmp)?;

    for i in 0..(self.num_top) {
      let entry_idx = self.top[i];
      let entry = &mut self.queue.q.compare.all_terms_enum_with_slice[entry_idx];

      let sub_index = entry.base.sub_index;
      debug_assert!(
        sub_index < docs_enum.sub_postings_enums.len(),
        "{} vs {}; {}",
        sub_index,
        docs_enum.sub_postings_enums.len(),
        self.subs.len()
      );

      let sub_postings_enum = entry
        .base
        .terms_enum
        .as_mut()
        .unwrap()
        .postings_with_flags(docs_enum.sub_postings_enums[sub_index].take(), flags)?;
      docs_enum.sub_postings_enums[sub_index] = Some(sub_postings_enum);
      self.sub_docs[upto].postings_enum_idx = sub_index;
      self.sub_docs[upto].slice = entry.sub_slice.clone();
      upto += 1;
    }
    docs_enum.reset(&self.sub_docs, upto as i32);
    Ok(docs_enum)
  }

  type ImpactsEnum = SlowImpactsEnum<MultiPostingsEnum<TE::PostingsEnum>>;

  fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
    Ok(SlowImpactsEnum::new(self.postings_with_flags(None, flags)?))
  }

  fn term_state(&mut self) -> Result<TermStateEnum> {
    todo!()
  }
}
struct TopTermsEnumWithSliceCmp<'a, TE>
where
  TE: TermsEnum,
{
  terms_enums: &'a [TermsEnumWithSlice<TE>],
}
impl<'a, TE> TopTermsEnumWithSliceCmp<'a, TE>
where
  TE: TermsEnum,
{
  pub fn new(terms_enums: &'a [TermsEnumWithSlice<TE>]) -> Self {
    Self { terms_enums }
  }
}
impl<TE> Comparator<usize> for TopTermsEnumWithSliceCmp<'_, TE>
where
  TE: TermsEnum,
{
  const TYPE: &'static str = "TopTermsEnumWithSliceCmp";

  fn compare(&self, a: &usize, b: &usize) -> Result<i32> {
    let va = self.terms_enums[*a].base.sub_index;
    let vb = self.terms_enums[*b].base.sub_index;
    Ok(va.cmp(&vb).to_int())
  }
}

pub type MultiTermsEnumType<TE> = TermsEnumEnum2<MultiTermsEnum<TE>, EmptyTermsEnum>;

struct TermsEnumWithSlice<TE>
where
  TE: TermsEnum,
{
  base: TermsEnumIndex<TE>,
  sub_slice: Rc<ReaderSlice>,
}
impl<TE> TermsEnumWithSlice<TE>
where
  TE: TermsEnum,
{
  pub fn new(index: usize, sub_slice: Rc<ReaderSlice>) -> Self {
    debug_assert!(sub_slice.length >= 0, "length={}", sub_slice.length);

    Self {
      base: TermsEnumIndex::new(None, index),
      sub_slice,
    }
  }
}

struct TermMergeQueue<TE>
where
  TE: TermsEnum,
{
  stack: Vec<i32>,
  q: PriorityQueue<usize, TermMergeQueueCmp<TE>>,
}
impl<TE> TermMergeQueue<TE>
where
  TE: TermsEnum,
{
  pub fn new(size: usize, all_terms_enum_with_slice: Vec<TermsEnumWithSlice<TE>>) -> Result<Self> {
    let cmp = TermMergeQueueCmp::new(all_terms_enum_with_slice);
    let queue = PriorityQueue::new(size, cmp)?;
    Ok(Self {
      stack: vec![0; size],
      q: queue,
    })
  }
  /// Add the top() slice as well as all slices that are positionned on the same term to tops and return how many of them there are.
  pub(crate) fn fill_top(&mut self, tops: &mut [usize]) -> Result<i32> {
    let size = self.q.size();
    if size == 0 {
      return Ok(0);
    }

    tops[0] = *self
      .q
      .top()
      .ok_or_else(|| LuceneError::illegal_state("top() returned None"))?;
    let mut num_top: usize = 1;
    self.stack[0] = 1;
    let mut stack_len: usize = 1;

    while stack_len != 0 {
      stack_len -= 1;
      let index = self.stack[stack_len] as usize;

      let left_child = index << 1;
      let end = std::cmp::min(size, left_child + 1);

      for child in left_child..=end {
        let te_idx = self.get(child)?;
        let top0_idx = tops[0];

        let cmp = {
          let te = &self.q.compare.all_terms_enum_with_slice[te_idx];
          let top0 = &self.q.compare.all_terms_enum_with_slice[top0_idx];
          te.base.compare_term_to(&top0.base)?
        };

        if cmp == 0 {
          tops[num_top] = te_idx;
          num_top += 1;

          self.stack[stack_len] = child.try_convert()?;
          stack_len += 1;
        }
      }
    }
    num_top.try_convert()
  }
  fn get(&self, i: usize) -> Result<usize> {
    self.q.get_heap_array()[i]
      .ok_or_else(|| LuceneError::illegal_state("get_heap_array() returned None"))
  }
}
struct TermMergeQueueCmp<TE>
where
  TE: TermsEnum,
{
  all_terms_enum_with_slice: Vec<TermsEnumWithSlice<TE>>,
}
impl<TE> TermMergeQueueCmp<TE>
where
  TE: TermsEnum,
{
  fn new(all_terms_enum_with_slice: Vec<TermsEnumWithSlice<TE>>) -> Self {
    Self {
      all_terms_enum_with_slice,
    }
  }
}
impl<TE> Compare<usize> for TermMergeQueueCmp<TE>
where
  TE: TermsEnum,
{
  fn less_than(&self, a: &usize, b: &usize) -> Result<bool> {
    Ok(
      self.all_terms_enum_with_slice[*a]
        .base
        .compare_term_to(&self.all_terms_enum_with_slice[*b].base)?
        < 0,
    )
  }
}

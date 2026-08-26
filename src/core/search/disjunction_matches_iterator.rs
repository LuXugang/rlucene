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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::postings_enum::OFFSETS;
use crate::core::index::term::Term;
use crate::core::index::terms::{Terms, get_terms};
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::matches_iterator::MatchesIterator;
use crate::core::search::query::{Query, QueryWeightMatchesIterator};
use crate::core::search::term_matches_iterator::TermMatchesIterator;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};
use std::borrow::Cow;
use std::sync::Arc;

/// A [`MatchesIterator`] that combines matches from a set of sub-iterators.
///
/// Matches are sorted by their start positions, and then by their end positions,
/// so that prefixes sort first.
///
/// Matches may overlap, or be duplicated if they appear in more than one of the
/// sub-iterators.
pub struct DisjunctionMatchesIterator<M> {
  queue: PriorityQueue<M, DisjunctionMatchesIteratorPQCmp>,
  started: bool,
}
impl<M> DisjunctionMatchesIterator<M>
where
  M: MatchesIterator,
{
  pub fn new(mut matches: Vec<M>) -> Result<Self> {
    debug_assert!(matches.len() <= i32::MAX as usize);
    let size = matches.len();
    let mut queue = PriorityQueue::new(size, DisjunctionMatchesIteratorPQCmp)?;
    for mut sub in matches.drain(..) {
      if sub.next()? {
        queue.add(sub)?;
      }
    }
    Ok(DisjunctionMatchesIterator {
      queue,
      started: false,
    })
  }
}
impl<M> MatchesIterator for DisjunctionMatchesIterator<M>
where
  M: MatchesIterator,
{
  fn next(&mut self) -> Result<bool> {
    if !self.started {
      self.started = true;
      return Ok(self.queue.size() > 0);
    }

    if !self
      .queue
      .top_mut()
      .ok_or_else(|| LuceneError::illegal_state("no top available"))?
      .next()?
    {
      self.queue.pop_unchecked()?;
    }

    if self.queue.size() > 0 {
      self.queue.update_top()?;
      Ok(true)
    } else {
      Ok(false)
    }
  }

  fn start_position(&self) -> Result<i32> {
    self
      .queue
      .top()
      .ok_or_else(|| LuceneError::illegal_state("priority queue top element should exist"))?
      .start_position()
  }

  fn end_position(&self) -> i32 {
    self
      .queue
      .top()
      .expect("priority queue top element should exist")
      .end_position()
  }

  fn start_offset(&self) -> Result<i32> {
    self
      .queue
      .top()
      .ok_or_else(|| LuceneError::illegal_state("priority queue top element should exist"))?
      .start_offset()
  }

  fn end_offset(&self) -> Result<i32> {
    self
      .queue
      .top()
      .ok_or_else(|| LuceneError::illegal_state("priority queue top element should exist"))?
      .end_offset()
  }

  fn get_sub_matches(&mut self) -> Result<Option<QueryWeightMatchesIterator<'_>>> {
    self
      .queue
      .top_mut()
      .ok_or_else(|| LuceneError::illegal_state("no top available"))?
      .get_sub_matches()
  }

  fn get_query(&self) -> Arc<Query> {
    self
      .queue
      .top()
      .expect("priority queue top element should exist")
      .get_query()
  }
}

pub(crate) struct DisjunctionMatchesIteratorPQCmp;
impl<M> Compare<M> for DisjunctionMatchesIteratorPQCmp
where
  M: MatchesIterator,
{
  fn less_than(&self, a: &M, b: &M) -> Result<bool> {
    if a.start_position()? == -1 && b.start_position()? == -1 {
      let a_start = a.start_offset()?;
      let b_start = b.start_offset()?;
      let a_end = a.end_offset()?;
      let b_end = b.end_offset()?;
      return Ok(a_start < b_start || (a_start == b_start && a_end <= b_end));
    }
    let a_start = a.start_position()?;
    let b_start = b.start_position()?;
    let a_end = a.end_position();
    let b_end = b.end_position();
    Ok(a_start < b_start || (a_start == b_start && a_end <= b_end))
  }
}
// MatchesIterator over a set of terms that only loads the first matching term at construction,
// waiting until the iterator is actually used before it loads all other matching terms.
pub(crate) struct TermsEnumDisjunctionMatchesIterator<'a, TE, BRI>
where
  TE: TermsEnum,
  TE::PostingsEnum: 'a,
{
  first: Option<TermMatchesIterator<TE::PostingsEnum>>,
  terms: BRI,
  te: TE,
  doc: i32,
  query: Arc<Query>,
  it: Option<QueryWeightMatchesIterator<'a>>,
}
impl<'a, TE, BRI> TermsEnumDisjunctionMatchesIterator<'a, TE, BRI>
where
  TE: TermsEnum,
  TE::PostingsEnum: 'a,
  BRI: BytesRefIterator,
{
  pub fn new(
    first: TermMatchesIterator<TE::PostingsEnum>,
    terms: BRI,
    te: TE,
    doc: i32,
    query: Arc<Query>,
  ) -> Self {
    TermsEnumDisjunctionMatchesIterator {
      first: Some(first),
      terms,
      te,
      doc,
      query,
      it: None,
    }
  }

  fn init(&mut self) -> Result<()> {
    let mut matches: Vec<QueryWeightMatchesIterator<'a>> =
      vec![Box::new(self.first.take().ok_or_else(|| {
        LuceneError::illegal_state("first matches iterator is missing")
      })?)];
    let mut reuse = None;
    while let Some(term) = self.terms.next()? {
      if self.te.seek_exact(term.as_ref())? {
        let mut postings = self.te.postings_with_flags(reuse, OFFSETS as i32)?;
        if postings.advance(self.doc)? == self.doc {
          matches.push(Box::new(TermMatchesIterator::new(
            postings,
            self.query.clone(),
          )?));
          reuse = None;
        } else {
          reuse = Some(postings);
        }
      }
    }
    self.it = from_sub_iterators(matches)?;
    debug_assert!(self.it.is_some());
    Ok(())
  }
}

impl<'a, TE, BRI> MatchesIterator for TermsEnumDisjunctionMatchesIterator<'a, TE, BRI>
where
  TE: TermsEnum,
  TE::PostingsEnum: 'a,
  BRI: BytesRefIterator,
{
  fn next(&mut self) -> Result<bool> {
    if self.it.is_none() {
      self.init()?;
    }
    self
      .it
      .as_mut()
      .ok_or_else(|| LuceneError::illegal_state("matches iterator is not initialized"))?
      .next()
  }

  fn start_position(&self) -> Result<i32> {
    self
      .it
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("matches iterator is not initialized"))?
      .start_position()
  }

  fn end_position(&self) -> i32 {
    self
      .it
      .as_ref()
      .expect("matches iterator is not initialized")
      .end_position()
  }

  fn start_offset(&self) -> Result<i32> {
    self
      .it
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("matches iterator is not initialized"))?
      .start_offset()
  }

  fn end_offset(&self) -> Result<i32> {
    self
      .it
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("matches iterator is not initialized"))?
      .end_offset()
  }

  fn get_sub_matches(&mut self) -> Result<Option<QueryWeightMatchesIterator<'_>>> {
    self
      .it
      .as_mut()
      .ok_or_else(|| LuceneError::illegal_state("matches iterator is not initialized"))?
      .get_sub_matches()
  }

  fn get_query(&self) -> Arc<Query> {
    self
      .it
      .as_ref()
      .expect("matches iterator is not initialized")
      .get_query()
  }
}

struct TermBytesRefIterator {
  terms: Vec<Term>,
  index: usize,
}
impl BytesRefIterator for TermBytesRefIterator {
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    if self.index == self.terms.len() {
      return Ok(None);
    }
    let term = &self.terms[self.index];
    self.index += 1;
    Ok(Some(Cow::Borrowed(term.bytes())))
  }
}

/// Create a [`MatchesIterator`] over a list of terms.
///
/// Only terms that have at least one match in the given document will be included.
pub(crate) fn from_terms<'a, LR>(
  context: &LeafReaderContext<LR>,
  doc: i32,
  query: Arc<Query>,
  field: &str,
  terms: Vec<Term>,
) -> Result<Option<QueryWeightMatchesIterator<'a>>>
where
  LR: LeafReader,
  <LR::Terms as Terms>::TermsEnum: 'a,
  <<LR::Terms as Terms>::TermsEnum as TermsEnum>::PostingsEnum: 'a,
{
  for term in &terms {
    if term.field() != field {
      return Err(LuceneError::illegal_argument(format!(
        "Tried to generate iterator from terms in multiple fields: expected [{}] but got [{}]",
        field,
        term.field()
      )));
    }
  }
  from_terms_enum(
    context,
    doc,
    query,
    field,
    TermBytesRefIterator { terms, index: 0 },
  )
}

/// Create a [`MatchesIterator`] over terms extracted from a [`BytesRefIterator`].
///
/// Only terms that have at least one match in the given document will be included.
pub(crate) fn from_terms_enum<'a, LR, BRI>(
  context: &LeafReaderContext<LR>,
  doc: i32,
  query: Arc<Query>,
  field: &str,
  mut terms: BRI,
) -> Result<Option<QueryWeightMatchesIterator<'a>>>
where
  LR: LeafReader,
  BRI: BytesRefIterator + 'a,
  <LR::Terms as Terms>::TermsEnum: 'a,
  <<LR::Terms as Terms>::TermsEnum as TermsEnum>::PostingsEnum: 'a,
{
  let indexed_terms = get_terms(context.reader(), field)?;
  let mut terms_enum = indexed_terms.iterator()?;
  let mut reuse = None;
  while let Some(term) = terms.next()? {
    if terms_enum.seek_exact(term.as_ref())? {
      let mut postings = terms_enum.postings_with_flags(reuse, OFFSETS as i32)?;
      if postings.advance(doc)? == doc {
        return Ok(Some(Box::new(TermsEnumDisjunctionMatchesIterator::new(
          TermMatchesIterator::new(postings, query.clone())?,
          terms,
          terms_enum,
          doc,
          query,
        ))));
      }
      reuse = Some(postings);
    }
  }
  Ok(None)
}

pub fn from_sub_iterators<'a>(
  mut mis: Vec<QueryWeightMatchesIterator<'a>>,
) -> Result<Option<QueryWeightMatchesIterator<'a>>> {
  if mis.is_empty() {
    return Ok(None);
  }
  if mis.len() == 1 {
    return Ok(mis.pop());
  }
  Ok(Some(Box::new(DisjunctionMatchesIterator::new(mis)?)))
}

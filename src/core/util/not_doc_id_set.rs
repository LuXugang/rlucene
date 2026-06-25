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
use crate::core::index::index_reader::Identity;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::HasIdentity;
use crate::core::util::accountable::Accountable;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
/// This [`DocIdSet`] encodes the negation of another
/// [`DocIdSet`]. It is cacheable and supports random-access
/// if the underlying set is cacheable and supports random-access.
///
/// # Note
/// This is an internal API.
pub struct NotDocIdSet<T>
where
  T: DocIdSet,
{
  max_doc: i32,
  in_: T,
}

impl<T> NotDocIdSet<T>
where
  T: DocIdSet,
{
  pub fn new(max_doc: i32, set: T) -> Self {
    NotDocIdSet { max_doc, in_: set }
  }
}

impl<T> Accountable for NotDocIdSet<T>
where
  T: DocIdSet,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    self.in_.ram_bytes_used()
  }
}

impl<T> DocIdSet for NotDocIdSet<T>
where
  T: DocIdSet,
{
  type DocIdSetIterator = NotDocDocIdSetIterator<T::DocIdSetIterator>;

  fn iterator(&self) -> Result<Self::DocIdSetIterator> {
    Ok(NotDocDocIdSetIterator::new(
      self.in_.iterator()?,
      self.max_doc,
    ))
  }

  type Bits = NotDocIdBits<T::Bits>;

  fn bits(&self) -> Option<Self::Bits> {
    self.in_.bits().map(NotDocIdBits::new)
  }
}

#[derive(Clone)]
pub struct NotDocIdBits<B>
where
  B: Bits,
{
  in_bit: B,
  id: Identity,
}

impl<B> NotDocIdBits<B>
where
  B: Bits,
{
  pub fn new(in_bits: B) -> NotDocIdBits<B> {
    NotDocIdBits {
      in_bit: in_bits,
      id: Identity::new(),
    }
  }
}

impl<B> HasIdentity for NotDocIdBits<B>
where
  B: Bits,
{
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl<B> Bits for NotDocIdBits<B>
where
  B: Bits,
{
  fn get(&self, index: usize) -> Result<bool> {
    Ok(!self.in_bit.get(index)?)
  }

  fn length(&self) -> usize {
    self.in_bit.length()
  }
}

pub struct NotDocDocIdSetIterator<D: DocIdSetIterator> {
  in_iterator: D,
  doc: i32,
  next_skipped_doc: i32,
  max_doc: i32,
}

impl<D: DocIdSetIterator> NotDocDocIdSetIterator<D> {
  fn new(in_iterator: D, max_doc: i32) -> Self {
    NotDocDocIdSetIterator {
      in_iterator,
      doc: -1,
      next_skipped_doc: -1,
      max_doc,
    }
  }
}

impl<D: DocIdSetIterator> DocIdSetIterator for NotDocDocIdSetIterator<D> {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.advance(self.doc + 1)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.doc = target;
    if self.doc > self.next_skipped_doc {
      self.next_skipped_doc = self.in_iterator.advance(self.doc)?;
    }
    loop {
      if self.doc >= self.max_doc {
        self.doc = NO_MORE_DOCS;
        break;
      }
      debug_assert!(self.doc <= self.next_skipped_doc);
      if self.doc != self.next_skipped_doc {
        return Ok(self.doc);
      }
      self.doc += 1;
      self.next_skipped_doc = self.in_iterator.next_doc()?;
    }
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.max_doc as i64)
  }
}

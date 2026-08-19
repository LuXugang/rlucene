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
use crate::core::search::doc_id_set_iterator::{AllDISI, DocIdSetIterator, EmptyDISI};
use crate::core::util::accountable::Accountable;
use crate::core::util::bits::{Bits, MatchAllBits};
use crate::core::util::dummy::dummy_bits::DummyBits;
use crate::core::util::error::lucene_error::Result;

/// A `DocIdSet` contains a set of document IDs.
/// Implementing types must provide an [`iterator`](DocIdSet::iterator) method
/// to access the set.
pub trait DocIdSet: Accountable {
  type DocIdSetIterator: DocIdSetIterator;
  fn iterator(&self) -> Result<Self::DocIdSetIterator>;

  /// Optionally provides a [`Bits`] view for random access to matching
  /// documents.
  ///
  /// # Returns
  /// * `None` if this `DocIdSet` does not support random access.
  ///
  /// Note that, unlike [`iterator`](DocIdSet::iterator), a return value of
  /// `None` **does not** imply that no documents match the filter!
  ///
  /// The default implementation does not provide random access, so you only
  /// need to implement this method if your [`DocIdSet`] can guarantee
  /// random access to every document ID in `O(1)` time without external
  /// disk access because the [`Bits`] trait cannot return an I/O error.
  /// This is generally true for bit sets like
  /// [`FixedBitSet`](crate::core::util::fixed_bit_set::FixedBitSet),
  /// which return themselves if used as a [`DocIdSet`].
  type Bits: Bits + Clone;
  fn bits(&self) -> Option<Self::Bits>;

  /// Some implementations require calling the finish method before invoking iterator.
  /// # See
  /// [`DocsWithFieldSet`](crate::core::index::docs_with_field_set::DocsWithFieldSet)
  fn finish(&mut self) {}
}

/// A [`DocIdSet`] that matches all document IDs up to a specified document
/// (exclusive).
pub struct All {
  max_doc: i32,
  bits: Option<MatchAllBits>,
}
impl All {
  fn new(max_doc: i32) -> Self {
    let bits = Some(MatchAllBits::new(max_doc as usize));
    All { max_doc, bits }
  }
}

/// Returns a [`DocIdSet`] that matches all document IDs below `max_doc`.
pub fn all(max_doc: i32) -> All {
  All::new(max_doc)
}

/// A `DocIdSet` that matches all doc ids up to a specified doc (exclusive).
impl DocIdSet for All {
  type DocIdSetIterator = AllDISI;

  fn iterator(&self) -> Result<Self::DocIdSetIterator> {
    Ok(AllDISI::new(self.max_doc))
  }

  type Bits = MatchAllBits;

  fn bits(&self) -> Option<Self::Bits> {
    self.bits.clone()
  }
}

impl Accountable for All {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(0)
  }
}

pub struct EmptyDocIdSet;
impl Accountable for EmptyDocIdSet {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(0)
  }
}
impl DocIdSet for EmptyDocIdSet {
  type DocIdSetIterator = EmptyDISI;

  fn iterator(&self) -> Result<Self::DocIdSetIterator> {
    Ok(EmptyDISI::new())
  }

  type Bits = DummyBits;

  fn bits(&self) -> Option<Self::Bits> {
    None
  }
}

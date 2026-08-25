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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::error::lucene_error::Result;
/// Decorator trait for a [`DocIdSetIterator`] that filters or validates documents on demand.
pub trait FilteredDocIdSetIterator: DocIdSetIterator {
  type DocIdSetIterator: DocIdSetIterator;
  fn base(&self) -> &FilteredDocIdSetIteratorBase<Self::DocIdSetIterator>;
  fn base_mut(&mut self) -> &mut FilteredDocIdSetIteratorBase<Self::DocIdSetIterator>;
  /// Validation method to determine whether a docid should be in the result set.
  ///
  /// # Arguments
  ///
  /// * `doc` - docid to be tested
  ///
  /// # Returns
  ///
  /// `true` if input docid should be in the result set, `false` otherwise.
  ///
  /// # See also
  ///
  /// [`FilteredDocIdSetIterator`]
  fn match_(&mut self, doc: i32) -> Result<bool>;

  fn doc_id(&self) -> i32 {
    self.base().doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    loop {
      let base = self.base_mut();
      let doc = base.inner_iter.next_doc()?;
      base.doc = doc;
      if doc == NO_MORE_DOCS {
        return Ok(doc);
      }
      if self.match_(doc)? {
        return Ok(self.base().doc);
      }
    }
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    {
      let base = self.base_mut();
      base.doc = base.inner_iter.advance(target)?;
      if base.doc == NO_MORE_DOCS {
        return Ok(base.doc);
      }
    }

    if self.match_(self.base().doc)? {
      return Ok(self.base().doc);
    }

    loop {
      let base = self.base_mut();
      let doc = base.inner_iter.next_doc()?;
      base.doc = doc;
      if doc == NO_MORE_DOCS {
        return Ok(doc);
      }
      if self.match_(doc)? {
        return Ok(doc);
      }
    }
  }

  fn cost(&self) -> Result<i64> {
    self.base().inner_iter.cost()
  }
}
pub struct FilteredDocIdSetIteratorBase<D> {
  doc: i32,
  pub(crate) inner_iter: D,
}
impl<D> FilteredDocIdSetIteratorBase<D> {
  pub(crate) fn new(inner_iter: D) -> FilteredDocIdSetIteratorBase<D> {
    FilteredDocIdSetIteratorBase {
      doc: -1,
      inner_iter,
    }
  }
}

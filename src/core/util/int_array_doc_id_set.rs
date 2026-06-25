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
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::accountable::Accountable;
use crate::core::util::dummy::dummy_bits::DummyBits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::ram_usage_estimator::size_of_vec;
use std::mem;
use std::rc::Rc;

/// A doc id set based on a sorted `Vec<i32>`.
///
/// # Note
/// This is an internal API.
pub struct IntArrayDocIdSet {
  docs: Rc<Vec<i32>>,
  length: i32,
}
/// Builds an `IntArrayDocIdSet` from an `i32` array and its length.
///
/// # Arguments
/// * `docs` - A docs array whose length must be greater than the `len`
///   parameter. The array needs to be sorted from 0 (inclusive) to `len`
///   (exclusive), and the `len`-th doc in `docs` must be
///   [`DocIdSetIterator::NO_MORE_DOCS`](NO_MORE_DOCS).
/// * `len` - The valid docs length in the array.
impl IntArrayDocIdSet {
  pub fn new(docs: Vec<i32>, length: i32) -> Result<IntArrayDocIdSet> {
    if docs[length as usize] != NO_MORE_DOCS {
      return Err(LuceneError::illegal_argument(format!(
        "last value must be {NO_MORE_DOCS}"
      )));
    }
    debug_assert!(
      assert_array_sorted(&docs),
      "IntArrayDocIdSet need docs to be sorted:{}",
      docs
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(", ")
    );
    Ok(IntArrayDocIdSet {
      docs: Rc::new(docs),
      length,
    })
  }
}
fn assert_array_sorted(docs: &[i32]) -> bool {
  docs.windows(2).all(|w| w[0] < w[1])
}

impl DocIdSet for IntArrayDocIdSet {
  type DocIdSetIterator = IntArrayDocIdSetIterator;

  fn iterator(&self) -> Result<Self::DocIdSetIterator> {
    Ok(IntArrayDocIdSetIterator::new(
      self.docs.clone(),
      self.length,
    ))
  }

  type Bits = DummyBits;

  fn bits(&self) -> Option<Self::Bits> {
    None
  }
}

impl Accountable for IntArrayDocIdSet {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(
      (mem::size_of_val(self.docs.as_ref()) as i64).saturating_add(size_of_vec(self.docs.as_ref())),
    )
  }
}

pub struct IntArrayDocIdSetIterator {
  docs: Rc<Vec<i32>>,
  length: i32,
  i: i32,
  doc: i32,
}
impl IntArrayDocIdSetIterator {
  pub fn new(docs: Rc<Vec<i32>>, length: i32) -> IntArrayDocIdSetIterator {
    IntArrayDocIdSetIterator {
      docs,
      length,
      i: 0,
      doc: -1,
    }
  }
}
impl DocIdSetIterator for IntArrayDocIdSetIterator {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.doc = self.docs[self.i as usize];
    self.i += 1;
    Ok(self.doc)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    let mut bound = 1;
    // given that we use this for small arrays only, this is very unlikely
    // to overflow
    while (self.i + bound < self.length) && (self.docs[self.i as usize + bound as usize] < target) {
      bound *= 2;
    }
    let mut start = self.i as usize + (bound / 2) as usize;
    let end = std::cmp::min(self.i + bound + 1, self.length) as usize;
    let index = self.docs[start..end]
      .binary_search(&target)
      .unwrap_or_else(|index| index);
    start += index;
    self.doc = self.docs[start];
    self.i = start as i32 + 1;
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.length as i64)
  }
}

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

pub struct MinDocIterator {
  segment_min_doc: i32,
  max_doc: i32,
  doc: i32,
}

impl MinDocIterator {
  pub fn new(segment_min_doc: i32, max_doc: i32) -> Self {
    Self {
      segment_min_doc,
      max_doc,
      doc: -1,
    }
  }
}
impl crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for MinDocIterator {}
impl DocIdSetIterator for MinDocIterator {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.advance(self.doc + 1)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    debug_assert!(target > self.doc);
    if self.doc == -1 {
      // skip directly to minDoc
      self.doc = target.max(self.segment_min_doc);
    } else {
      self.doc = target;
    }
    if self.doc >= self.max_doc {
      self.doc = NO_MORE_DOCS;
    }
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    Ok((self.max_doc - self.segment_min_doc) as i64)
  }
}

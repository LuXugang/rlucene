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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::term_states::TermStates;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{MultiTermQuery, MultiTermQueryEnum};
use crate::core::search::query::Query;
use crate::core::search::term_collecting_rewrite::TermCollectingRewrite;
use crate::core::util::SharedCounter;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::bytes_ref_hash::{BytesStartArray, DirectBytesStartArray};
use crate::core::util::error::lucene_error::Result;

pub trait ScoringRewrite: TermCollectingRewrite {
  fn default_rewrite<IRC, Q>(self, _index_searcher: &IndexSearcher<IRC>, _query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQueryEnum>,
    IRC: IndexReaderContext,
    Self: Sized,
  {
    todo!()
  }

  fn check_max_clause_count() -> Result<()>;
}

struct ParallelArraysTermCollector {}
/// Special implementation of BytesStartArray that keeps parallel arrays for boost and docFreq
struct TermFreqBoostByteStart {
  base: DirectBytesStartArray,
  boost: Vec<f32>,
  term_state: Vec<TermStates>,
}
impl TermFreqBoostByteStart {
  fn new(init_size: usize) -> Self {
    Self {
      base: DirectBytesStartArray::new(init_size),
      boost: Vec::new(),
      term_state: Vec::new(),
    }
  }
}
impl BytesStartArray for TermFreqBoostByteStart {
  fn init(&mut self) {
    self.base.init();
    let len = self.base.bytes_start.as_slice().len();
    self.boost = vec![0.0; len];
    self.term_state = vec![std::default::Default::default(); len];

    debug_assert!(self.term_state.len() >= len);
    debug_assert!(self.boost.len() >= len);
  }

  fn grow(&mut self) -> Result<()> {
    self.base.grow()?;
    let ord = self.base.bytes_start.as_slice();
    let len = ord.len();
    ArrayUtil::grow_with_len(&mut self.boost, len);
    if self.term_state.len() < len {
      self
        .term_state
        .resize(len, std::default::Default::default());
    }
    Ok(())
  }

  fn clear(&mut self) {
    self.boost.clear();
    self.term_state.clear();
    self.base.clear();
  }

  fn bytes_used(&mut self) -> SharedCounter {
    self.base.bytes_used()
  }

  fn get_value(&self, index: usize) -> i32 {
    self.base.get_value(index)
  }

  fn set_value(&mut self, index: usize, value: i32) {
    self.base.set_value(index, value);
  }

  fn len(&self) -> usize {
    self.base.len()
  }

  fn need_init(&self) -> bool {
    self.base.need_init()
  }
}

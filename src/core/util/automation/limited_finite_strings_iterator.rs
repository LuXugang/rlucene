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
use std::borrow::Cow;

use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::finite_strings_iterator::{
  FiniteStringsIterator, FiniteStringsIteratorBase,
};
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::ints_ref::IntsRef;
/// [`FiniteStringsIterator`] that limits the number of iterated accepted
/// strings. If more than `limit` strings are accepted, only the first `limit`
/// strings found are returned.
///
/// If the [`Automaton`] has cycles, this iterator may return an error,
/// though this is not guaranteed.
///
/// Be aware that the iteration order is implementation dependent and may change
/// across releases.
#[derive(Debug)]
pub struct LimitedFiniteStringsIterator<'a> {
  /// Maximum number of finite strings to create.
  limit: i32,
  /// Number of generated finite strings.
  count: i32,
  base: FiniteStringsIterator<'a>,
}
impl<'a> LimitedFiniteStringsIterator<'a> {
  pub fn new(automaton: &'a Automaton, limit: i32) -> Result<Self> {
    if limit != -1 && limit <= 0 {
      return Err(LuceneError::illegal_argument(format!(
        "limit must be -1 (which means no limit), or > 0; got: {limit}"
      )));
    }

    Ok(Self {
      limit: if limit > 0 { limit } else { i32::MAX },
      count: 0,
      base: FiniteStringsIterator::new(automaton),
    })
  }

  /// Number of iterated finite strings so far
  pub fn size(&self) -> i32 {
    self.count
  }
}
impl FiniteStringsIteratorBase for LimitedFiniteStringsIterator<'_> {
  fn next(&mut self) -> Result<Option<Cow<'_, IntsRef<Vec<i32>>>>> {
    if self.count >= self.limit {
      return Ok(None);
    }

    if let Some(result) = self.base.next()? {
      self.count += 1;
      Ok(Some(result))
    } else {
      Ok(None)
    }
  }
}

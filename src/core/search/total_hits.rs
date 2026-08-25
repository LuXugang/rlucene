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
use std::fmt;

/// Description of the total number of hits of a query. The total hit count
/// can't generally be computed accurately without visiting all matches, which
/// is costly for queries that match lots of documents. Given that it is often
/// enough to have a lower bound of the number of hits, such as "there are more
/// than 1000 hits", Lucene has options to stop counting as soon as a threshold
/// has been reached in order to improve query times.
///
/// # Parameters
///
/// - `value`: The value of the total hit count. Must be interpreted in the
///   context of [`Relation`].
/// - `relation`: Whether `value` is the exact hit count (in which case
///   [`Relation`] is equal to [`Relation::EqualTo`]), or a lower bound of the
///   total hit count (in which case [`Relation`] is equal to
///   [`Relation::GreaterThanOrEqualTo`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TotalHits {
  pub value: usize,
  pub relation: Relation,
}
impl Default for TotalHits {
  fn default() -> Self {
    Self {
      value: 0,
      relation: Relation::EqualTo,
    }
  }
}
impl TotalHits {
  pub fn new(value: usize, relation: Relation) -> Self {
    Self { value, relation }
  }
  pub fn value(&self) -> usize {
    self.value
  }
  pub fn relation(&self) -> Relation {
    self.relation
  }
}

impl fmt::Display for TotalHits {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.relation {
      Relation::EqualTo => write!(f, "{} hits", self.value),
      Relation::GreaterThanOrEqualTo => write!(f, "{}+ hits", self.value),
    }
  }
}
/// How the [`TotalHits::value`](crate::core::search::total_hits::TotalHits::value) should be interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Relation {
  /// The total hit count is equal to [`TotalHits::value`](crate::core::search::total_hits::TotalHits::value).
  EqualTo,
  /// The total hit count is greater than or equal to [`TotalHits::value`](crate::core::search::total_hits::TotalHits::value).
  GreaterThanOrEqualTo,
}

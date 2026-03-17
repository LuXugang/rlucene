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
use crate::core::search::scorer::Scorer;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::priority_queue::{Compare, PriorityQueue};

pub(crate) struct ScorerUtil;
impl ScorerUtil {
  /// the idea here is the following: a boolean query c1,c2,...cn with minShouldMatch=m
  /// could be rewritten to:
  /// (c1 AND (c2..cn|msm=m-1)) OR (!c1 AND (c2..cn|msm=m))
  /// if we assume that clauses come in ascending cost, then
  /// the cost of the first part is the cost of c1 (because the cost of a conjunction is
  /// the cost of the least costly clause)
  /// the cost of the second part is the cost of finding m matches among the c2...cn
  /// remaining clauses
  /// since it is a disjunction overall, the total cost is the sum of the costs of these
  /// two parts
  /// If we recurse infinitely, we find out that the cost of a msm query is the sum of the
  /// costs of the num_scorers - minShouldMatch + 1 least costly scorers
  pub fn cost_with_min_should_match<I>(
    costs: I,
    num_scorers: usize,
    min_should_match: usize,
  ) -> Result<i64>
  where
    I: IntoIterator<Item = i64>,
  {
    let k = num_scorers - min_should_match + 1;
    let mut pq = PriorityQueue::new(k, MaxCostCmp)?;
    for c in costs {
      let _ = pq.insert_with_overflow(c)?;
    }
    let mut sum = 0;
    for v in pq.iter() {
      sum += v;
    }
    Ok(sum)
  }

  #[inline]
  pub fn doc_id(s: &impl Scorer) -> i32 {
    s.approximation().doc_id()
  }
  #[inline]
  pub fn next_doc(s: &mut impl Scorer) -> Result<i32> {
    s.approximation_mut().next_doc()
  }
  #[inline]
  pub fn advance(s: &mut impl Scorer, target: i32) -> Result<i32> {
    s.approximation_mut().advance(target)
  }
  #[inline]
  pub fn slow_advance(s: &mut impl Scorer, target: i32) -> Result<i32> {
    s.approximation_mut().slow_advance(target)
  }
  #[inline]
  pub fn cost(s: &impl Scorer) -> Result<i64> {
    s.approximation().cost()
  }
}
struct MaxCostCmp;
impl Compare<i64> for MaxCostCmp {
  fn less_than(&self, a: &i64, b: &i64) -> Result<bool> {
    Ok(a > b)
  }
}

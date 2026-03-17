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
use crate::core::search::disi_priority_queue::DisiPriorityQueue;
use crate::core::search::disi_wrapper::DisiWrapper;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::scorer::Scorer;
use crate::core::search::scorer_util::ScorerUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub struct DisjunctionDISIApproximation<S>
where
  S: Scorer,
{
  pub(crate) all_scores: Vec<DisiWrapper<S>>,
  pub(crate) sub_iterators: DisiPriorityQueue,
  pub(crate) cost: i64,
}
impl<S> DisjunctionDISIApproximation<S>
where
  S: Scorer,
{
  pub fn new(sub_iterators: DisiPriorityQueue, all_scores: Vec<DisiWrapper<S>>) -> Self {
    let mut cost = 0i64;
    for idx in sub_iterators.iter() {
      cost += all_scores[idx].cost;
    }
    Self {
      all_scores,
      sub_iterators,
      cost,
    }
  }
}
impl<S> DocIdSetIterator for DisjunctionDISIApproximation<S>
where
  S: Scorer,
{
  fn doc_id(&self) -> i32 {
    let top_idx = self.sub_iterators.top().expect("top ie empty");
    self.all_scores[top_idx].doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    let mut top_idx = self
      .sub_iterators
      .top()
      .ok_or_else(|| LuceneError::illegal_state("no top available"))?;
    let old_doc = self.all_scores[top_idx].doc;

    loop {
      let top = &mut self.all_scores[top_idx];
      let v = ScorerUtil::next_doc(&mut top.scorer)?;
      top.doc = v;
      top_idx = self.sub_iterators.update_top(&self.all_scores);
      if self.all_scores[top_idx].doc != old_doc {
        break;
      }
    }
    Ok(self.all_scores[top_idx].doc)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    let mut top_idx = self
      .sub_iterators
      .top()
      .ok_or_else(|| LuceneError::illegal_state("no top available"))?;
    loop {
      let top = &mut self.all_scores[top_idx];
      let v = ScorerUtil::next_doc(&mut top.scorer)?;
      top.doc = v;
      top_idx = self.sub_iterators.update_top(&self.all_scores);
      if self.all_scores[top_idx].doc >= target {
        break;
      }
    }

    Ok(self.all_scores[top_idx].doc)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.cost)
  }
}

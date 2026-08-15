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
use crate::core::search::scorer::Scorer;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// Diff to Java Lucene, Compile-time polymorphism makes it unnecessary to wrap `likelyTermScorer`
/// or `likelyImpactsEnum`.
#[derive(Default)]
pub struct DisiWrapper<S> {
  pub(crate) scorer: S,
  pub(crate) next: Option<usize>,
  pub(crate) doc: i32,
  pub(crate) cost: i64,
  // the match cost for two-phase iterators, 0 otherwise
  pub(crate) match_cost: f32,
  // for MaxScoreBulkScorer
  pub(crate) scaled_max_score: i64,
  // for MaxScoreBulkScorer
  pub(crate) max_window_score: f32,
}
impl<S> DisiWrapper<S>
where
  S: Scorer,
{
  pub fn new(mut scorer: S) -> Result<Self> {
    let cost = scorer.iterator_mut().cost()?;
    let match_cost = match scorer.two_phase_iterator_mut() {
      Some(tpi) => tpi.match_cost(),
      None => 0.0,
    };
    Ok(Self {
      scorer,
      next: None,
      doc: -1,
      cost,
      match_cost,
      scaled_max_score: 0,
      max_window_score: 0.0,
    })
  }

  pub fn matches(&mut self) -> Result<bool> {
    match self.scorer.two_phase_iterator_mut() {
      Some(mut tpi) => tpi.matches(),
      None => Err(LuceneError::illegal_state(
        "this scorer does not support two-phase iteration",
      )),
    }
  }
  pub fn matches_may_none(&mut self) -> Result<bool> {
    match self.scorer.two_phase_iterator_mut() {
      Some(mut tpi) => tpi.matches(),
      None => Ok(true),
    }
  }
}

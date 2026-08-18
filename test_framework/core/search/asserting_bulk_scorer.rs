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
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::bulk_scorer::BulkScorerKind;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::query::QueryWeightSsBulkScorer;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::TryIntoInt;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::search::asserting_leaf_collector::AssertingLeafCollector;
use crate::test_framework::core::util::lucene_test_case::random_from_seed;
use rand::RngExt;

/// Wraps a BulkScorer with additional checks.
pub(crate) struct AssertingBulkScorer {
  random_seed: u64,
  in_: QueryWeightSsBulkScorer,
  max_doc: i32,
  _score_mode: ScoreMode,
  max: i32,
}

impl AssertingBulkScorer {
  pub(crate) fn wrap(
    random_seed: u64,
    in_: QueryWeightSsBulkScorer,
    max_doc: i32,
    score_mode: ScoreMode,
  ) -> QueryWeightSsBulkScorer {
    if matches!(in_.kind(), BulkScorerKind::Asserting) {
      return in_;
    }
    Box::new(Self {
      random_seed,
      in_,
      max_doc,
      _score_mode: score_mode,
      max: 0,
    })
  }

  #[allow(unused)]
  pub(crate) fn get_in(&self) -> &QueryWeightSsBulkScorer {
    &self.in_
  }
}

impl BulkScorer for AssertingBulkScorer {
  fn score(
    &mut self,
    collector: &mut dyn LeafCollector,
    accept_docs: Option<&dyn Bits>,
    min: i32,
    max: i32,
  ) -> Result<i32> {
    assert!(
      min >= self.max,
      "Scoring backward: min={} while previous max was max={}",
      min,
      self.max
    );
    assert!(
      min <= max,
      "max must be greater than min, got min={}, and max={}",
      min,
      max
    );
    self.max = max;

    let mut random = random_from_seed(self.random_seed);
    let mut next = min;
    let mut asserting_collector = AssertingLeafCollector::new(collector, min, max);
    while next < max {
      let up_to = if random.random_bool(0.5) {
        max
      } else {
        let interval: i64 = if random.random_range(0..100) <= 5 {
          1 + random.random_range(0..10)
        } else if random.random_bool(0.5) {
          1 + random.random_range(0..100)
        } else {
          1 + random.random_range(0..5000)
        };
        (next as i64 + interval).min(max as i64).try_convert()?
      };
      let mut interval_collector =
        AssertingLeafCollector::new(&mut asserting_collector, next, up_to);
      next = self
        .in_
        .score(&mut interval_collector, accept_docs, next, up_to)?;
    }

    if max >= self.max_doc || next >= self.max_doc {
      assert_eq!(next, NO_MORE_DOCS);
      Ok(NO_MORE_DOCS)
    } else {
      Ok(random.random_range(max..=next))
    }
  }

  fn cost(&mut self) -> Result<i64> {
    let cost = self.in_.cost()?;
    Ok(cost)
  }

  fn kind(&self) -> BulkScorerKind {
    BulkScorerKind::Asserting
  }
}

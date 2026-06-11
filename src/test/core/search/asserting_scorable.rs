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
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::util::error::lucene_error::Result;

/// Wraps another Scorable and asserts that scores are reasonable.
pub(crate) struct AssertingScorable<'a> {
  in_: &'a mut dyn Scorable,
}

impl<'a> AssertingScorable<'a> {
  pub(crate) fn wrap(in_: &'a mut dyn Scorable) -> Self {
    Self { in_ }
  }
}

impl FixedScore for AssertingScorable<'_> {
  fn set_score(&mut self, score: f32) -> Result<()> {
    self.in_.set_score(score)
  }
}

impl Scorable for AssertingScorable<'_> {
  fn score(&mut self) -> Result<f32> {
    let score = self.in_.score()?;
    assert!(score >= 0.0, "score={}", score);
    Ok(score)
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    self.in_.set_min_competitive_score(min_score)
  }

  fn cost(&self) -> Result<i64> {
    self.in_.cost()
  }
}

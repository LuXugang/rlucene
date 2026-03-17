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
use crate::core::search::scorable::Scorable;
use crate::core::util::error::lucene_error::Result;

#[derive(Default)]
pub struct SimpleScorable {
  pub(crate) score: f32,
  pub(crate) min_competitive_score: f32,
}
impl SimpleScorable {
  pub(crate) fn set_score(&mut self, score: f32) {
    self.score = score
  }
}

impl Scorable for SimpleScorable {
  fn score(&mut self) -> Result<f32> {
    Ok(self.score)
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    self.min_competitive_score = min_score;
    Ok(())
  }
}

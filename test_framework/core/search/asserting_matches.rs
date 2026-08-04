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
use crate::core::search::matches::Matches;
use crate::core::search::query::{QueryWeightMatches, QueryWeightMatchesIterator};
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::search::asserting_matches_iterator::AssertingMatchesIterator;

/// An implementation of [`Matches`] with additional consistency checks.
pub struct AssertingMatches<'a> {
  in_: QueryWeightMatches<'a>,
}

impl<'a> AssertingMatches<'a> {
  pub fn new(in_: QueryWeightMatches<'a>) -> Self {
    Self { in_ }
  }
}

impl Matches for AssertingMatches<'_> {
  fn get_matches(&self, field: &str) -> Result<Option<QueryWeightMatchesIterator<'_>>> {
    Ok(self.in_.get_matches(field)?.map(|matches| {
      Box::new(AssertingMatchesIterator::new(matches)) as QueryWeightMatchesIterator
    }))
  }

  fn get_sub_matches(&self) -> Vec<&QueryWeightMatches<'_>> {
    vec![&self.in_]
  }

  fn field(&self) -> &[String] {
    self.in_.field()
  }
}

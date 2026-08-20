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
use crate::core::search::matches_iterator::MatchesIterator;
use crate::core::search::query::{Query, QueryWeightMatchesIterator};
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

pub(crate) struct AssertingMatchesIterator<'a> {
  in_: QueryWeightMatchesIterator<'a>,
  state: State,
}

#[derive(Debug, PartialEq)]
enum State {
  Unpositioned,
  Iterating,
  Exhausted,
}

impl<'a> AssertingMatchesIterator<'a> {
  pub(crate) fn new(in_: QueryWeightMatchesIterator<'a>) -> Self {
    Self {
      in_,
      state: State::Unpositioned,
    }
  }
}

impl MatchesIterator for AssertingMatchesIterator<'_> {
  fn next(&mut self) -> Result<bool> {
    assert_ne!(self.state, State::Exhausted);
    let more = self.in_.next()?;
    self.state = if more {
      State::Iterating
    } else {
      State::Exhausted
    };
    Ok(more)
  }

  fn start_position(&self) -> Result<i32> {
    assert_eq!(self.state, State::Iterating);
    self.in_.start_position()
  }

  fn end_position(&self) -> i32 {
    assert_eq!(self.state, State::Iterating);
    self.in_.end_position()
  }

  fn start_offset(&self) -> Result<i32> {
    assert_eq!(self.state, State::Iterating);
    self.in_.start_offset()
  }

  fn end_offset(&self) -> Result<i32> {
    assert_eq!(self.state, State::Iterating);
    self.in_.end_offset()
  }

  fn get_sub_matches(&mut self) -> Result<Option<QueryWeightMatchesIterator<'_>>> {
    assert_eq!(self.state, State::Iterating);
    self.in_.get_sub_matches()
  }

  fn get_query(&self) -> Arc<Query> {
    assert_eq!(self.state, State::Iterating);
    self.in_.get_query()
  }
}

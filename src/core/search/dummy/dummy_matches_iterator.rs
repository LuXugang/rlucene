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
use crate::core::search::query::Query;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

pub struct DummyMatchesIterator;
impl MatchesIterator for DummyMatchesIterator {
  fn next(&mut self) -> crate::core::util::error::lucene_error::Result<bool> {
    dummy_unreachable!()
  }

  fn start_position(&self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn end_position(&self) -> i32 {
    dummy_unreachable!()
  }

  fn start_offset(&self) -> crate::core::util::error::lucene_error::Result<i32> {
    dummy_unreachable!()
  }

  fn end_offset(&self) -> crate::core::util::error::lucene_error::Result<i32> {
    dummy_unreachable!()
  }

  type MatchesIterRef<'a>
    = DummyMatchesIterator
  where
    Self: 'a;

  fn get_sub_matches(&mut self) -> Result<Option<Self::MatchesIterRef<'_>>> {
    dummy_unreachable!()
  }

  fn get_query(&self) -> Arc<Query> {
    dummy_unreachable!()
  }
}

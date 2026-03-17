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
use crate::core::util::error::lucene_error::Result;

/// Reports the positions and optionally offsets of all matching terms
/// in a query for a single document.
///
/// To obtain a [`MatchesIterator`] for a particular field, call
/// [`Matches::get_matches`]. Note that you can call this method multiple
/// times to retrieve new iterators, but it is not thread-safe.
///
/// @lucene.experimental
pub trait Matches {
  type MatchesIterator: MatchesIterator;
  /// Returns a [`MatchesIterator`] over the matches for a single field,
  /// or `None` if there are no matches in that field.
  fn get_matches(&self, field: &str) -> Result<Option<Self::MatchesIterator>>;

  type Matches: Matches;
  /// Returns a collection of [`Matches`] that make up this instance;
  /// if it is not a composite, then this returns an empty list.
  fn get_sub_matches(&mut self) -> Vec<Self::Matches>;

  fn field(&self) -> &[String];
}

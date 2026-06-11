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
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::index::term::Term;
use crate::core::util::error::lucene_error::Result;

/// An interface defining the collection of postings information from the
/// leaves of a [`Spans`](crate::queries::span::spans::Spans)
///
/// @lucene.experimental
pub trait SpanCollector {
  /// Collect information from postings
  ///
  /// * `postings` a [`PostingsEnum`]
  /// * `position` the position of the PostingsEnum
  /// * `term`    the [`Term`] for this postings list
  fn collect_leaf<P: PostingsEnum>(
    &mut self,
    postings: &mut P,
    position: i32,
    term: &Term,
  ) -> Result<()>;

  /// Call to indicate that the driving Spans has moved to a new position
  fn reset(&mut self);
}

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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::util::error::lucene_error::Result;

/// Factory used by `SearcherManager` to create new [`IndexSearcher`] instances. The default
/// implementation just creates an [`IndexSearcher`] with no custom behavior:
///
/// ```text
/// fn new_searcher<IRC>(context: IRC, _previous: IRC) -> Result<IndexSearcher<IRC>>
/// where
///     IRC: IndexReaderContext,
/// {
///     IndexSearcher::new(context)
/// }
/// ```
///
/// You can pass your own factory instead if you want custom behavior, such as:
///
/// - Setting a custom scoring model: [`IndexSearcher::set_similarity`]
/// - Parallel per-segment search
/// - Returning custom subclasses of `IndexSearcher`, for example for distributed scoring
/// - Running queries to warm your [`IndexSearcher`] before it is used. Note: when using
///   near-realtime search you may also want to warm newly merged segments in the background,
///   outside of the reopen path.
///
/// @lucene.experimental
#[derive(Default)]
pub struct SearcherFactory {
  sub: SearcherFactoryEnum,
}
impl SearcherFactory {
  pub fn new(sub: SearcherFactoryEnum) -> SearcherFactory {
    SearcherFactory { sub }
  }
}

pub trait SearcherFactoryBase {
  /// Returns a new [`IndexSearcher`] over the given context.
  ///
  /// # Parameters
  /// - `context`: the reader context to create a new searcher for.
  /// - `_previous`: the reader context previously used to create a new searcher. This can be used
  ///   to find newly opened segments compared to the new context and warm the searcher up before
  ///   returning it.
  fn new_searcher<IRC>(context: IRC, _previous: IRC) -> Result<IndexSearcher<IRC>>
  where
    IRC: IndexReaderContext,
  {
    IndexSearcher::new(context)
  }
}

#[derive(Default)]
pub struct DefaultSearcherFactoryBase;
impl SearcherFactoryBase for DefaultSearcherFactoryBase {}

pub enum SearcherFactoryEnum {
  Default(DefaultSearcherFactoryBase),
}

impl Default for SearcherFactoryEnum {
  fn default() -> Self {
    SearcherFactoryEnum::Default(DefaultSearcherFactoryBase)
  }
}

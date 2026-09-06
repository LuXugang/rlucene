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
use crate::core::search::query::Query;
use crate::core::util::error::lucene_error::Result;
use crate::sandbox::search::term_automaton_query::{TermAutomatonQuery, TermAutomatonQueryBase};

#[allow(dead_code)] // for quick search
struct TestTermAutomatonQuery;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CustomTermAutomatonQuery;

impl TermAutomatonQueryBase for CustomTermAutomatonQuery {
  fn rewrite<IRC>(
    &self,
    _query: &TermAutomatonQuery,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Query>>
  where
    IRC: IndexReaderContext,
  {
    Ok(None)
  }
}

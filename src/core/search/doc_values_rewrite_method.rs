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
use crate::core::search::multi_term_query::{MultiTermQuery, MultiTermQueryEnum, RewriteMethod};
use crate::core::search::query::Query;
use crate::core::util::error::lucene_error::LuceneError;

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DocValuesRewriteMethod;
impl RewriteMethod for DocValuesRewriteMethod {
  fn rewrite<IRC, Q>(
    self,
    _index_searcher: &IndexSearcher<IRC>,
    _query: Q,
  ) -> crate::core::util::error::lucene_error::Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQueryEnum>,
    IRC: IndexReaderContext,
  {
    Err(LuceneError::unsupported_operation(
      "DocValuesRewriteMethod is not implemented",
    ))
  }
}

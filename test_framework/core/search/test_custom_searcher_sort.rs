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
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::index_searcher::{
  IndexSearcher, IndexSearcherBase, IndexSearcherDefaults,
};
use crate::core::search::query::Query;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::sort::Sort;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocs;
use crate::core::search::top_field_docs::TopFieldDocs;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestCustomSearcherSort;

pub(crate) struct CustomSearcher {
  switcher: i32,
}

impl CustomSearcher {
  pub(crate) fn new(switcher: i32) -> Self {
    Self { switcher }
  }
}

impl<IRC> IndexSearcherBase<IRC> for CustomSearcher
where
  IRC: IndexReaderContext,
{
  fn search(
    &self,
    searcher: &IndexSearcher<IRC>,
    query: Query,
    n_docs: usize,
  ) -> Result<TopDocs<ScoreDoc>>
  where
    IndexSearcher<IRC>: Sync,
  {
    let mut bq = Builder::new();
    bq.add(query, Occur::Must)?;
    bq.add(
      TermQuery::new(Term::from_text("mandant", self.switcher.to_string())),
      Occur::Must,
    )?;
    IndexSearcherDefaults::search(searcher, bq.build().into(), n_docs)
  }

  fn search_with_sort(
    &self,
    searcher: &IndexSearcher<IRC>,
    query: Query,
    n_docs: usize,
    sort: Arc<Sort>,
  ) -> Result<TopFieldDocs>
  where
    IndexSearcher<IRC>: Sync,
  {
    let mut bq = Builder::new();
    bq.add(query, Occur::Must)?;
    bq.add(
      TermQuery::new(Term::from_text("mandant", self.switcher.to_string())),
      Occur::Must,
    )?;
    IndexSearcherDefaults::search_with_sort(searcher, bq.build().into(), n_docs, sort)
  }
}

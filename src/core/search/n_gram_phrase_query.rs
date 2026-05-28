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
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::term::Term;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::phrase_query::{Builder as PhraseQueryBuilder, PhraseQuery};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::Result;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// This is a [`PhraseQuery`] which is optimized for n-gram phrase query. For example, when you
/// query "ABCD" on a 2-gram field, you may want to use NGramPhraseQuery rather than
/// [`PhraseQuery`], because NGramPhraseQuery will [`Query::rewrite`] the query to
/// "AB/0 CD/2", while [`PhraseQuery`] will query "AB/0 BC/1 CD/2" (where term/position).
#[derive(Debug, Clone)]
pub struct NGramPhraseQuery {
  id: Identity,
  n: usize,
  phrase_query: PhraseQuery,
}

impl NGramPhraseQuery {
  /// Creates a query with the given n-gram size.
  pub fn new(n: usize, phrase_query: PhraseQuery) -> Self {
    Self {
      id: Identity::new(),
      n,
      phrase_query,
    }
  }

  /// Return the n in n-gram.
  pub fn get_n(&self) -> usize {
    self.n
  }

  /// Return the list of terms.
  pub fn get_terms(&self) -> &[Term] {
    self.phrase_query.get_terms()
  }

  /// Return the list of relative positions that each term should appear at.
  pub fn get_positions(&self) -> &[usize] {
    self.phrase_query.get_positions()
  }
}

impl Eq for NGramPhraseQuery {}

impl PartialEq for NGramPhraseQuery {
  fn eq(&self, other: &Self) -> bool {
    self.n == other.n && self.phrase_query == other.phrase_query
  }
}

impl Hash for NGramPhraseQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    std::any::type_name::<Self>().hash(state);
    self.phrase_query.hash(state);
    self.n.hash(state);
  }
}

impl HasIdentity for NGramPhraseQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for NGramPhraseQuery {
  fn as_string(&self, field: &str) -> Result<String> {
    self.phrase_query.as_string(field)
  }

  fn create_weight<IRC>(
    self,
    searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    self.phrase_query.create_weight(searcher, score_mode, boost)
  }

  fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    let terms = self.phrase_query.get_terms();
    let positions = self.phrase_query.get_positions();

    let is_optimizable = self.phrase_query.get_slop() == 0
      && self.n >= 2
      && terms.len() >= 3
      && positions
        .windows(2)
        .all(|window| window[1] == window[0] + 1);

    if !is_optimizable {
      return self.phrase_query.rewrite(searcher);
    }
    let n = self.n;
    let terms = self.phrase_query.get_term_arc();
    drop(self);
    let terms = Arc::try_unwrap(terms).unwrap_or_else(|terms| terms.as_ref().clone());

    let terms_len = terms.len();
    let mut builder = PhraseQueryBuilder::new();

    for (i, term) in terms.into_iter().enumerate() {
      if i % n == 0 || i == terms_len - 1 {
        builder.add(term, i)?;
      }
    }
    Ok(builder.build()?.into())
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
    todo!()
  }
}

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
/// A [`Query`] wrapper that allows giving a boost to the wrapped query.
///
/// Boost values that are less than one will give less importance to this query
/// compared to other ones, while values that are greater than one will give
/// more importance to the scores returned by this query.
///
///
/// More complex boosts can be applied by using `FunctionScoreQuery` in the
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{IntoBoxQuery, Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct BoostQuery {
  id: Identity,
  query: Box<Query>,
  boost: f32,
}
impl BoostQuery {
  pub fn new<T>(query: T, boost: f32) -> Result<Self>
  where
    T: IntoBoxQuery,
  {
    let query = query.into_box_query();
    if !boost.is_finite() || boost < 0.0 || (boost == 0.0 && boost.is_sign_negative()) {
      return Err(LuceneError::illegal_argument(format!(
        "boost must be a positive float, got {:.1}",
        boost
      )));
    }
    Ok(Self {
      id: Identity::new(),
      query,
      boost,
    })
  }
  pub fn get_query(&self) -> &Query {
    &self.query
  }
  pub fn get_boost(&self) -> f32 {
    self.boost
  }
  pub fn into_inner(self) -> Query {
    *self.query
  }
}

impl PartialEq for BoostQuery {
  fn eq(&self, other: &Self) -> bool {
    self.boost.to_bits() == other.boost.to_bits() && self.query == other.query
  }
}
impl Eq for BoostQuery {}
impl QueryBase for BoostQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    let inner = self.query.to_string(field)?;
    let mut s = String::new();
    s.push('(');
    s.push_str(&inner);
    s.push(')');
    s.push('^');
    s.push_str(&format!("{:.1}", self.boost));
    Ok(s)
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
    self
      .query
      .create_weight(searcher, score_mode, self.boost * boost)
  }

  fn rewrite<IRC>(mut self, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    let query_id = self.query.identity().clone();

    let rewritten = self.query.rewrite(searcher)?;

    if self.boost == 1.0 {
      return Ok(rewritten);
    }

    let rewritten = match rewritten {
      Query::Boost(in_boost) => {
        return Ok(BoostQuery::new(in_boost.query, self.boost * in_boost.boost)?.into());
      },
      Query::MatchNoDocs(_) => {
        return Ok(rewritten);
      },
      other => other,
    };

    if self.boost == 0.0 && !matches!(rewritten, Query::ConstantScore(_)) {
      return Ok(BoostQuery::new(ConstantScoreQuery::new(rewritten), 0.0)?.into());
    }

    if &query_id != rewritten.identity() {
      return Ok(BoostQuery::new(rewritten, self.boost)?.into());
    }
    self.query = Box::new(rewritten);
    Ok(self.into())
  }

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    let query = self.into();
    let mut visitor = visitor.get_sub_visitor(Occur::Must, query);
    self.query.visit(&mut visitor)
  }
}

impl Hash for BoostQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.query.hash(state);
    self.boost.to_bits().hash(state);
  }
}

impl HasIdentity for BoostQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl crate::core::util::accountable::Accountable for BoostQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}

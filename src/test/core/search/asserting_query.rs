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
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{IntoBoxQuery, Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::search::asserting_weight::AssertingWeight;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random_from_seed;
use rand::RngExt;
use rand::prelude::StdRng;
use rand_xoshiro::rand_core::Rng;
use std::hash::{Hash, Hasher};

/// Assertion-enabled query.
#[derive(Clone, Debug)]
pub struct AssertingQuery {
  id: Identity,
  random_seed: u64,
  in_: Box<Query>,
}

impl AssertingQuery {
  /// Sole constructor.
  pub(crate) fn new<R, Q>(random: &mut R, query: Q) -> Self
  where
    R: Rng + ?Sized,
    Q: IntoBoxQuery,
  {
    Self {
      id: Identity::new(),
      random_seed: random.random(),
      in_: query.into_box_query(),
    }
  }

  /// Wrap a query if necessary.
  pub(crate) fn wrap(random: &mut StdRng, query: Query) -> Self {
    match query {
      Query::Asserting(q) => q,
      q => Self::new(random, q),
    }
  }

  pub(crate) fn get_random_seed(&self) -> u64 {
    self.random_seed
  }

  pub(crate) fn get_in(&self) -> &Query {
    &self.in_
  }
}

impl PartialEq for AssertingQuery {
  fn eq(&self, other: &Self) -> bool {
    self.in_.eq(&other.in_)
  }
}

impl Eq for AssertingQuery {}

impl Hash for AssertingQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.in_.hash(state);
  }
}

impl crate::core::util::HasIdentity for AssertingQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for AssertingQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    self.in_.to_string(field)
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
    assert!(boost >= 0.0);
    let mut random = random_from_seed(self.random_seed);
    let weight = self.in_.create_weight(searcher, score_mode, boost)?;
    Ok(Box::new(AssertingWeight::new(
      random.random(),
      weight,
      *score_mode,
    )))
  }

  fn rewrite<IRC>(mut self, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    let query_id = self.in_.identity().clone();
    let rewritten = self.in_.rewrite(searcher)?;
    if rewritten.identity() != &query_id {
      let mut random = random_from_seed(self.random_seed);
      Ok(Self::wrap(&mut random, rewritten).into())
    } else {
      self.in_ = rewritten.into();
      Ok(self.into())
    }
  }

  fn visit<QV>(&self, visitor: &QV)
  where
    QV: QueryVisitor,
  {
    self.in_.visit(visitor);
  }
}

impl IntoBoxQuery for AssertingQuery {
  fn into_box_query(self) -> Box<Query> {
    Box::new(self.into())
  }
}

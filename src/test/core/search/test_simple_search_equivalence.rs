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
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::disjunction_max_query::DisjunctionMaxQuery;
use crate::core::search::phrase_query::{Builder as PhraseQueryBuilder, PhraseQuery};
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::search::search_equivalence_test_base::{
  SearchEquivalenceTestBase, SearchEquivalenceTestBaseMeta,
};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use rand_chacha::rand_core::Rng;

pub struct TestSimpleSearchEquivalence {
  meta: SearchEquivalenceTestBaseMeta,
}

impl TestSimpleSearchEquivalence {
  fn new<R>(random: &mut R) -> Self
  where
    R: Rng + ?Sized,
  {
    Self {
      meta: SearchEquivalenceTestBaseMeta::new(random).expect(""),
    }
  }
}

impl SearchEquivalenceTestBase for TestSimpleSearchEquivalence {
  fn get_meta(&self) -> &SearchEquivalenceTestBaseMeta {
    &self.meta
  }
}

fn test_term_versus_boolean_or() -> Result<()> {
  let mut random = random();
  let case = TestSimpleSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let q1 = TermQuery::new(t1.clone());
  let mut q2 = BooleanQueryBuilder::new();
  q2.add(TermQuery::new(t1), Occur::Should)?;
  q2.add(TermQuery::new(t2), Occur::Should)?;
  case.assert_subset_of(&mut random, &q1.into(), &q2.build().into())
}

fn test_term_versus_boolean_req_opt() -> Result<()> {
  let mut random = random();
  let case = TestSimpleSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let q1 = TermQuery::new(t1.clone());
  let mut q2 = BooleanQueryBuilder::new();
  q2.add(TermQuery::new(t1), Occur::Must)?;
  q2.add(TermQuery::new(t2), Occur::Should)?;
  case.assert_subset_of(&mut random, &q1.into(), &q2.build().into())
}

fn test_boolean_req_excl_versus_term() -> Result<()> {
  let mut random = random();
  let case = TestSimpleSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let mut q1 = BooleanQueryBuilder::new();
  q1.add(TermQuery::new(t1.clone()), Occur::Must)?;
  q1.add(TermQuery::new(t2), Occur::MustNot)?;
  let q2 = TermQuery::new(t1);
  case.assert_subset_of(&mut random, &q1.build().into(), &q2.into())
}

fn test_boolean_and_versus_boolean_or() -> Result<()> {
  let mut random = random();
  let case = TestSimpleSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let mut q1 = BooleanQueryBuilder::new();
  q1.add(TermQuery::new(t1.clone()), Occur::Must)?;
  q1.add(TermQuery::new(t2.clone()), Occur::Must)?;
  let mut q2 = BooleanQueryBuilder::new();
  q2.add(TermQuery::new(t1), Occur::Should)?;
  q2.add(TermQuery::new(t2), Occur::Should)?;
  case.assert_subset_of(&mut random, &q1.build().into(), &q2.build().into())
}
fn test_disjunction_sum_versus_disjunction_max() -> Result<()> {
  let mut random = random();
  let case = TestSimpleSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let mut q1 = BooleanQueryBuilder::new();
  q1.add(TermQuery::new(t1.clone()), Occur::Should)?;
  q1.add(TermQuery::new(t2.clone()), Occur::Should)?;
  let q2 = DisjunctionMaxQuery::new(
    vec![TermQuery::new(t1).into(), TermQuery::new(t2).into()],
    0.5,
  )?;
  case.assert_same_set(&mut random, &q1.build().into(), &q2.into())
}
fn test_exact_phrase_versus_boolean_and() -> Result<()> {
  let mut random = random();
  let case = TestSimpleSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let q1 = PhraseQuery::from_bytes(0, t1.field(), vec![t1.bytes().clone(), t2.bytes().clone()])?;
  let mut q2 = BooleanQueryBuilder::new();
  q2.add(TermQuery::new(t1), Occur::Must)?;
  q2.add(TermQuery::new(t2), Occur::Must)?;
  case.assert_subset_of(&mut random, &q1.into(), &q2.build().into())
}
fn test_exact_phrase_versus_boolean_and_with_holes() -> Result<()> {
  let mut random = random();
  let case = TestSimpleSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let mut builder = PhraseQueryBuilder::new();
  builder.add(t1.clone(), 0)?;
  builder.add(t2.clone(), 2)?;
  let q1 = builder.build()?;
  let mut q2 = BooleanQueryBuilder::new();
  q2.add(TermQuery::new(t1), Occur::Must)?;
  q2.add(TermQuery::new(t2), Occur::Must)?;
  case.assert_subset_of(&mut random, &q1.into(), &q2.build().into())
}
fn test_phrase_versus_sloppy_phrase() -> Result<()> {
  let mut random = random();
  let case = TestSimpleSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let q1 = PhraseQuery::from_bytes(0, t1.field(), vec![t1.bytes().clone(), t2.bytes().clone()])?;
  let q2 = PhraseQuery::from_bytes(1, t1.field(), vec![t1.bytes().clone(), t2.bytes().clone()])?;
  case.assert_subset_of(&mut random, &q1.into(), &q2.into())
}
fn test_phrase_versus_sloppy_phrase_with_holes() -> Result<()> {
  let mut random = random();
  let case = TestSimpleSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let mut builder = PhraseQueryBuilder::new();
  builder.add(t1, 0)?;
  builder.add(t2, 2)?;
  let q1 = builder.clone().build()?;
  builder.set_slop(2);
  let q2 = builder.build()?;
  case.assert_subset_of(&mut random, &q1.into(), &q2.into())
}
#[test]
fn test_exact_phrase_versus_multi_phrase() -> Result<()> {
  // TODO IMPORTANT MultiPhraseQuery未实现
  Ok(())
}

#[test]
fn test_exact_phrase_versus_multi_phrase_with_holes() -> Result<()> {
  // TODO IMPORTANT MultiPhraseQuery未实现
  Ok(())
}

#[test]
fn test_sloppy_phrase_versus_boolean_and() -> Result<()> {
  // TODO IMPORTANT MultiPhraseQuery未实现
  Ok(())
}

#[test]
fn test_phrase_relative_positions() -> Result<()> {
  // TODO IMPORTANT MultiPhraseQuery未实现
  Ok(())
}

#[test]
fn test_sloppy_phrase_relative_positions() -> Result<()> {
  // TODO IMPORTANT MultiPhraseQuery未实现
  Ok(())
}

#[test]
fn test_boost_query_simplification() -> Result<()> {
  // TODO IMPORTANT MultiPhraseQuery未实现
  Ok(())
}

#[test]
fn test_boolean_boost_propagation() -> Result<()> {
  // TODO IMPORTANT MultiPhraseQuery未实现
  Ok(())
}

#[test]
fn test_boolean_or_vs_synonym() -> Result<()> {
  // TODO IMPORTANT MultiPhraseQuery未实现
  Ok(())
}

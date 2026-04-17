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
use crate::core::search::phrase_query::{Builder, PhraseQuery};
use crate::core::util::error::lucene_error::Result;
use crate::test::core::search::search_equivalence_test_base::{
  SearchEquivalenceTestBase, SearchEquivalenceTestBaseMeta,
};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use rand::RngExt;
use rand_chacha::rand_core::Rng;

pub struct TestSloppyPhraseQuery2 {
  meta: SearchEquivalenceTestBaseMeta,
}
impl TestSloppyPhraseQuery2 {
  fn new<R>(random: &mut R) -> Self
  where
    R: Rng + ?Sized,
  {
    Self {
      meta: SearchEquivalenceTestBaseMeta::new(random).expect(""),
    }
  }
}
impl SearchEquivalenceTestBase for TestSloppyPhraseQuery2 {
  fn get_meta(&self) -> &SearchEquivalenceTestBaseMeta {
    &self.meta
  }
}

fn test_increasing_sloppiness() -> Result<()> {
  let mut random = random();
  let case = TestSloppyPhraseQuery2::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  for i in 0..10 {
    let q1 = PhraseQuery::from_bytes(i, t1.field(), vec![t1.bytes().clone(), t2.bytes().clone()])?;
    let q2 = PhraseQuery::from_bytes(
      i + 1,
      t1.field(),
      vec![t1.bytes().clone(), t2.bytes().clone()],
    )?;
    case.assert_subset_of(&mut random, &q1.into(), &q2.into())?;
  }
  Ok(())
}

fn test_increasing_sloppiness_with_holes() -> Result<()> {
  let mut random = random();
  let case = TestSloppyPhraseQuery2::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  for i in 0..10 {
    let mut builder = Builder::new();
    builder.add(t1.clone(), 0)?;
    builder.add(t2.clone(), 2)?;
    builder.set_slop(i);
    let q1 = builder.clone().build()?;
    builder.set_slop(i + 1);
    let q2 = builder.build()?;
    case.assert_subset_of(&mut random, &q1.into(), &q2.into())?;
  }
  Ok(())
}

fn test_increasing_sloppiness3() -> Result<()> {
  let mut random = random();
  let case = TestSloppyPhraseQuery2::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let t3 = case.random_term(&mut random);
  for i in 0..10 {
    let q1 = PhraseQuery::from_bytes(
      i,
      t1.field(),
      vec![t1.bytes().clone(), t2.bytes().clone(), t3.bytes().clone()],
    )?
    .into();
    let q2 = PhraseQuery::from_bytes(
      i + 1,
      t1.field(),
      vec![t1.bytes().clone(), t2.bytes().clone(), t3.bytes().clone()],
    )?
    .into();
    case.assert_subset_of(&mut random, &q1, &q2)?;
    case.assert_subset_of(&mut random, &q1, &q2)?;
  }
  Ok(())
}

fn test_increasing_sloppiness3_with_holes() -> Result<()> {
  let mut random = random();
  let case = TestSloppyPhraseQuery2::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let t3 = case.random_term(&mut random);
  let pos1 = 1 + random.random_range(0..3);
  let pos2 = pos1 + 1 + random.random_range(0..3);
  for i in 0..10 {
    let mut builder = Builder::new();
    builder.add(t1.clone(), 0)?;
    builder.add(t2.clone(), pos1)?;
    builder.add(t3.clone(), pos2)?;
    builder.set_slop(i);
    let q1 = builder.clone().build()?;
    builder.set_slop(i + 1);
    let q2 = builder.build()?;
    case.assert_subset_of(&mut random, &q1.into(), &q2.into())?;
  }
  Ok(())
}

fn test_repetitive_increasing_sloppiness() -> Result<()> {
  let mut random = random();
  let case = TestSloppyPhraseQuery2::new(&mut random);
  let t = case.random_term(&mut random);
  for i in 0..10 {
    let q1 =
      PhraseQuery::from_bytes(i, t.field(), vec![t.bytes().clone(), t.bytes().clone()])?.into();
    let q2 =
      PhraseQuery::from_bytes(i + 1, t.field(), vec![t.bytes().clone(), t.bytes().clone()])?.into();
    case.assert_subset_of(&mut random, &q1, &q2)?;
  }
  Ok(())
}

fn test_repetitive_increasing_sloppiness_with_holes() -> Result<()> {
  let mut random = random();
  let case = TestSloppyPhraseQuery2::new(&mut random);
  let t = case.random_term(&mut random);
  for i in 0..10 {
    let mut builder = Builder::new();
    builder.add(t.clone(), 0)?;
    builder.add(t.clone(), 2)?;
    builder.set_slop(i);
    let q1 = builder.clone().build()?;
    builder.set_slop(i + 1);
    let q2 = builder.build()?;
    case.assert_subset_of(&mut random, &q1.into(), &q2.into())?;
  }
  Ok(())
}

fn test_repetitive_increasing_sloppiness3() -> Result<()> {
  let mut random = random();
  let case = TestSloppyPhraseQuery2::new(&mut random);
  let t = case.random_term(&mut random);
  for i in 0..10 {
    let q1 = PhraseQuery::from_bytes(
      i,
      t.field(),
      vec![t.bytes().clone(), t.bytes().clone(), t.bytes().clone()],
    )?
    .into();
    let q2 = PhraseQuery::from_bytes(
      i + 1,
      t.field(),
      vec![t.bytes().clone(), t.bytes().clone(), t.bytes().clone()],
    )?
    .into();
    case.assert_subset_of(&mut random, &q1, &q2)?;
    case.assert_subset_of(&mut random, &q1, &q2)?;
  }
  Ok(())
}
fn test_random_increasing_sloppiness() -> Result<()> {
  // TODO IMPORTANT MultiPhraseQuery 未实现
  Ok(())
}

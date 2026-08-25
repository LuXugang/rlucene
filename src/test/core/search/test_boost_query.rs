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
use crate::core::index::multi_reader::MultiReader;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::util::lucene_test_case::{new_searcher_with_reader, random};
use rand::RngExt;

#[allow(dead_code)] // for quick search
struct TestBoostQuery;

#[test]
fn test_validation() -> Result<()> {
  let err = BoostQuery::new(MatchAllDocsQuery::new(), -3.0).unwrap_err();
  match err {
    LuceneError::IllegalArgument(msg) => {
      assert_eq!("boost must be a positive float, got -3.0", msg.to_string())
    },
    _ => unreachable!("expected LuceneError::IllegalArgument"),
  }

  let err = BoostQuery::new(MatchAllDocsQuery::new(), -0.0).unwrap_err();
  match err {
    LuceneError::IllegalArgument(msg) => {
      assert_eq!("boost must be a positive float, got -0.0", msg.to_string())
    },
    _ => unreachable!("expected LuceneError::IllegalArgument"),
  }

  let err = BoostQuery::new(MatchAllDocsQuery::new(), f32::NAN).unwrap_err();
  match err {
    LuceneError::IllegalArgument(msg) => {
      assert_eq!("boost must be a positive float, got NaN", msg.to_string())
    },
    _ => unreachable!("expected LuceneError::IllegalArgument"),
  }

  Ok(())
}

#[test]
fn test_equals() -> Result<()> {
  let mut random = random();

  let boost = random.random::<f32>() * 3.0;
  let q1 = BoostQuery::new(MatchAllDocsQuery::new(), boost)?;
  let q2 = BoostQuery::new(MatchAllDocsQuery::new(), boost)?;
  assert_eq!(q1, q2);
  assert_eq!(q1.get_boost(), q2.get_boost());

  let mut boost2 = boost;
  while boost == boost2 {
    boost2 = random.random::<f32>() * 3.0;
  }

  let q3 = BoostQuery::new(MatchAllDocsQuery::new(), boost2)?;
  assert_ne!(q1, q3);

  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  let mut h1 = DefaultHasher::new();
  q1.hash(&mut h1);

  let mut h3 = DefaultHasher::new();
  q3.hash(&mut h3);

  assert_ne!(h1.finish(), h3.finish());

  Ok(())
}

#[test]
fn test_to_string() -> Result<()> {
  assert_eq!(
    "(foo:bar)^2.0",
    BoostQuery::new(TermQuery::new(Term::from_text("foo", "bar")), 2.0)?.to_string("")?
  );

  let mut builder = Builder::new();
  builder.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
  builder.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
  let bq = builder.build();

  assert_eq!(
    "(foo:bar foo:baz)^2.0",
    BoostQuery::new(bq, 2.0)?.to_string("")?
  );

  Ok(())
}

#[test]
fn test_rewrite() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let q = BoostQuery::new(PhraseQuery::from_terms_no_slop("foo", &["bar"])?, 2.0)?;
  let v: Query = BoostQuery::new(TermQuery::new(Term::from_text("foo", "bar")), 2.0)?.into();
  assert_eq!(v, searcher.rewrite(q)?);

  let q = BoostQuery::new(BoostQuery::new(MatchAllDocsQuery::new(), 3.0)?, 2.0)?;
  let v: Query = BoostQuery::new(MatchAllDocsQuery::new(), 6.0)?.into();
  assert_eq!(v, searcher.rewrite(q)?);

  let q = BoostQuery::new(MatchAllDocsQuery::new(), 0.0)?;
  let v: Query = BoostQuery::new(ConstantScoreQuery::new(MatchAllDocsQuery::new()), 0.0)?.into();
  assert_eq!(v, searcher.rewrite(q)?);

  Ok(())
}

#[test]
fn test_rewrite_bubbles_up_match_no_docs_query() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let query = BoostQuery::new(MatchNoDocsQuery::new(), 2.0)?;
  let v: Query = MatchNoDocsQuery::new().into();
  assert_eq!(v, searcher.rewrite(query)?);

  let query = BoostQuery::new(MatchNoDocsQuery::new(), 0.0)?;
  let v: Query = MatchNoDocsQuery::new().into();
  assert_eq!(v, searcher.rewrite(query)?);

  Ok(())
}

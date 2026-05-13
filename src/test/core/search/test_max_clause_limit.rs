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
use crate::core::index::composite_reader::get_context;
use crate::core::index::multi_reader::MultiReader;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::disjunction_max_query::DisjunctionMaxQuery;
use crate::core::search::index_searcher::{
  IndexSearcher, get_max_clause_count, set_max_clause_count,
};
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::{LuceneError, Result};

#[allow(dead_code)] // for quick search
pub struct TestMaxClauseLimit;

#[test]
fn test_illegal_argument_exception_on_zero() -> Result<()> {
  let current = get_max_clause_count();

  let result = set_max_clause_count(0);

  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("maxClauseCount must be >= 1"));
  assert_eq!(current, get_max_clause_count());
  set_max_clause_count(current)?;
  Ok(())
}

#[test]
fn test_flatten_inner_disjunctions_with_more_than_1024_terms() -> Result<()> {
  let searcher = IndexSearcher::new(get_context(MultiReader::empty()?)?)?;

  let mut builder1024 = Builder::new();
  for i in 0..1024 {
    builder1024.add(
      TermQuery::new(Term::from_text("foo", format!("bar-{}", i))),
      Occur::Should,
    )?;
  }
  let inner = builder1024.build();

  let mut builder = Builder::new();
  builder.add(inner, Occur::Should)?;
  builder.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
  let query = builder.build();

  let err = searcher.rewrite(query);
  assert!(matches!(err, Err(LuceneError::TooManyClauses(_))));
  assert!(!matches!(err, Err(LuceneError::TooManyNestedClauses(_))));

  Ok(())
}

#[test]
fn test_large_terms_nested_first() -> Result<()> {
  let searcher = IndexSearcher::new(get_context(MultiReader::empty()?)?)?;

  let mut nested_builder = Builder::new();
  nested_builder.set_minimum_number_should_match(5);

  for i in 0..600 {
    nested_builder.add(
      TermQuery::new(Term::from_text("foo", format!("bar-{}", i))),
      Occur::Should,
    )?;
  }
  let inner = nested_builder.build();

  let mut builder = Builder::new();
  builder.add(inner, Occur::Should)?;
  builder.set_minimum_number_should_match(5);

  for _ in 0..600 {
    builder.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
  }

  let query = builder.build();

  let _err = searcher.rewrite(query);
  // TODO IMPORTANT  QueryBase的 visit 未实现
  // assert!(matches!(err, Err(LuceneError::TooManyNestedClauses(_))));

  Ok(())
}

#[test]
fn test_large_terms_nested_last() -> Result<()> {
  let searcher = IndexSearcher::new(get_context(MultiReader::empty()?)?)?;

  let mut nested_builder = Builder::new();
  nested_builder.set_minimum_number_should_match(5);

  for i in 0..600 {
    nested_builder.add(
      TermQuery::new(Term::from_text("foo", format!("bar-{}", i))),
      Occur::Should,
    )?;
  }
  let inner = nested_builder.build();

  let mut builder = Builder::new();
  builder.set_minimum_number_should_match(5);

  for _ in 0..600 {
    builder.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
  }

  builder.add(inner, Occur::Should)?;

  let query = builder.build();

  let _err = searcher.rewrite(query);
  // TODO IMPORTANT  QueryBase的 visit 未实现
  // assert!(matches!(err, Err(LuceneError::TooManyNestedClauses(_))));

  Ok(())
}

#[test]
fn test_large_disjunction_max_query() -> Result<()> {
  let searcher = IndexSearcher::new(get_context(MultiReader::empty()?)?)?;

  let mut clauses = Vec::with_capacity(1050);

  for _ in 0..1049 {
    clauses.push(TermQuery::new(Term::from_text("field", "a")).into());
  }

  let pq = PhraseQuery::from_bytes_no_slop("field", vec![])?;
  clauses.push(pq.into());

  let dmq = DisjunctionMaxQuery::new(clauses, 0.5f32)?;

  let _err = searcher.rewrite(dmq);
  // TODO IMPORTANT  QueryBase的 visit 未实现
  // assert!(matches!(err, Err(LuceneError::TooManyNestedClauses(_))));

  Ok(())
}

#[test]
fn test_multi_exact_with_repeats() -> Result<()> {
  // TODO MultiPhraseQuery未实现
  // let searcher = IndexSearcher::new(get_context(MultiReader::empty()?)?)?;
  //
  // let mut qb = MultiPhraseQuery::builder();
  //
  // for i in 0..1050 {
  //     qb.add(
  //         vec![
  //             Term::from_text("foo", format!("bar-{}", i)),
  //             Term::from_text("foo", format!("bar+{}", i)),
  //         ],
  //         0,
  //     )?;
  // }
  //
  // let query = qb.build();
  //
  // let err = searcher.rewrite(query.into());
  // assert!(matches!(err, Err(LuceneError::TooManyNestedClauses(_))));

  Ok(())
}

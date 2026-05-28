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
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::string_field::StringField;
use crate::core::index::term::Term;
use crate::core::search::blended_term_query;
use crate::core::search::blended_term_query::{BooleanRewrite, DisjunctionMaxRewrite};
use crate::core::search::query::{Query, QueryBase};
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::query_utils::QueryUtils;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_searcher_with_reader, random,
};
use rand::RngExt;
#[allow(dead_code)] // for quick search
struct TestBlendedTermQuery;
#[test]
fn test_equals() -> Result<()> {
  let mut random = random();

  let t1 = Term::from_text("foo", "bar");

  let mut builder = blended_term_query::Builder::new();
  builder.add(t1.clone())?;
  let bt1: Query = builder.build()?.into();

  let mut builder = blended_term_query::Builder::new();
  builder.add(t1.clone())?;
  let bt2: Query = builder.build()?.into();

  QueryUtils::check_equal::<Query>(&bt1, &bt2);

  let mut builder = blended_term_query::Builder::new();
  builder.set_rewrite_method(BooleanRewrite);
  builder.add(t1.clone())?;
  let bt1: Query = builder.build()?.into();

  let mut builder = blended_term_query::Builder::new();
  builder.set_rewrite_method(DisjunctionMaxRewrite::default());
  builder.add(t1.clone())?;
  let bt2: Query = builder.build()?.into();

  QueryUtils::check_unequal::<Query>(&bt1, &bt2);

  let t2 = Term::from_text("foo", "baz");

  let mut builder = blended_term_query::Builder::new();
  builder.add(t1.clone())?;
  builder.add(t2.clone())?;
  let bt1: Query = builder.build()?.into();

  let mut builder = blended_term_query::Builder::new();
  builder.add(t2.clone())?;
  builder.add(t1.clone())?;
  let bt2: Query = builder.build()?.into();

  QueryUtils::check_equal::<Query>(&bt1, &bt2);

  let boost1 = random.random::<f32>();
  let boost2 = random.random::<f32>();

  let mut builder = blended_term_query::Builder::new();
  builder.add_with_boost(t1.clone(), boost1)?;
  builder.add_with_boost(t2.clone(), boost2)?;
  let bt1: Query = builder.build()?.into();

  let mut builder = blended_term_query::Builder::new();
  builder.add_with_boost(t2, boost2)?;
  builder.add_with_boost(t1, boost1)?;
  let bt2: Query = builder.build()?.into();

  QueryUtils::check_equal::<Query>(&bt1, &bt2);

  Ok(())
}
#[test]
fn test_to_string() -> Result<()> {
  let builder = blended_term_query::Builder::new();
  let query = builder.build()?;
  assert_eq!("Blended()", query.as_string("")?);

  let t1 = Term::from_text("foo", "bar");

  let mut builder = blended_term_query::Builder::new();
  builder.add(t1.clone())?;
  let query = builder.build()?;
  assert_eq!("Blended(foo:bar)", query.as_string("")?);

  let t2 = Term::from_text("foo", "baz");

  let mut builder = blended_term_query::Builder::new();
  builder.add(t1.clone())?;
  builder.add(t2.clone())?;
  let query = builder.build()?;
  assert_eq!("Blended(foo:bar foo:baz)", query.as_string("")?);

  let mut builder = blended_term_query::Builder::new();
  builder.add_with_boost(t1, 4.0)?;
  builder.add_with_boost(t2, 3.0)?;
  let query = builder.build()?;
  assert_eq!("Blended((foo:bar)^4.0 (foo:baz)^3.0)", query.as_string("")?);

  Ok(())
}
#[test]
fn test_blended_scores() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone());

  let mut doc = Document::new();
  doc.add(StringField::from_string("f", "a", Store::No)?);
  w.add_document(doc)?;

  for _ in 0..10 {
    let mut doc = Document::new();
    doc.add(StringField::from_string("f", "b", Store::No)?);
    w.add_document(doc)?;
  }

  let reader = w.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut builder = blended_term_query::Builder::new();
  builder.set_rewrite_method(DisjunctionMaxRewrite::new(0.0));
  builder.add(Term::from_text("f", "a"))?;
  builder.add(Term::from_text("f", "b"))?;
  let query: Query = builder.build()?.into();

  let top_docs = searcher.search(query, 20)?;
  assert_eq!(11, top_docs.total_hits.value());

  for score_doc in &top_docs.score_docs {
    assert_eq!(top_docs.score_docs[0].score, score_doc.score);
  }

  w.close()?;

  Ok(())
}

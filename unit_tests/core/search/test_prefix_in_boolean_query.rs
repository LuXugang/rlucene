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
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::prefix_query::PrefixQuery;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::DefaultIndexSearchCR;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_reader, new_string_field, random,
};
use rand::Rng;
use std::collections::HashMap;

const FIELD: &str = "name";
#[allow(dead_code)] // for quick search
pub struct TestPrefixInBooleanQuery;

fn set_up<R>(random: &mut R) -> Result<DefaultIndexSearchCR>
where
  R: Rng + ?Sized,
{
  let directory = new_directory_shared(random)?;
  let writer = RandomIndexWriter::new(random, directory.clone())?;

  let mut field_to_type = HashMap::new();

  let mut doc = Document::new();
  doc.add(new_string_field(
    random,
    FIELD,
    "meaninglessnames",
    Store::No,
    &mut field_to_type,
  )?);

  for _ in 0..5137 {
    writer.add_document(random, doc.clone())?;
  }

  let mut doc = Document::new();
  doc.add(new_string_field(
    random,
    FIELD,
    "tangfulin",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(random, doc)?;

  let mut doc = Document::new();
  doc.add(new_string_field(
    random,
    FIELD,
    "meaninglessnames",
    Store::No,
    &mut field_to_type,
  )?);

  for _ in 5138..11377 {
    writer.add_document(random, doc.clone())?;
  }

  let mut doc = Document::new();
  doc.add(new_string_field(
    random,
    FIELD,
    "tangfulin",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(random, doc)?;

  let reader = writer.get_reader(random)?;
  let searcher = new_searcher_with_reader(reader)?;

  writer.close(random)?;
  Ok(searcher)
}
#[test]
fn test_prefix_query() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let query = PrefixQuery::new(Term::from_text(FIELD, "tang"))?;
  assert_eq!(2, searcher.search(query, 1000)?.total_hits.value());

  Ok(())
}

#[test]
fn test_term_query() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let query = TermQuery::new(Term::from_text(FIELD, "tangfulin"));
  assert_eq!(2, searcher.search(query, 1000)?.total_hits.value());

  Ok(())
}

#[test]
fn test_term_boolean_query() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let mut query = Builder::new();
  query.add(
    TermQuery::new(Term::from_text(FIELD, "tangfulin")),
    Occur::Should,
  )?;
  query.add(
    TermQuery::new(Term::from_text(FIELD, "notexistnames")),
    Occur::Should,
  )?;

  assert_eq!(2, searcher.search(query.build(), 1000)?.total_hits.value());

  Ok(())
}

#[test]
fn test_prefix_boolean_query() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let mut query = Builder::new();
  query.add(
    PrefixQuery::new(Term::from_text(FIELD, "tang"))?,
    Occur::Should,
  )?;
  query.add(
    TermQuery::new(Term::from_text(FIELD, "notexistnames")),
    Occur::Should,
  )?;

  assert_eq!(2, searcher.search(query.build(), 1000)?.total_hits.value());

  Ok(())
}

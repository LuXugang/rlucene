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
use crate::core::index::BytesRef;
use crate::core::index::term::Term;
use crate::core::search::prefix_query::PrefixQuery;
use crate::core::util::StringHelper;
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
use crate::test::support::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_searcher_with_reader, new_string_field,
  new_string_field_binary, random,
};
use crate::test::support::core::util::test_util::TestUtil;
use rand::Rng;
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
struct TestPrefixQuery;
#[test]
fn test_prefix_query() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;

  let categories = ["/Computers", "/Computers/Mac", "/Computers/Windows"];
  let writer = RandomIndexWriter::new(&mut random, directory.clone())?;
  let mut field_to_type = HashMap::new();

  for cat in categories {
    let mut doc = Document::new();
    doc.add(new_string_field(
      &mut random,
      "category",
      cat,
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(&mut random, doc)?;
  }

  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let query = PrefixQuery::new(Term::from_text("category", "/Computers"))?;
  let hits = searcher.search(query.clone(), 1000)?.score_docs;
  assert_eq!(
    3,
    hits.len(),
    "All documents in /Computers category and below"
  );

  let query = PrefixQuery::new(Term::from_text("category", "/Computers/Mac"))?;
  let hits = searcher.search(query.clone(), 1000)?.score_docs;
  assert_eq!(1, hits.len(), "One in /Computers/Mac");

  let query = PrefixQuery::new(Term::from_text("category", ""))?;
  let hits = searcher.search(query.clone(), 1000)?.score_docs;
  assert_eq!(3, hits.len(), "everything");

  writer.close(&mut random)?;
  Ok(())
}
#[test]
fn test_match_all() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, directory.clone())?;
  let mut field_to_type = HashMap::new();

  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "field",
    "field",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;

  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let query = PrefixQuery::new(Term::from_text("field", ""))?;
  let top_docs = searcher.search(query, 1000)?;
  assert_eq!(1, top_docs.total_hits.value());

  writer.close(&mut random)?;
  Ok(())
}
#[test]
fn test_random_binary_prefix() -> Result<()> {
  use rand::seq::SliceRandom;
  use std::collections::HashSet;

  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();

  let num_terms = at_least(&mut random, 1000);
  let mut terms = HashSet::new();
  while terms.len() < num_terms as usize {
    let len = TestUtil::next_int(&mut random, 1, 10) as usize;
    let mut bytes = vec![0u8; len];
    random.fill_bytes(&mut bytes);
    terms.insert(BytesRef::from_bytes(bytes));
  }

  let mut terms_list: Vec<_> = terms.into_iter().collect();
  terms_list.shuffle(&mut random);

  for term in &terms_list {
    let mut doc = Document::new();
    doc.add(new_string_field_binary(
      &mut random,
      "field",
      term.clone(),
      Store::No,
      &mut field_to_type,
    )?);
    w.add_document(&mut random, doc)?;
  }

  let reader = w.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let iters = at_least(&mut random, 100);
  for _ in 0..iters {
    let len = (random.next_u32() % 3) as usize;
    let mut bytes = vec![0u8; len];
    random.fill_bytes(&mut bytes);
    let prefix = BytesRef::from_bytes(bytes);

    let q = PrefixQuery::new(Term::new("field", prefix.clone()))?;

    let mut count = 0;
    for term in &terms_list {
      if StringHelper::starts_with_byte_ref(term, &prefix) {
        count += 1;
      }
    }

    assert_eq!(count, searcher.count(q)?);
  }

  w.close(&mut random)?;
  Ok(())
}

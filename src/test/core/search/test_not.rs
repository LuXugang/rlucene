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
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_reader, new_text_field, random,
};
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
pub struct TestNot;

#[test]
fn test_not() -> Result<()> {
  let mut random = random();
  let store = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, store.clone());

  let mut field_to_type = HashMap::new();

  let mut d1 = Document::new();
  d1.add(new_text_field(
    &mut random,
    "field",
    "a b",
    Store::Yes,
    &mut field_to_type,
  )?);

  writer.add_document(&mut random, d1)?;

  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut query = Builder::new();
  query.add(TermQuery::new(Term::from_text("field", "a")), Occur::Should)?;
  query.add(
    TermQuery::new(Term::from_text("field", "b")),
    Occur::MustNot,
  )?;

  let hits = searcher.search(query.build(), 1000)?.score_docs;
  assert_eq!(0, hits.len());

  writer.close(&mut random)?;
  Ok(())
}

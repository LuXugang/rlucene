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
use crate::test::support::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_threads, random,
};

use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;

use crate::core::search::match_all_docs_query::MatchAllDocsQuery;

use crate::core::search::term_query::TermQuery;
use crate::core::search::total_hit_count_collector_manager::TotalHitCountCollectorManager;

use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;

#[allow(dead_code)] // for quick search
struct TestTotalHitCountCollector;

#[test]
fn test_basics() -> Result<()> {
  let mut random = random();
  let index_store = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, index_store.clone())?;

  for i in 0..5 {
    let mut doc = Document::new();
    doc.add(StringField::from_string(
      "string",
      format!("a{}", i),
      Store::No,
    )?);
    doc.add(StringField::from_string(
      "string",
      format!("b{}", i),
      Store::No,
    )?);
    writer.add_document(&mut random, doc)?;
  }

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  // TODO IMPORTANT Concurrency未实现
  let searcher = new_searcher_with_threads(&mut random, reader, true, true, true)?;
  let collector_manager = TotalHitCountCollectorManager::new(searcher.get_slices()?.as_slice());
  let mut total_hits =
    searcher.search_with_collector_manager(MatchAllDocsQuery::new(), &collector_manager)?;
  assert_eq!(5, total_hits);

  let mut builder = Builder::new();
  builder.add(
    TermQuery::new(Term::from_text("string", "a1")),
    Occur::Should,
  )?;
  builder.add(
    TermQuery::new(Term::from_text("string", "b3")),
    Occur::Should,
  )?;
  let query = builder.build();

  total_hits = searcher.search_with_collector_manager(query, &collector_manager)?;
  assert_eq!(2, total_hits);

  Ok(())
}

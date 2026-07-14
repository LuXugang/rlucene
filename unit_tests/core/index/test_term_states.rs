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
use crate::core::index::index_reader::IndexReader;
use crate::core::index::term::Term;
use crate::core::index::term_states::build;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{new_directory_shared, random};
use rand::RngExt;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestTermStates;
#[test]
fn test_to_string_on_null_term_state() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir)?;
  w.add_document(&mut random, Document::new())?;
  let reader = w.get_reader(&mut random)?;
  let reader = reader.get_context()?;
  let searcher = IndexSearcher::new(reader)?;
  let term = Term::from_text("foo", "bar");
  let needs_stats = random.random_bool(0.5);
  let states = build(&searcher, Arc::new(term), needs_stats)?;
  assert_eq!("TermStates\n  state=null\n", states.to_string());
  Ok(())
}

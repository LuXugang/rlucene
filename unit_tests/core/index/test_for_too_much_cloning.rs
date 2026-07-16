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
use crate::core::document::text_field::TextField;
use crate::core::index::BytesRef;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::tiered_merge_policy::TieredMergePolicy;
use crate::core::search::term_range_query::TermRangeQuery;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  new_index_writer_config_with_analyzer, new_mock_directory, new_searcher_with_wrap, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestForTooMuchCloning;

// Make sure we don't clone IndexInputs too frequently
// during merging and searching:
#[test]
fn test() -> Result<()> {
  let mut random = random();
  let dir = Arc::new(new_mock_directory(&mut random)?);
  let mut tmp = TieredMergePolicy::new();
  tmp.set_max_merge_at_once(2)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  config.set_max_buffered_docs(2).set_merge_policy(tmp);
  // Java uses a FilterMergePolicy here so RandomIndexWriter cannot randomly
  // reconfigure the merge policy. Rust RandomIndexWriter does not currently
  // perform that reconfiguration.
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), config);
  let num_docs = 20;
  for _ in 0..num_docs {
    let mut sb = String::new();
    for _ in 0..100 {
      sb.push_str(&TestUtil::random_realistic_unicode_string(&mut random));
      sb.push(' ');
    }
    let mut doc = Document::new();
    doc.add(TextField::from_string("field", sb, Store::No)?);
    w.add_document(&mut random, doc)?;
  }
  let r = Arc::new(w.get_reader(&mut random)?);
  w.close(&mut random)?;
  // println!("merge clone count={}", dir.get_input_clone_count());
  assert!(
    dir.get_input_clone_count() < 500,
    "too many calls to IndexInput::try_clone during merging: {}",
    dir.get_input_clone_count()
  );

  let s = new_searcher_with_wrap(&mut random, r.clone(), true)?;
  // important: set this after newSearcher, it might have run checkindex
  let clone_count = dir.get_input_clone_count();
  // dir.set_verbose_clone(true);

  // MTQ that matches all terms so the AUTO_REWRITE should
  // cutover to filter rewrite and reuse a single DocsEnum
  // across all terms;
  let hits = s.search(
    TermRangeQuery::new(
      "field",
      Some(BytesRef::default()),
      Some(BytesRef::from_string("\u{FFFF}")),
      true,
      true,
    )?,
    10,
  )?;
  assert!(hits.total_hits.value > 0);
  let query_clone_count = dir.get_input_clone_count() - clone_count;
  // println!("query clone count={query_clone_count}");
  // It is rather difficult to reliably predict how many query clone calls will be performed. One
  // important factor is the number of segment partitions being searched, but it depends as well
  // on the terms being indexed, and the distribution of the matches across the documents, which
  // affects how the query gets rewritten and the subsequent number of clone calls it will
  // perform.
  let max_partitions = s.get_leaf_contexts()?.len().max(s.get_slices()?.len()) as i32;
  assert!(
    query_clone_count <= max_partitions * 5,
    "too many calls to IndexInput::try_clone during TermRangeQuery: {query_clone_count}"
  );
  r.close()?;
  dir.close()?;
  Ok(())
}

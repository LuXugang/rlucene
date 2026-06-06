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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::wildcard_query::WildcardQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, new_searcher_with_reader,
  new_string_field, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
use std::collections::HashMap;

/// Create an index with terms from 000-999. Generates random wildcards according to patterns,
/// and validates the correct number of hits are returned.
#[allow(dead_code)] // for quick search
pub struct TestWildcardRandom;

fn fill_pattern(wildcard_pattern: &str, random: &mut impl RngExt) -> String {
  wildcard_pattern
    .chars()
    .map(|ch| match ch {
      'N' => (b'0' + random.random_range(0..10) as u8) as char,
      _ => ch,
    })
    .collect()
}

fn assert_pattern_hits<IRC>(
  searcher: &IndexSearcher<IRC>,
  random: &mut impl RngExt,
  pattern: &str,
  num_hits: usize,
) -> Result<()>
where
  IRC: IndexReaderContext + Sync,
{
  let wq = WildcardQuery::new(Term::from_text("field", fill_pattern(pattern, random)))?;
  let docs = searcher.search(wq, 25)?;
  assert_eq!(
    num_hits,
    docs.total_hits.value(),
    "Incorrect hits for pattern: {}",
    pattern
  );
  Ok(())
}

#[test]
fn test_wildcards() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_max_buffered_docs(TestUtil::next_int(&mut random, 50, 1000));

  let writer = RandomIndexWriter::with_config(&mut random, dir, iwc);
  let mut field_to_type = HashMap::new();
  let mut field = new_string_field(&mut random, "field", "", Store::No, &mut field_to_type)?;

  for i in 0..1000 {
    field.set_string_value(format!("{:03}", i))?;
    let mut doc = Document::new();
    doc.add(field.clone());
    writer.add_document(doc)?;
  }

  let reader = writer.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;
  writer.close()?;

  let num = at_least(&mut random, 1);
  for _ in 0..num {
    assert_pattern_hits(&searcher, &mut random, "NNN", 1)?;
    assert_pattern_hits(&searcher, &mut random, "?NN", 10)?;
    assert_pattern_hits(&searcher, &mut random, "N?N", 10)?;
    assert_pattern_hits(&searcher, &mut random, "NN?", 10)?;
  }

  for _ in 0..num {
    assert_pattern_hits(&searcher, &mut random, "??N", 100)?;
    assert_pattern_hits(&searcher, &mut random, "N??", 100)?;
    assert_pattern_hits(&searcher, &mut random, "???", 1000)?;

    assert_pattern_hits(&searcher, &mut random, "NN*", 10)?;
    assert_pattern_hits(&searcher, &mut random, "N*", 100)?;
    assert_pattern_hits(&searcher, &mut random, "*", 1000)?;

    assert_pattern_hits(&searcher, &mut random, "*NN", 10)?;
    assert_pattern_hits(&searcher, &mut random, "*N", 100)?;

    assert_pattern_hits(&searcher, &mut random, "N*N", 10)?;

    assert_pattern_hits(&searcher, &mut random, "?N*", 100)?;
    assert_pattern_hits(&searcher, &mut random, "N?*", 100)?;

    assert_pattern_hits(&searcher, &mut random, "*N?", 100)?;
    assert_pattern_hits(&searcher, &mut random, "*??", 1000)?;
    assert_pattern_hits(&searcher, &mut random, "*?N", 100)?;
  }

  Ok(())
}

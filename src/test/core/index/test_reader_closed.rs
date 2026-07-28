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
use crate::core::index::index_reader::IndexReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::parallel_leaf_reader::ParallelLeafReader;
use crate::core::index::standard_directory_reader::StandardDirectoryReader;
use crate::core::search::term_range_query::TermRangeQuery;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::analysis::mock_tokenizer;
use crate::test_framework::core::index::own_cache_key_multi_reader::OwnCacheKeyMultiReader;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, get_only_leaf_reader, new_directory_shared, new_index_writer_config_with_analyzer,
  new_searcher, new_string_field, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestReaderClosed;

fn set_up<R>(random: &mut R) -> Result<(Arc<DirEnum>, Arc<StandardDirectoryReader<DirEnum>>)>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let analyzer = MockAnalyzer::with_automaton(random, mock_tokenizer::KEYWORD.clone(), false);
  let mut config = new_index_writer_config_with_analyzer(random, analyzer)?;
  config.set_max_buffered_docs(TestUtil::next_int(random, 50, 1000));
  let writer = RandomIndexWriter::with_config(random, dir.clone(), config);

  let num = at_least(random, 10);
  let mut field_to_type = HashMap::new();
  for _ in 0..num {
    let mut doc = Document::new();
    let field_value = TestUtil::random_unicode_string_with_len(random, 10);
    doc.add(new_string_field(
      random,
      "field",
      field_value,
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(random, doc)?;
  }
  writer.force_merge(random, 1)?;
  let reader = Arc::new(writer.get_reader(random)?);
  writer.close(random)?;
  Ok((dir, reader))
}

#[test]
fn test() -> Result<()> {
  let mut random = random();
  let (dir, reader) = set_up(&mut random)?;
  assert!(reader.get_ref_count() > 0);
  let searcher = new_searcher(&mut random, reader.clone())?;
  let query = TermRangeQuery::new_string_range("field", Some("a"), Some("z"), true, true)?;
  searcher.search(query.clone(), 5)?;
  reader.close()?;
  match searcher.search(query, 5) {
    Ok(_) => {},
    Err(LuceneError::AlreadyClosed(_)) => {},
    Err(e) => return Err(e),
  }
  dir.close()
}

#[test]
fn test_reader_chaining() -> Result<()> {
  let mut random = random();
  let (dir, reader) = set_up(&mut random)?;
  assert!(reader.get_ref_count() > 0);
  let wrapped_reader = ParallelLeafReader::new(vec![get_only_leaf_reader(reader.clone())?])?;

  // We wrap with an OwnCacheKeyMultiReader so that closing the underlying reader
  // does not terminate the threadpool (if that index searcher uses one).
  let multi_reader = OwnCacheKeyMultiReader::new(vec![wrapped_reader])?;
  let searcher = new_searcher(&mut random, multi_reader)?;

  let query = TermRangeQuery::new_string_range("field", Some("a"), Some("z"), true, true)?;
  searcher.search(query.clone(), 5)?;
  reader.close()?;
  match searcher.search(query, 5) {
    Ok(_) => {},
    Err(LuceneError::AlreadyClosed(e)) => {
      assert_eq!(
        "this IndexReader cannot be used anymore as one of its child readers was closed",
        e.to_string()
      );
    },
    Err(e) => return Err(e),
  }
  searcher.get_index_reader().close()?;
  dir.close()
}

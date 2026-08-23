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
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::search::index_searcher::{self, IndexSearcher};
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::top_docs;
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::check_hits::CheckHits;
use crate::test_framework::core::util::DefaultCRReader;
use crate::test_framework::core::util::lucene_test_case::{
  is_light_mode, new_directory_shared, new_index_writer_config, new_log_merge_policy,
  new_search_executor, random,
};
use rand::{Rng, RngExt};
use std::sync::{Arc, LazyLock};
#[allow(dead_code)] // for quick search
pub struct TestMultiSliceMerge;
static LIGHT_DIRS: LazyLock<(Arc<DirEnum>, Arc<DirEnum>)> = LazyLock::new(|| {
  let mut random = random();
  build_set_up_dirs(&mut random).expect("failed to initialize TestMultiSliceMerge")
});

fn set_up_readers<R>(random: &mut R) -> Result<(DefaultCRReader, DefaultCRReader)>
where
  R: Rng + ?Sized,
{
  let (dir1, dir2) = if is_light_mode() {
    (LIGHT_DIRS.0.clone(), LIGHT_DIRS.1.clone())
  } else {
    build_set_up_dirs(random)?
  };
  Ok((directory_reader::open(dir1)?, directory_reader::open(dir2)?))
}

fn build_set_up_dirs<R>(random: &mut R) -> Result<(Arc<DirEnum>, Arc<DirEnum>)>
where
  R: Rng + ?Sized,
{
  let dir1 = new_directory_shared(random)?;
  let dir2 = new_directory_shared(random)?;

  let mut iwc1 = new_index_writer_config(random)?;
  iwc1.set_merge_policy(new_log_merge_policy(random)?);
  let iw1 = RandomIndexWriter::with_config(random, dir1.clone(), iwc1);

  for i in 0..100 {
    let mut doc = Document::new();

    doc.add(StringField::from_string("field", i.to_string(), Store::No)?);
    doc.add(StringField::from_string(
      "field2",
      (i % 2 == 0).to_string(),
      Store::No,
    )?);
    doc.add(SortedDocValuesField::new(
      "field2",
      BytesRef::from_string(&(i % 2 == 0).to_string()),
    ));

    iw1.add_document(random, doc)?;

    if random.random_bool(0.5) {
      iw1.get_reader(random)?.close()?;
    }
  }

  iw1.close(random)?;

  let mut iwc2 = new_index_writer_config(random)?;
  iwc2.set_merge_policy(new_log_merge_policy(random)?);
  let iw2 = RandomIndexWriter::with_config(random, dir2.clone(), iwc2);

  for i in 0..100 {
    let mut doc = Document::new();

    doc.add(StringField::from_string("field", i.to_string(), Store::No)?);
    doc.add(StringField::from_string(
      "field2",
      (i % 2 == 0).to_string(),
      Store::No,
    )?);
    doc.add(SortedDocValuesField::new(
      "field2",
      BytesRef::from_string(&(i % 2 == 0).to_string()),
    ));

    iw2.add_document(random, doc)?;

    if random.random_bool(0.5) {
      iw2.commit(random)?;
    }
  }

  iw2.close(random)?;

  Ok((dir1, dir2))
}
#[test]
fn test_multiple_slices_of_same_index_searcher() -> Result<()> {
  let mut random = random();
  let (reader1, reader2) = set_up_readers(&mut random)?;

  let searcher1 = index_searcher::from_reader_with_executor(
    reader1,
    new_search_executor(random.random_range(2..=5))?,
  )?;
  let searcher2 = index_searcher::from_reader_with_executor(
    reader2,
    new_search_executor(random.random_range(2..=5))?,
  )?;

  let query = MatchAllDocsQuery::new();

  let top_docs1 = searcher1.search(query.clone(), i32::MAX as usize)?;
  let top_docs2 = searcher2.search(query.clone(), i32::MAX as usize)?;

  CheckHits::check_equal(&query.into(), &top_docs1.score_docs, &top_docs2.score_docs)?;

  Ok(())
}
#[test]
fn test_multiple_slices_of_multiple_index_searchers() -> Result<()> {
  let mut random = random();
  let (reader1, reader2) = set_up_readers(&mut random)?;

  let searcher1 = index_searcher::from_reader_with_executor(
    reader1,
    new_search_executor(random.random_range(2..=5))?,
  )?;
  let searcher2 = index_searcher::from_reader_with_executor(
    reader2,
    new_search_executor(random.random_range(2..=5))?,
  )?;

  let query = MatchAllDocsQuery::new();

  let mut top_docs1 = searcher1.search(query.clone(), i32::MAX as usize)?;
  let mut top_docs2 = searcher2.search(query.clone(), i32::MAX as usize)?;

  assert_eq!(top_docs1.score_docs.len(), top_docs2.score_docs.len());

  for i in 0..top_docs1.score_docs.len() {
    top_docs1.score_docs[i].shard_index = 0;
    top_docs2.score_docs[i].shard_index = 1;
  }

  let shard_hits = vec![top_docs1, top_docs2];

  let merged_hits1 =
    top_docs::merge_top_docs_with_start(0, shard_hits[0].score_docs.len(), shard_hits.clone())?;
  let merged_hits2 =
    top_docs::merge_top_docs_with_start(0, shard_hits[0].score_docs.len(), shard_hits)?;

  CheckHits::check_equal(
    &query.into(),
    &merged_hits1.score_docs,
    &merged_hits2.score_docs,
  )?;

  Ok(())
}

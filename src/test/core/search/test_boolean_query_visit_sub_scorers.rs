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
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::search::query::Query;
use crate::core::search::similarities_impl::classic_similarity;
use crate::core::search::similarities_impl::raw_tf_similarity::RawTFSimilarity;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::scorer_index_searcher::ScorerIndexSearcher;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_log_merge_policy,
  new_searcher_with_wrap_assert,
};
use crate::test::core::util::{DefaultCRReaderShared, DefaultIndexSearchCRShared};
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
const F1: &str = "title";
const F2: &str = "body";
#[allow(dead_code)] // for quick search
pub struct TestBooleanQueryVisitSubScorers;

fn set_up<R: Rng + ?Sized>(
  random: &mut R,
) -> Result<(
  DefaultIndexSearchCRShared,
  ScorerIndexSearcher<DefaultCRReaderShared>,
)> {
  let analyzer = MockAnalyzer::new(random);
  let dir = new_directory_shared(random)?;

  let mut config = new_index_writer_config_with_analyzer(random, analyzer)?;
  config.set_merge_policy(new_log_merge_policy(random)?);

  let writer = RandomIndexWriter::with_config(random, dir, config);

  writer.add_document(
    random,
    doc("lucene", "lucene is a very popular search engine library")?,
  )?;

  writer.add_document(
    random,
    doc(
      "solr",
      "solr is a very popular search server and is using lucene",
    )?,
  )?;

  writer.add_document(
    random,
    doc(
      "nutch",
      "nutch is an internet search engine with web crawler and is using lucene and hadoop",
    )?,
  )?;

  let reader = Arc::new(writer.get_reader(random)?);
  writer.close(random)?;

  let mut searcher: DefaultIndexSearchCRShared =
    new_searcher_with_wrap_assert(random, reader.clone(), true, false)?;
  searcher.set_similarity(classic_similarity::new());

  let mut scorer_searcher: ScorerIndexSearcher<DefaultCRReaderShared> =
    ScorerIndexSearcher::new(reader);
  scorer_searcher.s.set_similarity(RawTFSimilarity::default());
  Ok((searcher, scorer_searcher))
}

fn get_doc_counts(
  _searcher: &ScorerIndexSearcher<DefaultCRReaderShared>,
  _query: Query,
) -> Result<HashMap<usize, usize>> {
  // TODO IMPORTANT getChildren未实现
  todo!()
}

fn doc(v1: &str, v2: &str) -> Result<Document> {
  let mut doc = Document::new();

  doc.add(TextField::from_string(F1, v1, Store::Yes)?);

  doc.add(TextField::from_string(F2, v2, Store::Yes)?);
  Ok(doc)
}

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
use crate::core::document::field_type::FieldType;

use crate::core::index::directory_reader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::search::total_hits::Relation;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config, new_index_writer_config_with_analyzer,
  new_log_merge_policy, new_searcher_with_reader, new_searcher_with_threads, new_text_field,
  random,
};
use rand_chacha::rand_core::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use std::vec;

#[allow(dead_code)] // for quick search
struct TestMatchAllDocsQuery;
#[test]
fn test_query() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_max_buffered_docs(2);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);

  let iw = IndexWriter::new(dir.clone(), iwc)?;
  let mut field_types = HashMap::new();

  add_doc(&mut random, "one", &iw, &mut field_types)?;
  add_doc(&mut random, "two", &iw, &mut field_types)?;
  add_doc(&mut random, "three four", &iw, &mut field_types)?;

  let ir = directory_reader::open_from_writer(&iw)?;
  let mut searcher = new_searcher_with_reader(ir)?;

  let mut hits = searcher.search(MatchAllDocsQuery::new(), 1000)?.score_docs;
  assert_eq!(3, hits.len());
  assert_eq!(
    "one",
    searcher
      .stored_fields()?
      .document(hits[0].doc)?
      .get("key")?
      .unwrap()
      .as_ref()
  );
  assert_eq!(
    "two",
    searcher
      .stored_fields()?
      .document(hits[1].doc)?
      .get("key")?
      .unwrap()
      .as_ref()
  );
  assert_eq!(
    "three four",
    searcher
      .stored_fields()?
      .document(hits[2].doc)?
      .get("key")?
      .unwrap()
      .as_ref()
  );

  // some artificial queries to trigger the use of skipTo():

  let mut bq = Builder::new();
  bq.add(MatchAllDocsQuery::new(), Occur::Must)?;
  bq.add(MatchAllDocsQuery::new(), Occur::Must)?;
  hits = searcher.search(bq.build(), 1000)?.score_docs;
  assert_eq!(3, hits.len());

  let mut bq = Builder::new();
  bq.add(MatchAllDocsQuery::new(), Occur::Must)?;
  bq.add(TermQuery::new(Term::from_text("key", "three")), Occur::Must)?;
  hits = searcher.search(bq.build(), 1000)?.score_docs;
  assert_eq!(1, hits.len());

  iw.delete_documents_with_terms(vec![Term::from_text("key", "one")])?;

  let reader = directory_reader::open_from_writer(&iw)?;
  searcher = new_searcher_with_reader(reader)?;

  hits = searcher.search(MatchAllDocsQuery::new(), 1000)?.score_docs;
  assert_eq!(2, hits.len());

  iw.close()?;
  Ok(())
}
#[test]
fn test_equals() -> Result<()> {
  let q1 = MatchAllDocsQuery::new();
  let q2 = MatchAllDocsQuery::new();
  assert_eq!(q1, q2);
  Ok(())
}
fn add_doc<D, R>(
  random: &mut R,
  text: &str,
  iw: &IndexWriter<D>,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory,
  R: Rng + ?Sized,
{
  let mut doc = Document::new();
  let field = new_text_field(random, "key", text, Store::Yes, field_to_type)?;
  doc.add(field);
  iw.add_document(doc)?;
  Ok(())
}
#[test]
fn test_early_termination() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config.set_max_buffered_docs(2);
  config.set_merge_policy(new_log_merge_policy(&mut random)?);
  let iw = IndexWriter::new(dir.clone(), config)?;
  let mut field_types = HashMap::new();
  let num_docs = 500;
  for i in 0..num_docs {
    let text = format!("doc{}", i);
    add_doc(&mut random, &text, &iw, &mut field_types)?;
  }

  let ir = directory_reader::open_from_writer(&iw)?;
  let ir_arc = Arc::new(ir);

  let single_threaded_searcher =
    new_searcher_with_threads(&mut random, ir_arc.clone(), true, true, false)?;

  let total_hits_threshold = 200;
  let collector_mgr = TopScoreDocCollectorManager::new(10, total_hits_threshold)?;

  let top_docs = single_threaded_searcher
    .search_with_collector_manager(MatchAllDocsQuery::new(), &collector_mgr)?;

  assert_eq!(top_docs.total_hits.value(), total_hits_threshold + 1);
  assert_eq!(
    top_docs.total_hits.relation(),
    Relation::GreaterThanOrEqualTo
  );

  let searcher = new_searcher_with_reader(ir_arc.clone())?;
  let collector_mgr = TopScoreDocCollectorManager::new(10, num_docs)?;

  let top_docs =
    searcher.search_with_collector_manager(MatchAllDocsQuery::new(), &collector_mgr)?;

  assert_eq!(top_docs.total_hits.value(), num_docs);
  assert_eq!(top_docs.total_hits.relation(), Relation::EqualTo);
  iw.close()?;
  Ok(())
}

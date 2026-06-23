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
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::term_query::TermQuery;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::search::query_utils::QueryUtils;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_log_merge_policy,
  new_searcher_with_reader, new_text_field, random,
};
use crate::test::core::util::{dummy_directory, dummy_index_searcher};
use rand::Rng;
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
struct TestMatchNoDocsQuery;
fn set_up<R>(random: &mut R) -> MockAnalyzer
where
  R: Rng + ?Sized,
{
  MockAnalyzer::new(random)
}

#[test]
fn test_simple() -> Result<()> {
  {
    let mut query = MatchNoDocsQuery::new();
    assert_eq!(query.to_string("")?, "MatchNoDocsQuery(\"\")");

    query = MatchNoDocsQuery::with_reason("field 'title' not found");
    assert_eq!(
      query.to_string("")?,
      "MatchNoDocsQuery(\"field 'title' not found\")"
    );
    let dummy_searcher = dummy_index_searcher(dummy_directory()?)?;
    let rewrite = query.rewrite(&dummy_searcher)?;
    assert!(matches!(rewrite, Query::MatchNoDocs(_)));
    assert_eq!(
      rewrite.to_string("")?,
      "MatchNoDocsQuery(\"field 'title' not found\")"
    );
  }

  Ok(())
}

#[test]
fn test_query() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);

  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_max_buffered_docs(2);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);

  let mut iw = IndexWriter::new(dir.clone(), iwc)?;
  let mut field_to_type = HashMap::new();
  add_doc("one", &mut iw, &mut random, &mut field_to_type)?;
  add_doc("two", &mut iw, &mut random, &mut field_to_type)?;
  add_doc("three", &mut iw, &mut random, &mut field_to_type)?;

  let reader = directory_reader::open_from_writer(&iw)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut query: Query = MatchNoDocsQuery::with_reason("field not found").into();
  assert_eq!(searcher.count(query.clone())?, 0);

  let hits = searcher.search(MatchNoDocsQuery::new(), 1000)?.score_docs;
  assert_eq!(hits.len(), 0);
  assert_eq!(
    query.to_string("")?,
    "MatchNoDocsQuery(\"field not found\")"
  );

  let mut bq = Builder::new();
  bq.add(
    TermQuery::new(Term::from_text("key", "five")),
    Occur::Should,
  )?;
  bq.add(
    MatchNoDocsQuery::with_reason("field not found"),
    Occur::Must,
  )?;
  query = bq.build().into();

  assert_eq!(searcher.count(query.clone())?, 0);

  let hits = searcher.search(MatchNoDocsQuery::new(), 1000)?.score_docs;
  assert_eq!(hits.len(), 0);
  assert_eq!(
    query.to_string("")?,
    "key:five +MatchNoDocsQuery(\"field not found\")"
  );

  let mut bq = Builder::new();
  bq.add(TermQuery::new(Term::from_text("key", "one")), Occur::Should)?;
  bq.add(
    MatchNoDocsQuery::with_reason("field not found"),
    Occur::Should,
  )?;
  query = bq.build().into();

  assert_eq!(
    query.to_string("")?,
    "key:one MatchNoDocsQuery(\"field not found\")"
  );
  assert_eq!(searcher.count(query.clone())?, 1);

  let hits = searcher.search(query.clone(), 1000)?.score_docs;
  let rewrite = searcher.rewrite(query.clone())?;

  assert_eq!(hits.len(), 1);
  assert_eq!(rewrite.to_string("")?, "key:one");

  iw.close()?;

  Ok(())
}

#[test]
fn test_equals() -> Result<()> {
  let q1: Query = MatchNoDocsQuery::new().into();
  let q2: Query = MatchNoDocsQuery::new().into();

  assert_eq!(q1, q2);
  QueryUtils::check_from_query(&q1);

  Ok(())
}

fn add_doc<R, D>(
  text: &str,
  iw: &mut IndexWriter<D>,
  random: &mut R,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  R: Rng + ?Sized,
  D: Directory + 'static,
{
  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "key",
    text,
    Store::Yes,
    field_to_type,
  )?);

  iw.add_document(doc)?;

  Ok(())
}

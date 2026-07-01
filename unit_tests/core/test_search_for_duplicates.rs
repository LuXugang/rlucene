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
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::stored_field::StoredField;
use crate::core::index::directory_reader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::QueryBase;
use crate::core::search::score_doc::ScoreDocLike;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::{SortField, SortFieldType};
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::support::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, new_searcher_with_reader,
  new_text_field, random,
};
use rand_chacha::rand_core::Rng;
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
pub struct TestSearchForDuplicates;

const PRIORITY_FIELD: &str = "priority";
const ID_FIELD: &str = "id";
const HIGH_PRIORITY: &str = "high";
const MED_PRIORITY: &str = "medium";

#[test]
fn test_run() -> Result<()> {
  let mut random = random();
  let max_docs = at_least(&mut random, 225) as usize;

  let multi_file_output = do_test(&mut random, false, max_docs)?;
  let single_file_output = do_test(&mut random, true, max_docs)?;

  assert_eq!(multi_file_output, single_file_output);
  Ok(())
}

fn do_test<R>(random: &mut R, use_compound_files: bool, max_docs: usize) -> Result<String>
where
  R: Rng + ?Sized,
{
  let directory = new_directory_shared(random)?;
  let analyzer = MockAnalyzer::new(random);
  let mut conf = new_index_writer_config_with_analyzer(random, analyzer)?;
  conf.set_use_compound_file(use_compound_files);
  let writer = IndexWriter::new(directory.clone(), conf)?;

  let mut field_to_type = HashMap::new();
  for j in 0..max_docs {
    let mut d = Document::new();
    d.add(new_text_field(
      random,
      PRIORITY_FIELD,
      HIGH_PRIORITY,
      Store::Yes,
      &mut field_to_type,
    )?);
    d.add(StoredField::from_i32(ID_FIELD, j as i32)?);
    d.add(NumericDocValuesField::new(ID_FIELD, j as i64));
    writer.add_document(d)?;
  }
  writer.close()?;

  let reader = directory_reader::open(directory.clone())?;
  let searcher = new_searcher_with_reader(reader)?;
  let sort = Sort::with_fields(vec![
    SortField::get_field_score()?,
    SortField::new(Some(ID_FIELD), SortFieldType::Int)?,
  ])?;

  let mut output = String::new();
  let query = TermQuery::new(Term::from_text(PRIORITY_FIELD, HIGH_PRIORITY));
  output.push_str(&format!("Query: {}\n", query.to_string(PRIORITY_FIELD)?));

  let hits = searcher.search_with_sort_score(query, max_docs, sort.clone(), true)?;
  output.push_str(&format!("{} total results\n\n", hits.score_docs().len()));
  {
    let mut stored_fields = searcher.stored_fields()?;
    for (i, hit) in hits.score_docs().iter().enumerate() {
      if i < 10 || (i > 94 && i < 105) {
        let d = stored_fields.document(hit.doc())?;
        output.push_str(&format!("{} {}\n", i, d.get(ID_FIELD)?.unwrap()));
      }
    }
  }
  check_hits(hits.score_docs(), max_docs, &searcher)?;

  let searcher = new_searcher_with_reader(directory_reader::open(directory.clone())?)?;
  let mut boolean_query = BooleanQueryBuilder::new();
  boolean_query.add(
    TermQuery::new(Term::from_text(PRIORITY_FIELD, HIGH_PRIORITY)),
    Occur::Should,
  )?;
  boolean_query.add(
    TermQuery::new(Term::from_text(PRIORITY_FIELD, MED_PRIORITY)),
    Occur::Should,
  )?;
  let boolean_query = boolean_query.build();
  output.push_str(&format!(
    "Query: {}\n",
    boolean_query.to_string(PRIORITY_FIELD)?
  ));

  let hits = searcher.search_with_sort_score(boolean_query, max_docs, sort, true)?;
  output.push_str(&format!("{} total results\n\n", hits.score_docs().len()));
  {
    let mut stored_fields = searcher.stored_fields()?;
    for (i, hit) in hits.score_docs().iter().enumerate() {
      if i < 10 || (i > 94 && i < 105) {
        let d = stored_fields.document(hit.doc())?;
        output.push_str(&format!("{} {}\n", i, d.get(ID_FIELD)?.unwrap()));
      }
    }
  }
  check_hits(hits.score_docs(), max_docs, &searcher)?;

  Ok(output)
}

fn check_hits<IRC, SD>(
  hits: &[SD],
  expected_count: usize,
  searcher: &IndexSearcher<IRC>,
) -> Result<()>
where
  IRC: crate::core::index::index_reader_context::IndexReaderContext,
  SD: ScoreDocLike,
{
  assert_eq!(expected_count, hits.len(), "total results");
  let mut stored_fields = searcher.stored_fields()?;
  for (i, hit) in hits.iter().enumerate() {
    if i < 10 || (i > 94 && i < 105) {
      let d = stored_fields.document(hit.doc())?;
      assert_eq!(
        i.to_string().as_str(),
        d.get(ID_FIELD)?.unwrap().as_ref(),
        "check {}",
        i
      );
    }
  }
  Ok(())
}

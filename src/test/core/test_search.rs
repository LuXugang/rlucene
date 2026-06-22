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
use crate::core::index::directory_reader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::score_doc::ScoreDocLike;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::{SortField, SortFieldType};
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_searcher_with_reader,
  new_text_field, random,
};
use std::collections::HashMap;
#[allow(dead_code)] // for quick search
pub struct TestSearch;

#[test]
fn test_search() -> Result<()> {
  let multi_file_output = do_test_search(false)?;
  let single_file_output = do_test_search(true)?;

  assert_eq!(multi_file_output, single_file_output);
  Ok(())
}

fn do_test_search(use_compound_file: bool) -> Result<String> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_use_compound_file(use_compound_file);
  let writer = IndexWriter::new(directory.clone(), conf)?;

  let docs = [
    "a b c d e",
    "a b c d e a b c d e",
    "a b c d e f g h i j",
    "a c e",
    "e c a",
    "a c e a c e",
    "a c e a b c",
  ];
  let mut field_to_type = HashMap::new();
  for (j, contents) in docs.iter().enumerate() {
    let mut d = Document::new();
    d.add(new_text_field(
      &mut random,
      "contents",
      *contents,
      Store::Yes,
      &mut field_to_type,
    )?);
    d.add(NumericDocValuesField::new("id", j as i64));
    writer.add_document(d)?;
  }
  writer.close()?;

  let reader = directory_reader::open(directory.clone())?;
  let searcher = new_searcher_with_reader(reader)?;
  let sort = Sort::with_fields(vec![
    SortField::get_field_score()?,
    SortField::new(Some("id"), SortFieldType::Int)?,
  ])?;

  let mut output = String::new();
  for query in build_queries()? {
    output.push_str(&format!("Query: {}\n", query.to_string("contents")?));

    let hits = searcher.search_with_sort_score(query, 1000, sort.clone(), true)?;
    output.push_str(&format!("{} total results\n", hits.total_hits().value()));
    let mut stored_fields = searcher.stored_fields()?;
    for (i, hit) in hits.score_docs().iter().take(10).enumerate() {
      let d = stored_fields.document(hit.doc())?;
      output.push_str(&format!(
        "{} {} {}\n",
        i,
        hit.score(),
        d.get("contents")?.unwrap()
      ));
    }
  }

  Ok(output)
}

fn build_queries() -> Result<Vec<Query>> {
  let mut queries = Vec::new();

  let mut boolean_ab = BooleanQueryBuilder::new();
  boolean_ab.add(
    TermQuery::new(Term::from_text("contents", "a")),
    Occur::Should,
  )?;
  boolean_ab.add(
    TermQuery::new(Term::from_text("contents", "b")),
    Occur::Should,
  )?;
  queries.push(boolean_ab.build().into());

  queries.push(PhraseQuery::from_terms_no_slop("contents", &["a", "b"])?.into());
  queries.push(PhraseQuery::from_terms_no_slop("contents", &["a", "b", "c"])?.into());

  let mut boolean_ac = BooleanQueryBuilder::new();
  boolean_ac.add(
    TermQuery::new(Term::from_text("contents", "a")),
    Occur::Should,
  )?;
  boolean_ac.add(
    TermQuery::new(Term::from_text("contents", "c")),
    Occur::Should,
  )?;
  queries.push(boolean_ac.build().into());

  queries.push(PhraseQuery::from_terms_no_slop("contents", &["a", "c"])?.into());
  queries.push(PhraseQuery::from_terms_no_slop("contents", &["a", "c", "e"])?.into());

  Ok(queries)
}

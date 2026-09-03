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
use crate::core::document::keyword_field::KeywordField;
use crate::core::document::string_field::StringField;
use crate::test_framework::core::util::lucene_test_case::{
  new_bytes_ref_from_string, new_directory_shared, new_searcher_with_reader, new_string_field,
  random,
};

use crate::core::index::index_reader::IndexReader;
use crate::core::index::multi_reader::MultiReader;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::MissingValueEnum::{StringFirst, StringLast};
use crate::core::search::sort_field::SortFiledBase;
use crate::core::search::sorted_set_selector::SortedSetSelectorType;
use crate::core::search::sorted_set_selector::SortedSetSelectorType::Max;
use crate::core::search::sorted_set_sort_field::SortedSetSortField;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::CoreHelper;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use std::collections::HashMap;
use std::sync::Arc;

/// Simple tests for SortedSetSortField, indexing the sortedset up front
#[allow(dead_code)]
struct TestSortedSetSortField;
#[test]
fn test_empty_index() -> Result<()> {
  let reader = MultiReader::empty()?;
  let empty = new_searcher_with_reader(reader)?;
  let query = TermQuery::new(Term::from_text("contents", "foo"));

  let sort = Sort::with_fields(vec![SortedSetSortField::new("sortedset", false)?])?;
  let td = empty.search_with_sort(query.clone(), 10, sort)?;
  assert_eq!(0, td.total_hits().value());

  // for an empty index, any selector should work
  for v in SortedSetSelectorType::values() {
    let sort = Sort::with_fields(vec![SortedSetSortField::with_selector(
      "sortedset",
      false,
      *v,
    )?])?;

    let td = empty.search_with_sort(query.clone(), 10, sort)?;
    assert_eq!(0, td.total_hits().value());
  }

  Ok(())
}

#[test]
fn test_equals() -> Result<()> {
  let sf = SortedSetSortField::new("a", false)?;
  assert!(sf == sf);
  let sf2 = SortedSetSortField::new("a", false)?;
  assert!(sf == sf2);
  assert_eq!(
    CoreHelper::calculate_hash(&sf),
    CoreHelper::calculate_hash(&sf2)
  );

  assert!(sf != SortedSetSortField::new("a", true)?);
  assert!(sf != SortedSetSortField::new("b", false)?);
  assert!(sf != SortedSetSortField::with_selector("a", false, Max)?);
  Ok(())
}

#[test]
fn test_forward() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  // doc1
  let mut doc = Document::new();
  doc.add(KeywordField::from_bytes_ref(
    "value",
    new_bytes_ref_from_string(&mut random, "baz")?,
    Store::No,
  )?);
  doc.add(StringField::from_string("id", "2", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  // doc2
  let mut doc = Document::new();
  doc.add(KeywordField::from_bytes_ref(
    "value",
    new_bytes_ref_from_string(&mut random, "foo")?,
    Store::No,
  )?);
  doc.add(KeywordField::from_bytes_ref(
    "value",
    new_bytes_ref_from_string(&mut random, "bar")?,
    Store::No,
  )?);
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let reader = Arc::new(writer.get_reader(&mut random)?);
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader.clone())?;
  let sort = Sort::with_fields(vec![SortedSetSortField::new("value", false)?])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(2, td.total_hits().value());

  let doc0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!("1", doc0.get("id")?.unwrap().as_ref());

  let doc1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!("2", doc1.get("id")?.unwrap().as_ref());

  reader.close()?;
  dir.close()?;
  Ok(())
}
#[test]
fn test_reverse() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  let mut doc = Document::new();
  doc.add(KeywordField::from_bytes_ref(
    "value",
    new_bytes_ref_from_string(&mut random, "foo")?,
    Store::No,
  )?);
  doc.add(KeywordField::from_bytes_ref(
    "value",
    new_bytes_ref_from_string(&mut random, "bar")?,
    Store::No,
  )?);
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  doc.add(KeywordField::from_bytes_ref(
    "value",
    new_bytes_ref_from_string(&mut random, "baz")?,
    Store::No,
  )?);
  doc.add(StringField::from_string("id", "2", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let reader = Arc::new(writer.get_reader(&mut random)?);
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader.clone())?;
  let sort = Sort::with_fields(vec![SortedSetSortField::new("value", true)?])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(2, td.total_hits().value());

  let doc0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!("2", doc0.get("id")?.unwrap().as_ref());

  let doc1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!("1", doc1.get("id")?.unwrap().as_ref());
  reader.close()?;
  dir.close()?;
  Ok(())
}
#[test]
fn test_missing_first() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  let mut field_types: HashMap<String, FieldType> = HashMap::new();

  let mut doc = Document::new();
  doc.add(KeywordField::from_bytes_ref(
    "value",
    new_bytes_ref_from_string(&mut random, "baz")?,
    Store::No,
  )?);
  doc.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_types,
  )?);
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  doc.add(KeywordField::from_bytes_ref(
    "value",
    new_bytes_ref_from_string(&mut random, "foo")?,
    Store::No,
  )?);
  doc.add(KeywordField::from_bytes_ref(
    "value",
    new_bytes_ref_from_string(&mut random, "bar")?,
    Store::No,
  )?);
  doc.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_types,
  )?);
  writer.add_document(&mut random, doc)?;

  // doc3: missing 'value'
  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "id",
    "3",
    Store::Yes,
    &mut field_types,
  )?);
  writer.add_document(&mut random, doc)?;

  let reader = Arc::new(writer.get_reader(&mut random)?);
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader.clone())?;

  let mut sort_field = SortedSetSortField::new("value", false)?;
  sort_field.set_missing_value(StringFirst)?;
  let sort = Sort::with_fields(vec![sort_field])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(3, td.total_hits().value());

  let doc0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!("3", doc0.get("id")?.unwrap().as_ref());

  let doc1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!("1", doc1.get("id")?.unwrap().as_ref());

  let doc2 = searcher
    .stored_fields()?
    .document(td.score_docs()[2].doc())?;
  assert_eq!("2", doc2.get("id")?.unwrap().as_ref());

  reader.close()?;
  dir.close()?;
  Ok(())
}

#[test]
fn test_missing_last() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  let mut field_types: HashMap<String, FieldType> = HashMap::new();

  let mut doc = Document::new();
  doc.add(KeywordField::from_bytes_ref(
    "value",
    new_bytes_ref_from_string(&mut random, "baz")?,
    Store::No,
  )?);
  doc.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_types,
  )?);
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  doc.add(KeywordField::from_bytes_ref(
    "value",
    new_bytes_ref_from_string(&mut random, "foo")?,
    Store::No,
  )?);
  doc.add(KeywordField::from_bytes_ref(
    "value",
    new_bytes_ref_from_string(&mut random, "bar")?,
    Store::No,
  )?);
  doc.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_types,
  )?);
  writer.add_document(&mut random, doc)?;

  // doc3: missing 'value'
  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "id",
    "3",
    Store::Yes,
    &mut field_types,
  )?);
  writer.add_document(&mut random, doc)?;

  let reader = Arc::new(writer.get_reader(&mut random)?);
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader.clone())?;

  let mut sort_field = SortedSetSortField::new("value", false)?;
  sort_field.set_missing_value(StringLast)?;
  let sort = Sort::with_fields(vec![sort_field])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(3, td.total_hits().value());

  // 'bar' comes before 'baz'
  let doc0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!("1", doc0.get("id")?.unwrap().as_ref());

  let doc1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!("2", doc1.get("id")?.unwrap().as_ref());

  // `None` comes last.
  let doc2 = searcher
    .stored_fields()?
    .document(td.score_docs()[2].doc())?;
  assert_eq!("3", doc2.get("id")?.unwrap().as_ref());

  reader.close()?;
  dir.close()?;
  Ok(())
}

#[test]
fn test_singleton() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  let mut doc = Document::new();
  doc.add(KeywordField::from_bytes_ref(
    "value",
    new_bytes_ref_from_string(&mut random, "baz")?,
    Store::No,
  )?);
  doc.add(StringField::from_string("id", "2", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  doc.add(KeywordField::from_bytes_ref(
    "value",
    new_bytes_ref_from_string(&mut random, "bar")?,
    Store::No,
  )?);
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let reader = Arc::new(writer.get_reader(&mut random)?);
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader.clone())?;
  let sort = Sort::with_fields(vec![SortedSetSortField::new("value", false)?])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(2, td.total_hits().value());

  let doc0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!("1", doc0.get("id")?.unwrap().as_ref());

  let doc1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!("2", doc1.get("id")?.unwrap().as_ref());

  reader.close()?;
  dir.close()?;
  Ok(())
}

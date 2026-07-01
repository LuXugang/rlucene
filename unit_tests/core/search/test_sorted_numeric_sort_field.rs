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
use crate::core::document::double_field::DoubleField;
use crate::core::document::field::Store;
use crate::core::document::float_field::FloatField;
use crate::core::document::int_field::IntField;
use crate::core::document::string_field::StringField;
use crate::test::support::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_reader, random,
};

use crate::core::index::multi_reader::MultiReader;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::{SortFieldType, SortFiledBase};
use crate::core::search::sorted_numeric_selector::SortedNumericSelectorType;
use crate::core::search::sorted_numeric_sort_field::SortedNumericSortField;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::CoreHelper;
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;

/// Simple tests for SortedNumericSortField
#[allow(dead_code)] // for quick search
struct TestSortedNumericSortField;
#[test]
fn test_empty_index() -> Result<()> {
  let reader = MultiReader::empty()?;
  let empty = new_searcher_with_reader(reader)?;
  let query = TermQuery::new(Term::from_text("contents", "foo"));

  let sort = Sort::with_fields(vec![SortedNumericSortField::new(
    "sortednumeric",
    SortFieldType::Long,
  )?])?;
  let td = empty.search_with_sort_score(query.clone(), 10, sort, true)?;
  assert_eq!(0, td.total_hits().value());

  // for an empty index, any selector should work
  for v in SortedNumericSelectorType::values() {
    let sort = Sort::with_fields(vec![SortedNumericSortField::with_selector(
      "sortednumeric",
      SortFieldType::Long,
      false,
      *v,
    )?])?;
    let td = empty.search_with_sort_score(query.clone(), 10, sort, true)?;
    assert_eq!(0, td.total_hits().value());
  }

  Ok(())
}

#[test]
fn test_equals() -> Result<()> {
  let sf = SortedNumericSortField::new("a", SortFieldType::Long)?;
  assert!(sf == sf);

  let sf2 = SortedNumericSortField::new("a", SortFieldType::Long)?;
  assert!(sf == sf2);
  assert_eq!(
    CoreHelper::calculate_hash(&sf),
    CoreHelper::calculate_hash(&sf2)
  );

  assert!(sf != SortedNumericSortField::with_reverse("a", SortFieldType::Long, true)?);
  assert!(sf != SortedNumericSortField::new("a", SortFieldType::Float)?);
  assert!(sf != SortedNumericSortField::new("b", SortFieldType::Long)?);

  assert!(
    sf != SortedNumericSortField::with_selector(
      "a",
      SortFieldType::Long,
      false,
      SortedNumericSelectorType::Max,
    )?
  );

  Ok(())
}
#[test]
fn test_forward() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  let mut doc = Document::new();
  doc.add(IntField::new("value", 5, Store::No)?);
  doc.add(StringField::from_string("id", "2", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  // doc2
  let mut doc = Document::new();
  doc.add(IntField::new("value", 3, Store::No)?);
  doc.add(IntField::new("value", 7, Store::No)?);
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader)?;
  let sort = Sort::with_fields(vec![SortedNumericSortField::new(
    "value",
    SortFieldType::Int,
  )?])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(2, td.total_hits().value());

  // 3 comes before 5
  let doc0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!("1", doc0.get("id")?.unwrap().as_ref());

  let doc1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!("2", doc1.get("id")?.unwrap().as_ref());

  Ok(())
}
#[test]
fn test_reverse() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  let mut doc = Document::new();
  doc.add(IntField::new("value", 3, Store::No)?);
  doc.add(IntField::new("value", 7, Store::No)?);
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  doc.add(IntField::new("value", 5, Store::No)?);
  doc.add(StringField::from_string("id", "2", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader)?;
  let sort = Sort::with_fields(vec![SortedNumericSortField::with_reverse(
    "value",
    SortFieldType::Int,
    true,
  )?])?;

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

  Ok(())
}

#[test]
fn test_missing_first() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  let mut doc = Document::new();
  doc.add(IntField::new("value", 5, Store::No)?);
  doc.add(StringField::from_string("id", "2", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  doc.add(IntField::new("value", 3, Store::No)?);
  doc.add(IntField::new("value", 7, Store::No)?);
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("id", "3", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader)?;

  let mut sort_field = SortedNumericSortField::new("value", SortFieldType::Int)?;
  sort_field.set_missing_value(i32::MIN)?;
  let sort = Sort::with_fields(vec![sort_field])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(3, td.total_hits().value());

  let d0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!("3", d0.get("id")?.unwrap().as_ref());

  let d1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!("1", d1.get("id")?.unwrap().as_ref());

  let d2 = searcher
    .stored_fields()?
    .document(td.score_docs()[2].doc())?;
  assert_eq!("2", d2.get("id")?.unwrap().as_ref());

  Ok(())
}

#[test]
fn test_missing_last() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  let mut doc = Document::new();
  doc.add(IntField::new("value", 5, Store::No)?);
  doc.add(StringField::from_string("id", "2", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  doc.add(IntField::new("value", 3, Store::No)?);
  doc.add(IntField::new("value", 7, Store::No)?);
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("id", "3", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader)?;

  let mut sort_field = SortedNumericSortField::new("value", SortFieldType::Int)?;
  sort_field.set_missing_value(i32::MAX)?;
  let sort = Sort::with_fields(vec![sort_field])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(3, td.total_hits().value());

  let d0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!("1", d0.get("id")?.unwrap().as_ref());

  let d1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!("2", d1.get("id")?.unwrap().as_ref());

  let d2 = searcher
    .stored_fields()?
    .document(td.score_docs()[2].doc())?;
  assert_eq!("3", d2.get("id")?.unwrap().as_ref());

  Ok(())
}

#[test]
fn test_singleton() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  // doc1
  let mut doc = Document::new();
  doc.add(IntField::new("value", 5, Store::No)?);
  doc.add(StringField::from_string("id", "2", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  // doc2
  let mut doc = Document::new();
  doc.add(IntField::new("value", 3, Store::No)?);
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader)?;
  let sort = Sort::with_fields(vec![SortedNumericSortField::new(
    "value",
    SortFieldType::Int,
  )?])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(2, td.total_hits().value());

  let d0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!("1", d0.get("id")?.unwrap().as_ref());

  let d1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!("2", d1.get("id")?.unwrap().as_ref());

  Ok(())
}
#[test]
fn test_float() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  let mut doc = Document::new();
  doc.add(FloatField::new("value", -3f32, Store::No)?);
  doc.add(StringField::from_string("id", "2", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  doc.add(FloatField::new("value", -5f32, Store::No)?);
  doc.add(FloatField::new("value", 7f32, Store::No)?);
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader)?;
  let sort = Sort::with_fields(vec![SortedNumericSortField::new(
    "value",
    SortFieldType::Float,
  )?])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(2, td.total_hits().value());

  let d0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!("1", d0.get("id")?.unwrap().as_ref());

  let d1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!("2", d1.get("id")?.unwrap().as_ref());

  Ok(())
}

#[test]
fn test_double() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  let mut doc = Document::new();
  doc.add(DoubleField::new("value", -3f64, Store::Yes)?);
  doc.add(StringField::from_string("id", "2", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  doc.add(DoubleField::new("value", -5f64, Store::Yes)?);
  doc.add(DoubleField::new("value", 7f64, Store::Yes)?);
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  writer.add_document(&mut random, doc)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader)?;
  let sort = Sort::with_fields(vec![SortedNumericSortField::new(
    "value",
    SortFieldType::Double,
  )?])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(2, td.total_hits().value());

  let d0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!("1", d0.get("id")?.unwrap().as_ref());

  let d1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!("2", d1.get("id")?.unwrap().as_ref());

  Ok(())
}

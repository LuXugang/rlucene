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
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::index::stored_fields::StoredFields;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::MissingValueEnum::{StringFirst, StringLast};
use crate::core::search::sort_field::SortFiledBase;
use crate::core::search::sorted_set_selector::SortedSetSelectorType::{Max, MiddleMax, MiddleMin};
use crate::core::search::sorted_set_sort_field::SortedSetSortField;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  new_bytes_ref_from_string, new_directory_shared, new_searcher_with_wrap, new_string_field, random,
};
use std::collections::HashMap;
/// Tests for SortedSetSortField selectors other than MIN, these require optional codec support (random access to ordinals)
#[allow(dead_code)] // for quick search
struct TestSortedSetSelector;
#[test]
fn test_max() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
  let mut doc1 = Document::new();
  doc1.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "foo")?,
  ));
  doc1.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "bar")?,
  ));
  doc1.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc1)?;

  let mut doc2 = Document::new();
  doc2.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "baz")?,
  ));
  doc2.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc2)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;
  // slow wrapper does not support random access ordinals (there is no need for that!)
  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let sort = Sort::with_fields(vec![SortedSetSortField::with_selector(
    "value", false, Max,
  )?])?;

  let top_docs = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;

  assert_eq!(top_docs.total_hits().value(), 2);
  let doc0 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[0].doc())?;
  assert_eq!(doc0.get("id")?.unwrap().as_ref(), "2");
  let doc1 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[1].doc())?;
  assert_eq!(doc1.get("id")?.unwrap().as_ref(), "1");

  Ok(())
}
#[test]
fn test_max_reverse() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

  let mut doc1 = Document::new();
  doc1.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "foo")?,
  ));
  doc1.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "bar")?,
  ));
  doc1.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc1)?;

  let mut doc2 = Document::new();
  doc2.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "baz")?,
  ));
  doc2.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc2)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;
  // slow wrapper does not support random access ordinals (there is no need for that!)
  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let sort = Sort::with_fields(vec![SortedSetSortField::with_selector("value", true, Max)?])?;

  let top_docs = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;

  assert_eq!(top_docs.total_hits().value(), 2);

  let doc0 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[0].doc())?;
  assert_eq!(doc0.get("id")?.unwrap().as_ref(), "1");

  let doc1 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[1].doc())?;
  assert_eq!(doc1.get("id")?.unwrap().as_ref(), "2");

  Ok(())
}
#[test]
fn test_max_missing_first() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

  let mut doc1 = Document::new();
  doc1.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc1)?;

  let mut doc2 = Document::new();
  doc2.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "foo")?,
  ));
  doc2.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "bar")?,
  ));
  doc2.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc2)?;

  let mut doc3 = Document::new();
  doc3.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "baz")?,
  ));
  doc3.add(new_string_field(
    &mut random,
    "id",
    "3",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc3)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let mut sort_field = SortedSetSortField::with_selector("value", false, Max)?;
  sort_field.set_missing_value(StringFirst)?;
  let sort = Sort::with_fields(vec![sort_field])?;

  let top_docs = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;

  assert_eq!(top_docs.total_hits().value(), 3);

  let doc0 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[0].doc())?;
  assert_eq!(doc0.get("id")?.unwrap().as_ref(), "1");

  let doc1 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[1].doc())?;
  assert_eq!(doc1.get("id")?.unwrap().as_ref(), "3");

  let doc2 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[2].doc())?;
  assert_eq!(doc2.get("id")?.unwrap().as_ref(), "2");

  Ok(())
}
#[test]
fn test_max_missing_last() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

  let mut doc1 = Document::new();
  doc1.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc1)?;

  let mut doc2 = Document::new();
  doc2.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "foo")?,
  ));
  doc2.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "bar")?,
  ));
  doc2.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc2)?;

  let mut doc3 = Document::new();
  doc3.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "baz")?,
  ));
  doc3.add(new_string_field(
    &mut random,
    "id",
    "3",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc3)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let mut sort_field = SortedSetSortField::with_selector("value", false, Max)?;
  sort_field.set_missing_value(StringLast)?;
  let sort = Sort::with_fields(vec![sort_field])?;

  let top_docs = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(top_docs.total_hits().value(), 3);

  let d0 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[0].doc())?;
  assert_eq!(d0.get("id")?.unwrap().as_ref(), "3");

  let d1 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[1].doc())?;
  assert_eq!(d1.get("id")?.unwrap().as_ref(), "2");

  let d2 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[2].doc())?;
  assert_eq!(d2.get("id")?.unwrap().as_ref(), "1");

  Ok(())
}
#[test]
fn test_max_singleton() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

  let mut doc1 = Document::new();
  doc1.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "baz")?,
  ));
  doc1.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc1)?;

  let mut doc2 = Document::new();
  doc2.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "bar")?,
  ));
  doc2.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc2)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let sort = Sort::with_fields(vec![SortedSetSortField::with_selector(
    "value", false, Max,
  )?])?;

  let top_docs = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(top_docs.total_hits().value(), 2);

  let d0 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[0].doc())?;
  assert_eq!(d0.get("id")?.unwrap().as_ref(), "1");

  let d1 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[1].doc())?;
  assert_eq!(d1.get("id")?.unwrap().as_ref(), "2");

  Ok(())
}
#[test]
fn test_middle_min() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

  let mut doc1 = Document::new();
  doc1.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "c")?,
  ));
  doc1.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc1)?;

  let mut doc2 = Document::new();
  for v in ["a", "b", "c", "d"] {
    doc2.add(SortedSetDocValuesField::new(
      "value",
      new_bytes_ref_from_string(&mut random, v)?,
    ));
  }
  doc2.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc2)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let sort = Sort::with_fields(vec![SortedSetSortField::with_selector(
    "value", false, MiddleMin,
  )?])?;

  let top_docs = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(top_docs.total_hits().value(), 2);

  let d0 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[0].doc())?;
  assert_eq!(d0.get("id")?.unwrap().as_ref(), "1");

  let d1 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[1].doc())?;
  assert_eq!(d1.get("id")?.unwrap().as_ref(), "2");

  Ok(())
}
#[test]
fn test_middle_min_reverse() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

  let mut doc1 = Document::new();
  for v in ["a", "b", "c", "d"] {
    doc1.add(SortedSetDocValuesField::new(
      "value",
      new_bytes_ref_from_string(&mut random, v)?,
    ));
  }
  doc1.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc1)?;

  let mut doc2 = Document::new();
  doc2.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "c")?,
  ));
  doc2.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc2)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let sort = Sort::with_fields(vec![SortedSetSortField::with_selector(
    "value", true, MiddleMin,
  )?])?;

  let top_docs = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(top_docs.total_hits().value(), 2);

  let d0 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[0].doc())?;
  assert_eq!(d0.get("id")?.unwrap().as_ref(), "2");

  let d1 = searcher
    .stored_fields()?
    .document(top_docs.score_docs()[1].doc())?;
  assert_eq!(d1.get("id")?.unwrap().as_ref(), "1");

  Ok(())
}
#[test]
fn test_middle_min_missing_first() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

  let mut doc1 = Document::new();
  doc1.add(new_string_field(
    &mut random,
    "id",
    "3",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc1)?;

  let mut doc2 = Document::new();
  doc2.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "c")?,
  ));
  doc2.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc2)?;

  let mut doc3 = Document::new();
  for v in ["a", "b", "c", "d"] {
    doc3.add(SortedSetDocValuesField::new(
      "value",
      new_bytes_ref_from_string(&mut random, v)?,
    ));
  }
  doc3.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc3)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let mut sort_field = SortedSetSortField::with_selector("value", false, MiddleMin)?;
  sort_field.set_missing_value(StringFirst)?;
  let sort = Sort::with_fields(vec![sort_field])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(td.total_hits().value(), 3);

  let d0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!(d0.get("id")?.unwrap().as_ref(), "3");

  let d1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!(d1.get("id")?.unwrap().as_ref(), "1");

  let d2 = searcher
    .stored_fields()?
    .document(td.score_docs()[2].doc())?;
  assert_eq!(d2.get("id")?.unwrap().as_ref(), "2");

  Ok(())
}
#[test]
fn test_middle_min_missing_last() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

  let mut doc1 = Document::new();
  doc1.add(new_string_field(
    &mut random,
    "id",
    "3",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc1)?;

  let mut doc2 = Document::new();
  doc2.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "c")?,
  ));
  doc2.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc2)?;

  let mut doc3 = Document::new();
  for v in ["a", "b", "c", "d"] {
    doc3.add(SortedSetDocValuesField::new(
      "value",
      new_bytes_ref_from_string(&mut random, v)?,
    ));
  }
  doc3.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc3)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  // MIDDLE_MIN with missing last
  let mut sort_field = SortedSetSortField::with_selector("value", false, MiddleMin)?;
  sort_field.set_missing_value(StringLast)?;
  let sort = Sort::with_fields(vec![sort_field])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(td.total_hits().value(), 3);

  // MiddleMin(["a","b","c","d"]) = "b" → first
  let d0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!(d0.get("id")?.unwrap().as_ref(), "1");

  // MiddleMin(["c"]) = "c" → second
  let d1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!(d1.get("id")?.unwrap().as_ref(), "2");

  // missing → last
  let d2 = searcher
    .stored_fields()?
    .document(td.score_docs()[2].doc())?;
  assert_eq!(d2.get("id")?.unwrap().as_ref(), "3");

  Ok(())
}
#[test]
fn test_middle_min_singleton() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

  let mut doc1 = Document::new();
  doc1.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "baz")?,
  ));
  doc1.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc1)?;

  let mut doc2 = Document::new();
  doc2.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "bar")?,
  ));
  doc2.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc2)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let sort = Sort::with_fields(vec![SortedSetSortField::with_selector(
    "value", false, MiddleMin,
  )?])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(td.total_hits().value(), 2);

  let d0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!(d0.get("id")?.unwrap().as_ref(), "1");

  let d1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!(d1.get("id")?.unwrap().as_ref(), "2");

  Ok(())
}
#[test]
fn test_middle_max() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

  let mut doc1 = Document::new();
  for v in ["a", "b", "c", "d"] {
    doc1.add(SortedSetDocValuesField::new(
      "value",
      new_bytes_ref_from_string(&mut random, v)?,
    ));
  }
  doc1.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc1)?;

  let mut doc2 = Document::new();
  doc2.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "b")?,
  ));
  doc2.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc2)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let sort = Sort::with_fields(vec![SortedSetSortField::with_selector(
    "value", false, MiddleMax,
  )?])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(td.total_hits().value(), 2);

  let d0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!(d0.get("id")?.unwrap().as_ref(), "2");

  let d1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!(d1.get("id")?.unwrap().as_ref(), "1");

  Ok(())
}
#[test]
fn test_middle_max_reverse() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();

  let mut d1 = Document::new();
  d1.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "b")?,
  ));
  d1.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, d1)?;

  let mut d2 = Document::new();
  for v in ["a", "b", "c", "d"] {
    d2.add(SortedSetDocValuesField::new(
      "value",
      new_bytes_ref_from_string(&mut random, v)?,
    ));
  }
  d2.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, d2)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;
  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let sort = Sort::with_fields(vec![SortedSetSortField::with_selector(
    "value", true, MiddleMax,
  )?])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(td.total_hits().value(), 2);

  assert_eq!(
    searcher
      .stored_fields()?
      .document(td.score_docs()[0].doc())?
      .get("id")?
      .unwrap()
      .as_ref(),
    "1"
  );
  assert_eq!(
    searcher
      .stored_fields()?
      .document(td.score_docs()[1].doc())?
      .get("id")?
      .unwrap()
      .as_ref(),
    "2"
  );
  Ok(())
}
#[test]
fn test_middle_max_missing_first() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

  let mut doc1 = Document::new();
  doc1.add(new_string_field(
    &mut random,
    "id",
    "3",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc1)?;

  let mut doc2 = Document::new();
  for v in ["a", "b", "c", "d"] {
    doc2.add(SortedSetDocValuesField::new(
      "value",
      new_bytes_ref_from_string(&mut random, v)?,
    ));
  }
  doc2.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc2)?;

  let mut doc3 = Document::new();
  doc3.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "b")?,
  ));
  doc3.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc3)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let mut sort_field = SortedSetSortField::with_selector("value", false, MiddleMax)?;
  sort_field.set_missing_value(StringFirst)?;
  let sort = Sort::with_fields(vec![sort_field])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;

  assert_eq!(td.total_hits().value(), 3);

  let d0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!(d0.get("id")?.unwrap().as_ref(), "3");

  let d1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!(d1.get("id")?.unwrap().as_ref(), "2");

  let d2 = searcher
    .stored_fields()?
    .document(td.score_docs()[2].doc())?;
  assert_eq!(d2.get("id")?.unwrap().as_ref(), "1");

  Ok(())
}
#[test]
fn test_middle_max_missing_last() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

  let mut doc1 = Document::new();
  doc1.add(new_string_field(
    &mut random,
    "id",
    "3",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc1)?;

  let mut doc2 = Document::new();
  for v in ["a", "b", "c", "d"] {
    doc2.add(SortedSetDocValuesField::new(
      "value",
      new_bytes_ref_from_string(&mut random, v)?,
    ));
  }
  doc2.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc2)?;

  let mut doc3 = Document::new();
  doc3.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "b")?,
  ));
  doc3.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc3)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let mut sf = SortedSetSortField::with_selector("value", false, MiddleMax)?;
  sf.set_missing_value(StringLast)?;
  let sort = Sort::with_fields(vec![sf])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(td.total_hits().value(), 3);

  let d0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!(d0.get("id")?.unwrap().as_ref(), "2");

  let d1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!(d1.get("id")?.unwrap().as_ref(), "1");

  let d2 = searcher
    .stored_fields()?
    .document(td.score_docs()[2].doc())?;
  assert_eq!(d2.get("id")?.unwrap().as_ref(), "3");

  Ok(())
}
#[test]
fn test_middle_max_singleton() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

  let mut doc2 = Document::new();
  doc2.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "baz")?,
  ));
  doc2.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc2)?;

  let mut doc1 = Document::new();
  doc1.add(SortedSetDocValuesField::new(
    "value",
    new_bytes_ref_from_string(&mut random, "bar")?,
  ));
  doc1.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc1)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let sort = Sort::with_fields(vec![SortedSetSortField::with_selector(
    "value", false, MiddleMax,
  )?])?;

  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, sort)?;
  assert_eq!(td.total_hits().value(), 2);

  let d0 = searcher
    .stored_fields()?
    .document(td.score_docs()[0].doc())?;
  assert_eq!(d0.get("id")?.unwrap().as_ref(), "1");

  let d1 = searcher
    .stored_fields()?
    .document(td.score_docs()[1].doc())?;
  assert_eq!(d1.get("id")?.unwrap().as_ref(), "2");

  Ok(())
}

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
use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::doc_values::DocValues;
use crate::core::index::index_writer::IndexWriter;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::util::lucene_test_case::{
  get_only_leaf_reader, new_directory_shared, new_index_writer_config, random,
};

use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;

#[allow(dead_code)] // for quick search
struct TestDocValues;

///If the field doesn't exist, we return empty instances:
/// It can easily happen that a segment just doesn't have any docs with the field.
#[test]
fn test_empty_index() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;
  let doc = Document::new();
  writer.add_document(doc)?;

  let dr = directory_reader::open_from_writer(&writer)?;
  let r = get_only_leaf_reader(dr)?;

  let mut v = DocValues::get_binary(r.as_ref(), "bogus")?;
  assert_eq!(v.next_doc()?, NO_MORE_DOCS);
  let mut v = DocValues::get_numeric(r.as_ref(), "bogus")?;
  assert_eq!(v.next_doc()?, NO_MORE_DOCS);
  let mut v = DocValues::get_sorted(r.as_ref(), "bogus")?;
  assert_eq!(v.next_doc()?, NO_MORE_DOCS);
  let mut v = DocValues::get_sorted_set(r.as_ref(), "bogus")?;
  assert_eq!(v.next_doc()?, NO_MORE_DOCS);
  let mut v = DocValues::get_sorted_numeric(r.as_ref(), "bogus")?;
  assert_eq!(v.next_doc()?, NO_MORE_DOCS);

  writer.close()?;
  Ok(())
}
/// field just doesnt have any docvalues at all:error
#[test]
fn test_misconfigured_field() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("foo", "bar", Store::No)?);
  writer.add_document(doc)?;

  let dr = directory_reader::open_from_writer(&writer)?;
  let r = get_only_leaf_reader(dr)?;

  // errors
  assert!(matches!(
    DocValues::get_binary(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_numeric(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_sorted(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_sorted_set(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_sorted_numeric(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));

  writer.close()?;
  Ok(())
}
/// field with numeric docvalues
#[test]
fn test_numeric_field() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;

  let mut doc = Document::new();
  doc.add(NumericDocValuesField::new("foo", 3));
  writer.add_document(doc)?;

  let dr = directory_reader::open_from_writer(&writer)?;
  let r = get_only_leaf_reader(dr)?;

  // ok
  let mut v = DocValues::get_numeric(r.as_ref(), "foo")?;
  assert_eq!(v.next_doc()?, 0);

  let mut v = DocValues::get_sorted_numeric(r.as_ref(), "foo")?;
  assert_eq!(v.next_doc()?, 0);

  // errors
  assert!(matches!(
    DocValues::get_binary(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_sorted(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_sorted_set(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));

  writer.close()?;
  Ok(())
}
/// field with binary docvalues
#[test]
fn test_binary_field() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;

  let mut doc = Document::new();
  doc.add(BinaryDocValuesField::new(
    "foo",
    BytesRef::from_string("bar"),
  ));
  writer.add_document(doc)?;

  let dr = directory_reader::open_from_writer(&writer)?;
  let r = get_only_leaf_reader(dr)?;

  // ok
  let mut v = DocValues::get_binary(r.as_ref(), "foo")?;
  assert_eq!(v.next_doc()?, 0);

  // errors
  assert!(matches!(
    DocValues::get_numeric(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_sorted(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_sorted_set(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_sorted_numeric(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));

  writer.close()?;
  Ok(())
}
/// field with sorted docvalues
#[test]
fn test_sorted_field() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;

  let mut doc = Document::new();
  doc.add(SortedDocValuesField::new(
    "foo",
    BytesRef::from_string("bar"),
  ));
  writer.add_document(doc)?;

  let dr = directory_reader::open_from_writer(&writer)?;
  let r = get_only_leaf_reader(dr)?;

  // ok
  let mut v = DocValues::get_sorted(r.as_ref(), "foo")?;
  assert_eq!(v.next_doc()?, 0);

  let mut v = DocValues::get_sorted_set(r.as_ref(), "foo")?;
  assert_eq!(v.next_doc()?, 0);

  // errors
  assert!(matches!(
    DocValues::get_binary(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_numeric(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_sorted_numeric(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));

  writer.close()?;
  Ok(())
}
/// field with sortedset docvalues
#[test]
fn test_sorted_set_field() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;

  let mut doc = Document::new();
  doc.add(SortedSetDocValuesField::new(
    "foo",
    BytesRef::from_string("bar"),
  ));
  writer.add_document(doc)?;

  let dr = directory_reader::open_from_writer(&writer)?;
  let r = get_only_leaf_reader(&dr)?;

  // ok
  let mut v = DocValues::get_sorted_set(r.as_ref(), "foo")?;
  assert_eq!(v.next_doc()?, 0);

  // errors
  assert!(matches!(
    DocValues::get_binary(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_numeric(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_sorted(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_sorted_numeric(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));

  writer.close()?;
  Ok(())
}
/// field with sortednumeric docvalues
#[test]
fn test_sorted_numeric_field() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;

  let mut doc = Document::new();
  doc.add(SortedNumericDocValuesField::new("foo", 3));
  writer.add_document(doc)?;

  let dr = directory_reader::open_from_writer(&writer)?;
  let r = get_only_leaf_reader(dr)?;

  // ok
  let mut v = DocValues::get_sorted_numeric(r.as_ref(), "foo")?;
  assert_eq!(v.next_doc()?, 0);

  // errors
  assert!(matches!(
    DocValues::get_binary(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_numeric(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_sorted(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));
  assert!(matches!(
    DocValues::get_sorted_set(r.as_ref(), "foo"),
    Err(LuceneError::IllegalState(_))
  ));

  writer.close()?;
  Ok(())
}
#[test]
fn test_add_null_numeric_doc_values() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

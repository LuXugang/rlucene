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
use crate::core::document::field::Field;
use crate::core::document::field_type::FieldType;
use crate::core::document::stored_field::StoredField;
use crate::core::index::index_reader::IndexReader;

use crate::core::index::stored_fields::StoredFields;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, random,
};

#[allow(dead_code)] // for quick search
pub struct TestBinaryDocument;
/// Tests {@link Document} class.
#[test]
fn test_binary_field_in_index() -> Result<()> {
  let binary_val_stored = "this text will be stored as a byte array in the index";
  let _binary_val_compressed =
    "this text will be also stored and compressed as a byte array in the index";

  // create a stored FieldType
  let mut ft = FieldType::new();
  ft.set_stored(true)?;

  // StoredField with binary value and Field with string value
  let binary_fld_stored =
    StoredField::from_binary("binaryStored", binary_val_stored.as_bytes().to_vec())?;
  let string_fld_stored = Field::from_string("stringStored", binary_val_stored, ft)?;

  let mut doc = Document::new();
  doc.add(binary_fld_stored);
  doc.add(string_fld_stored);

  // test for field count
  assert_eq!(2, doc.get_fields().len());

  // add the doc to an index
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone());
  writer.add_document(doc)?;

  // open a reader and fetch the document
  let reader = writer.get_reader()?;
  let doc_from_reader = reader.stored_fields()?.document(0)?;
  assert!(!doc_from_reader.get_fields().is_empty());

  // fetch the binary stored field and compare with the original
  let bytes = doc_from_reader.get_binary_value("binaryStored")?;
  assert!(bytes.is_some());
  let binary_fld_stored_test = bytes.unwrap().as_ref().utf8_to_string()?;
  assert_eq!(binary_fld_stored_test, binary_val_stored);

  // fetch the string field and compare with the original
  let string_fld_stored_test = doc_from_reader.get("stringStored")?;
  assert!(string_fld_stored_test.is_some());
  assert_eq!(string_fld_stored_test.unwrap().as_ref(), binary_val_stored);

  writer.close()?;

  Ok(())
}

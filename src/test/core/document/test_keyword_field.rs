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

use crate::core::document::field::{FieldBase, FieldDataEnum, Store};
use crate::core::document::keyword_field::KeywordField;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::terms::Terms;
use crate::core::index::{BytesRef, directory_reader};
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::lucene_test_case::{
  get_only_leaf_reader, new_bytes_ref_from_string, new_directory_shared, new_index_writer_config,
  random,
};
use crate::test::core::util::test_util::TestUtil;
use std::borrow::Cow;

#[allow(dead_code)] // for quick search
struct TestKeywordField;
#[test]
fn test_set_bytes_value() -> Result<()> {
  let mut random = random();
  let fields: Vec<KeywordField> = vec![
    KeywordField::from_bytes_ref(
      "name",
      new_bytes_ref_from_string(&mut random, "value")?,
      Store::No,
    )?,
    KeywordField::from_bytes_ref(
      "name",
      new_bytes_ref_from_string(&mut random, "value")?,
      Store::Yes,
    )?,
  ];

  for mut field in fields {
    assert_eq!(
      &new_bytes_ref_from_string(&mut random, "value")?,
      field.binary_value()?.unwrap().as_ref()
    );
    assert!(field.string_value()?.is_none());

    if field.field_type().stored() {
      let stored = field.stored_value().unwrap();
      match stored {
        FieldDataEnum::Binary(v) => {
          assert_eq!(new_bytes_ref_from_string(&mut random, "value")?, *v);
        },
        _ => unreachable!(""),
      }
    } else {
      assert!(field.stored_value().is_none());
    }

    field.set_bytes_value(new_bytes_ref_from_string(&mut random, "value2")?)?;

    assert_eq!(
      &new_bytes_ref_from_string(&mut random, "value2")?,
      field.binary_value()?.unwrap().as_ref()
    );
    assert!(field.string_value()?.is_none());

    if field.field_type().stored() {
      let stored = field.stored_value().unwrap();
      match stored {
        FieldDataEnum::Binary(v) => {
          assert_eq!(new_bytes_ref_from_string(&mut random, "value2")?, *v);
        },
        _ => unreachable!(""),
      }
    } else {
      assert!(field.stored_value().is_none());
    }
  }

  Ok(())
}
#[test]
fn test_set_string_value() -> Result<()> {
  let mut random = random();
  let fields: Vec<KeywordField> = vec![
    KeywordField::from_string("name", "value", Store::No)?,
    KeywordField::from_string("name", "value", Store::Yes)?,
  ];

  for mut field in fields {
    assert_eq!(Some(Cow::Owned("value".to_string())), field.string_value()?);
    assert_eq!(
      &new_bytes_ref_from_string(&mut random, "value")?,
      field.binary_value()?.unwrap().as_ref()
    );

    if field.field_type().stored() {
      let stored = field.stored_value().unwrap();
      match stored {
        FieldDataEnum::String(v) => {
          assert_eq!("value", v);
        },
        _ => unreachable!(""),
      }
    } else {
      assert!(field.stored_value().is_none());
    }

    field.set_string_value("value2")?;

    assert_eq!(
      Some(Cow::Owned("value2".to_string())),
      field.string_value()?
    );
    assert_eq!(
      &new_bytes_ref_from_string(&mut random, "value2")?,
      field.binary_value()?.unwrap().as_ref()
    );

    if field.field_type().stored() {
      let stored = field.stored_value().unwrap();
      match stored {
        FieldDataEnum::String(v) => {
          assert_eq!("value2", v);
        },
        _ => unreachable!(""),
      }
    } else {
      assert!(field.stored_value().is_none());
    }
  }

  Ok(())
}
#[test]
fn test_index_bytes_value() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;

  w.add_document(vec![
    KeywordField::from_bytes_ref(
      "field",
      new_bytes_ref_from_string(&mut random, "value")?,
      Store::Yes,
    )?
    .into(),
  ])?;

  let reader = directory_reader::open_from_writer(&w)?;
  w.close()?;
  let leaf = get_only_leaf_reader(reader)?;
  let mut terms = leaf.terms("field")?.unwrap().iterator()?;

  assert_eq!(
    &new_bytes_ref_from_string(&mut random, "value")?,
    terms.next()?.unwrap().as_ref()
  );
  assert!(terms.next()?.is_none());

  let mut values = leaf.get_sorted_set_doc_values("field")?.unwrap();
  assert!(values.advance_exact(0)?);
  assert_eq!(1, values.doc_value_count()?);
  assert_eq!(0, values.next_ord()?);
  assert_eq!(
    &new_bytes_ref_from_string(&mut random, "value")?,
    values.lookup_ord(0)?.as_ref()
  );

  let stored_doc = leaf.stored_fields()?.document(0)?;
  let bin = stored_doc.get_binary_value("field").unwrap();
  assert_eq!(
    &new_bytes_ref_from_string(&mut random, "value")?,
    bin.unwrap().as_ref()
  );

  Ok(())
}
#[test]
fn test_index_string_value() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;

  w.add_document(vec![
    KeywordField::from_string("field", "value", Store::Yes)?.into(),
  ])?;

  let reader = directory_reader::open_from_writer(&w)?;
  w.close()?;
  let leaf = get_only_leaf_reader(reader)?;
  let mut terms = leaf.terms("field")?.unwrap().iterator()?;

  assert_eq!(
    &new_bytes_ref_from_string(&mut random, "value")?,
    terms.next()?.unwrap().as_ref()
  );
  assert!(terms.next()?.is_none());

  let mut values = leaf.get_sorted_set_doc_values("field")?.unwrap();
  assert!(values.advance_exact(0)?);
  assert_eq!(1, values.doc_value_count()?);
  assert_eq!(0, values.next_ord()?);
  assert_eq!(
    &new_bytes_ref_from_string(&mut random, "value")?,
    values.lookup_ord(0)?.as_ref()
  );

  let stored_doc = leaf.stored_fields()?.document(0)?;
  let s = stored_doc.get("field")?.unwrap();
  assert_eq!("value", s.as_ref());

  Ok(())
}

#[test]
fn test_value_clone() -> Result<()> {
  let mut random = random();

  let values: Vec<BytesRef<Vec<u8>>> = (0..100)
    .map(|_| {
      let s = TestUtil::random_simple_string_range(&mut random, 10, 20);
      BytesRef::from_string(&s)
    })
    .collect();

  let expected = values.clone();
  KeywordField::new_set_query("f", values.clone())?;
  assert_eq!(expected, values);

  Ok(())
}

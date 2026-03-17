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
use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use crate::core::analysis::analyzer::Analyzer;
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_stream::{InnerTokenStreams, TokenStreamEnum2};
use crate::core::document::field::{Field, FieldBase, FieldDataEnum, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::invertable_field::InvertableType;
use crate::core::index::BytesRef;
use crate::core::index::indexable_field::IndexableField;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
pub mod keyword {
  use crate::core::document::field_type::FieldType;
  use crate::core::index::doc_values_type::DocValuesType;
  use crate::core::index::index_options::IndexOptions;
  use once_cell::sync::Lazy;

  pub(crate) static FIELD_TYPE: Lazy<FieldType> = Lazy::new(|| {
    let mut ft = FieldType::new();
    ft.set_index_options(IndexOptions::Docs)
      .expect("set_index_options");
    ft.set_omit_norms(true).expect("set_omit_norms");
    ft.set_tokenized(false).expect("set_tokenized");
    ft.set_doc_values_type(DocValuesType::SortedSet)
      .expect("set_doc_values_type");
    ft.freeze();
    ft
  });

  pub(crate) static FIELD_TYPE_STORED: Lazy<FieldType> = Lazy::new(|| {
    let mut ft = FieldType::from_ref(&*FIELD_TYPE).expect("Invalid field type");
    ft.set_stored(true).expect("set_stored");
    ft.freeze();
    ft
  });
}

pub struct KeywordField {
  parent_field: Field,
  binary_value: Option<BytesRef<Vec<u8>>>,
  has_stored_value: bool,
}

impl KeywordField {
  pub fn from_bytes_ref<T>(name: T, value: BytesRef<Vec<u8>>, store: Store) -> Result<Self>
  where
    T: Into<String>,
  {
    let store = store.into();
    let (ft, has_stored_value) = if store {
      (keyword::FIELD_TYPE_STORED.clone(), true)
    } else {
      (keyword::FIELD_TYPE.clone(), false)
    };

    let parent_field = Field::from_bytes_ref(name, value, ft)?;

    Ok(Self {
      parent_field,
      binary_value: None,
      has_stored_value,
    })
  }

  pub fn from_string<T1, T2>(name: T1, value: T2, store: Store) -> Result<Self>
  where
    T1: Into<String>,
    T2: Into<String>,
  {
    let store = store.into();
    let (ft, has_stored_value) = if store {
      (keyword::FIELD_TYPE_STORED.clone(), true)
    } else {
      (keyword::FIELD_TYPE.clone(), false)
    };

    let v = value.into();
    let binary_value = Some(BytesRef::from_string(&v));
    let parent_field = Field::from_string(name, v, ft)?;

    Ok(Self {
      parent_field,
      binary_value,
      has_stored_value,
    })
  }
}

impl FieldBase for KeywordField {
  fn set_string_value<T>(&mut self, value: T) -> Result<()>
  where
    T: Into<String>,
  {
    let v = value.into();
    self.parent_field.set_string_value(v)?;
    match &self.parent_field.fields_data {
      FieldDataEnum::String(v) => {
        self.binary_value = Some(BytesRef::from_string(v));
      },
      _ => return Err(LuceneError::illegal_state("invalid state")),
    }
    self.has_stored_value = true;
    Ok(())
  }

  fn set_bytes_value(&mut self, value: BytesRef<Vec<u8>>) -> Result<()> {
    debug_assert!(self.binary_value.is_none());
    self.parent_field.set_bytes_value(value)?;
    self.has_stored_value = true;
    Ok(())
  }
}

impl IndexableField for KeywordField {
  fn name(&self) -> &str {
    self.parent_field.name()
  }

  type FieldType = FieldType;

  fn field_type(&self) -> &Self::FieldType {
    self.parent_field.field_type()
  }

  type TokenStream = <Field as IndexableField>::TokenStream;

  fn token_stream<'a>(
    &'a mut self,
    ts: Option<&'a mut InnerTokenStreams>,
  ) -> Result<Option<TokenStreamEnum2<&'a mut InnerTokenStreams, &'a mut Self::TokenStream>>> {
    self.parent_field.token_stream(ts)
  }

  fn binary_value(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    match &self.binary_value {
      Some(v) => Ok(Some(Cow::Borrowed(v))),
      None => self.parent_field.binary_value(),
    }
  }

  fn take_binary_value(&mut self) -> Result<Option<BytesRef<Vec<u8>>>> {
    match self.binary_value.take() {
      Some(v) => Ok(Some(v)),
      None => self.parent_field.take_binary_value(),
    }
  }

  fn string_value(&self) -> Result<Option<Cow<'_, String>>> {
    self.parent_field.string_value()
  }

  fn take_string_value(&mut self) -> Result<Option<String>> {
    self.parent_field.take_string_value()
  }

  fn take_reader_value(&mut self) -> Result<Option<ReaderEnum>> {
    self.parent_field.take_reader_value()
  }

  fn numeric_value(&self) -> Result<Option<Number>> {
    self.parent_field.numeric_value()
  }

  fn stored_value(&self) -> Option<&FieldDataEnum> {
    if self.has_stored_value {
      self.parent_field.stored_value()
    } else {
      None
    }
  }

  fn take_stored_value(&mut self) -> Option<FieldDataEnum> {
    self.parent_field.take_stored_value()
  }

  fn invertable_type(&self) -> &InvertableType {
    &InvertableType::BINARY
  }

  fn init_token_stream<A>(&mut self, analyzer: &A) -> Result<()>
  where
    A: Analyzer,
  {
    self.parent_field.init_token_stream(analyzer)
  }
}

impl Display for KeywordField {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    self.parent_field.fmt(f)
  }
}

#[cfg(test)]
impl Clone for KeywordField {
  fn clone(&self) -> Self {
    Self {
      parent_field: self.parent_field.clone(),
      binary_value: self.binary_value.clone(),
      has_stored_value: self.has_stored_value,
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::core::document::field::{FieldBase, FieldDataEnum, Store};
  use crate::core::document::keyword_field::KeywordField;
  use crate::core::index::directory_reader::directory_reader_util;
  use crate::core::index::doc_values_iterator::DocValuesIterator;
  use crate::core::index::index_reader::IndexReader;
  use crate::core::index::index_writer::IndexWriter;
  use crate::core::index::indexable_field::IndexableField;
  use crate::core::index::indexable_field_type::IndexableFieldType;
  use crate::core::index::leaf_reader::LeafReader;
  use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
  use crate::core::index::stored_fields::StoredFields;
  use crate::core::index::terms::Terms;
  use crate::core::util::bytes_ref_iterator::BytesRefIterator;
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    get_only_leaf_reader, new_bytes_ref_from_string, new_directory_shared, new_index_writer_config,
    random,
  };
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
    let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

    w.add_document(vec![
      KeywordField::from_bytes_ref(
        "field",
        new_bytes_ref_from_string(&mut random, "value")?,
        Store::Yes,
      )?
      .into(),
    ])?;

    let reader = directory_reader_util::open_from_writer(&w)?;
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
    let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

    w.add_document(vec![
      KeywordField::from_string("field", "value", Store::Yes)?.into(),
    ])?;

    let reader = directory_reader_util::open_from_writer(&w)?;
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
    // TODO KeywordField.newSetQuery未实现
    Ok(())
  }
}

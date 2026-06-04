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
use crate::core::analysis::token_stream::AnalyzerTokenStreams;
use crate::core::document::field::{Field, FieldBase, FieldDataEnum, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::invertable_field::InvertableType;
use crate::core::index::BytesRef;
use crate::core::index::indexable_field::{
  IndexableField, IndexingTokenStream, ReusedIndexingTokenStream,
};
use crate::core::search::index_or_doc_values_query::IndexOrDocValuesQuery;
use crate::core::search::multi_term_query::{DOC_VALUES_REWRITE, MultiTermQuerySet};
use crate::core::search::sort_field_enum::SortFieldEnum;
use crate::core::search::sorted_set_selector::SortedSetSelectorType;
use crate::core::search::sorted_set_sort_field::SortedSetSortField;
use crate::core::search::term_in_set_query::TermInSetQuery;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
pub mod keyword {
  use crate::core::document::field_type::FieldType;
  use crate::core::index::doc_values_type::DocValuesType;
  use crate::core::index::index_options::IndexOptions;
  use std::sync::LazyLock;

  pub(crate) static FIELD_TYPE: LazyLock<FieldType> = LazyLock::new(|| {
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

  pub(crate) static FIELD_TYPE_STORED: LazyLock<FieldType> = LazyLock::new(|| {
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
  /// Create a new `SortField` for `BytesRef` values.
  ///
  /// * `field` - field name. must not be `null`.
  /// * `reverse` - true if natural order should be reversed.
  /// * `selector` - custom selector type for choosing the sort value from the set.
  pub fn new_sort_field<T>(
    field: T,
    reverse: bool,
    selector: SortedSetSelectorType,
  ) -> Result<SortFieldEnum>
  where
    T: Into<String>,
  {
    Ok(SortedSetSortField::with_selector(field, reverse, selector)?.into())
  }

  /// Create a query that matches any of the specified values. This is the keyword equivalent of
  /// `PointInSetQuery` for point fields.
  ///
  /// # Arguments
  ///
  /// * `field` - Field name.
  /// * `values` - Values to match.
  pub fn new_set_query<T>(field: T, values: Vec<BytesRef<Vec<u8>>>) -> IndexOrDocValuesQuery
  where
    T: Into<String>,
  {
    let field = field.into();
    let index_query = TermInSetQuery::new(field.clone(), values.clone());
    let dv_query = TermInSetQuery::new_with_rewrite_method(DOC_VALUES_REWRITE, field, values);
    IndexOrDocValuesQuery::new(
      MultiTermQuerySet::from(index_query),
      MultiTermQuerySet::from(dv_query),
    )
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
  fn token_stream<'a>(
    &'a mut self,
    ts: Option<&'a mut AnalyzerTokenStreams>,
    reuse_token_stream: &'a mut Option<ReusedIndexingTokenStream>,
  ) -> Result<IndexingTokenStream<'a>> {
    self.parent_field.token_stream(ts, reuse_token_stream)
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

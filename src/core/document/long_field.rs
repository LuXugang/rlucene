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
use crate::core::analysis::analyzer::Analyzer;
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_stream::AnalyzerTokenStreams;
use crate::core::document::field::{Field, FieldBase, FieldDataEnum, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::invertable_field::InvertableType;
use crate::core::document::long_field::long_field_type::{FIELD_TYPE, FIELD_TYPE_STORED};
use crate::core::document::long_point::LongPoint;

use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use crate::core::index::BytesRef;
use crate::core::index::indexable_field::{
  IndexableField, IndexingTokenStream, ReusedIndexingTokenStream,
};
use crate::core::search::index_or_doc_values_query::IndexOrDocValuesQuery;
use crate::core::search::index_sort_sorted_numeric_doc_values_range_query::IndexSortSortedNumericDocValuesRangeQuery;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
use crate::core::util::numeric_utils::NumericUtils;
use std::borrow::Cow;
use std::fmt;

/// Indexed as SortedNumeric DocValue, not stored.
pub mod long_field_type {
  use crate::core::document::field_type::FieldType;
  use crate::core::index::doc_values_type::DocValuesType;
  use crate::core::search::sort_field::SortFieldType;
  use crate::core::search::sorted_numeric_selector::SortedNumericSelectorType;
  use crate::core::search::sorted_numeric_sort_field::SortedNumericSortField;
  use crate::core::util::bit_util::BitUtil;
  use crate::core::util::error::lucene_error::Result;
  use std::sync::LazyLock;

  pub static FIELD_TYPE: LazyLock<FieldType> = LazyLock::new(|| {
    let mut ft = FieldType::new();
    ft.set_dimensions(1, BitUtil::LONG_BYTES)
      .expect("set_dimensions should not fail");
    ft.set_doc_values_type(DocValuesType::SortedNumeric)
      .expect("set_doc_values_type should not fail");
    ft.freeze();
    ft
  });
  /// Indexed as SortedNumeric DocValue, and stored.
  pub static FIELD_TYPE_STORED: LazyLock<FieldType> = LazyLock::new(|| {
    let mut ft = FieldType::from_ref(&*FIELD_TYPE).expect("should not fail");
    ft.set_stored(true)
      .expect("set_stored(true) should not fail");
    ft.freeze();
    ft
  });

  pub fn new_sort_field<S>(
    field: S,
    reverse: bool,
    selector: SortedNumericSelectorType,
  ) -> Result<SortedNumericSortField>
  where
    S: Into<String>,
  {
    SortedNumericSortField::with_selector(field, SortFieldType::Long, reverse, selector)
  }
}

pub struct LongField {
  parent_field: Field,
  stored_value: Option<FieldDataEnum>,
}

impl LongField {
  /// Creates a new `LongField`, indexing the provided value,
  /// storing it as a DocValue, and optionally as a stored field.
  pub fn new<T>(name: T, value: i64, stored: Store) -> Result<LongField>
  where
    T: Into<String>,
  {
    let stored = stored.into();
    let (field_type, stored_value) = if stored {
      (FIELD_TYPE_STORED.clone(), Some(value.into()))
    } else {
      (FIELD_TYPE.clone(), None)
    };
    let parent_field = Field::new(name, value, field_type);
    Ok(LongField {
      parent_field,
      stored_value,
    })
  }
  pub fn new_exact_query(
    field: &str,
    value: i64,
  ) -> Result<IndexSortSortedNumericDocValuesRangeQuery> {
    Self::new_range_query(field, value, value)
  }

  pub fn new_range_query(
    field: &str,
    lower_value: i64,
    upper_value: i64,
  ) -> Result<IndexSortSortedNumericDocValuesRangeQuery> {
    let fallback_query = IndexOrDocValuesQuery::new(
      LongPoint::new_range_query(field, lower_value, upper_value)?,
      SortedNumericDocValuesField::new_slow_range_query(field, lower_value, upper_value),
    );

    Ok(IndexSortSortedNumericDocValuesRangeQuery::new(
      field,
      lower_value,
      upper_value,
      fallback_query,
    ))
  }
}

impl FieldBase for LongField {
  fn set_long_value(&mut self, value: i64) -> Result<()> {
    self.parent_field.set_long_value(value)?;
    if self.stored_value.is_some() {
      self.stored_value = Some(value.into());
    }
    Ok(())
  }
}

impl IndexableField for LongField {
  fn name(&self) -> &str {
    self.parent_field.name()
  }

  type FieldType = FieldType;

  fn field_type(&self) -> &Self::FieldType {
    self.parent_field.field_type()
  }
  fn token_stream<'a>(
    &'a mut self,
    token_stream: &'a mut AnalyzerTokenStreams,
    reuse_token_stream: &'a mut Option<ReusedIndexingTokenStream>,
  ) -> Result<IndexingTokenStream<'a>> {
    self
      .parent_field
      .token_stream(token_stream, reuse_token_stream)
  }

  fn binary_value(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    match &self.parent_field.fields_data {
      FieldDataEnum::Number(Number::I64(v)) => {
        let mut bytes = vec![0u8; BitUtil::LONG_BYTES];
        NumericUtils::long_to_sortable_bytes(*v, &mut bytes, 0);
        Ok(Some(Cow::Owned(BytesRef::from_bytes(bytes))))
      },
      _ => Err(LuceneError::illegal_state(
        "parent_field`s fields_data does not have a long value",
      )),
    }
  }

  fn take_binary_value(&mut self) -> Result<Option<BytesRef<Vec<u8>>>> {
    self.binary_value().map(|v| v.map(|c| c.into_owned()))
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
    self.stored_value.as_ref()
  }

  fn take_stored_value(&mut self) -> Option<FieldDataEnum> {
    self.parent_field.take_stored_value()
  }

  fn invertable_type(&self) -> &InvertableType {
    self.parent_field.invertable_type()
  }

  fn init_token_stream<A>(&mut self, analyzer: &A) -> Result<()>
  where
    A: Analyzer,
  {
    self.parent_field.init_token_stream(analyzer)
  }
}

impl fmt::Display for LongField {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{} <{}:{}>",
      std::any::type_name::<Self>(),
      self.parent_field.name(),
      self.parent_field.fields_data
    )
  }
}

#[cfg(test)]
impl Clone for LongField {
  fn clone(&self) -> Self {
    Self {
      parent_field: self.parent_field.clone(),
      stored_value: self.stored_value.clone(),
    }
  }
}

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
use crate::core::document::float_point::FloatPoint;
use crate::core::document::invertable_field::InvertableType;

use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use crate::core::index::BytesRef;
use crate::core::index::indexable_field::{
  IndexableField, IndexingTokenStream, ReusedIndexingTokenStream,
};
use crate::core::search::index_or_doc_values_query::IndexOrDocValuesQuery;
use crate::core::util::TryIntoInt;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
use crate::core::util::numeric_utils::NumericUtils;
use std::borrow::Cow;
use std::fmt;

pub mod float_field_type {
  use crate::core::document::field_type::FieldType;
  use crate::core::index::doc_values_type::DocValuesType;
  use crate::core::util::bit_util::BitUtil;
  use std::sync::LazyLock;

  pub static FIELD_TYPE: LazyLock<FieldType> = LazyLock::new(|| {
    let mut ft = FieldType::new();
    ft.set_dimensions(1, BitUtil::FLOAT_BYTES)
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
}
pub struct FloatField {
  parent_field: Field,
  stored_value: Option<FieldDataEnum>,
}

impl FloatField {
  /// Creates a new `FloatField`, indexing the provided value,
  /// storing it as a DocValue, and optionally as a stored field.
  pub fn new<T>(name: T, value: f32, stored: Store) -> Result<FloatField>
  where
    T: Into<String>,
  {
    let stored = stored.into();
    let (field_type, stored_value) = if stored {
      (
        float_field_type::FIELD_TYPE_STORED.clone(),
        Some(value.into()),
      )
    } else {
      (float_field_type::FIELD_TYPE.clone(), None)
    };
    let sortable_long = NumericUtils::float_to_sortable_int(value) as i64;
    let parent_field = Field::new(name, sortable_long, field_type);
    Ok(FloatField {
      parent_field,
      stored_value,
    })
  }
  pub fn new_exact_query<T>(field: T, value: f32) -> Result<IndexOrDocValuesQuery>
  where
    T: Into<String>,
  {
    Self::new_range_query(field, value, value)
  }

  pub fn new_range_query<T>(
    field: T,
    lower_value: f32,
    upper_value: f32,
  ) -> Result<IndexOrDocValuesQuery>
  where
    T: Into<String>,
  {
    let field = field.into();
    Ok(IndexOrDocValuesQuery::new(
      FloatPoint::new_range_query(field.clone(), lower_value, upper_value)?,
      SortedNumericDocValuesField::new_slow_range_query(
        field,
        NumericUtils::float_to_sortable_int(lower_value) as i64,
        NumericUtils::float_to_sortable_int(upper_value) as i64,
      ),
    ))
  }

  /// Create a query that matches any of the specified values.
  ///
  /// # Arguments
  ///
  /// * `field` - Field name.
  /// * `values` - Values to match.
  pub fn new_set_query<T>(field: T, values: Vec<f32>) -> Result<IndexOrDocValuesQuery>
  where
    T: Into<String>,
  {
    let field = field.into();
    let point_query = FloatPoint::new_set_query(field.clone(), values.clone())?;
    let dv_query = SortedNumericDocValuesField::new_slow_set_query(
      field,
      values
        .into_iter()
        .map(|v| NumericUtils::float_to_sortable_int(v) as i64)
        .collect(),
    )?;
    Ok(IndexOrDocValuesQuery::new(point_query, dv_query))
  }

  /// Convert the stored sortable int back into a float.
  fn get_value_as_float(&self) -> Result<f32> {
    match self.numeric_value()? {
      None => Err(LuceneError::illegal_state(
        "field does not have a numeric value",
      )),
      Some(n) => match n {
        Number::I64(v) => Ok(NumericUtils::sortable_int_to_float(v.try_convert()?)),
        _ => Err(LuceneError::illegal_state(
          "numeric value is not a sortable int float",
        )),
      },
    }
  }
}

impl FieldBase for FloatField {
  fn set_float_value(&mut self, value: f32) -> Result<()> {
    let sortable = NumericUtils::float_to_sortable_int(value) as i64;
    self.parent_field.set_long_value(sortable)?;
    if self.stored_value.is_some() {
      self.stored_value = Some(value.into());
    }
    Ok(())
  }

  fn set_long_value(&mut self, _value: i64) -> Result<()> {
    Err(LuceneError::illegal_argument(
      "cannot change value type from Float to Long",
    ))
  }
}

impl IndexableField for FloatField {
  fn name(&self) -> &str {
    self.parent_field.name()
  }

  type FieldType = FieldType;

  fn field_type(&self) -> &Self::FieldType {
    self.parent_field.field_type()
  }
  fn token_stream<'a>(
    &'a mut self,
    token_stream: Option<&'a mut AnalyzerTokenStreams>,
    reuse_token_stream: &'a mut Option<ReusedIndexingTokenStream>,
  ) -> Result<IndexingTokenStream<'a>> {
    self
      .parent_field
      .token_stream(token_stream, reuse_token_stream)
  }

  fn binary_value(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    let mut encode_point = vec![0u8; BitUtil::FLOAT_BYTES];
    let value = self.get_value_as_float()?;
    FloatPoint::encode_dimension(value, &mut encode_point, 0);
    Ok(Some(Cow::Owned(BytesRef::from_bytes(encode_point))))
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

impl fmt::Display for FloatField {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let v = self.get_value_as_float().expect("should get float value");
    write!(
      f,
      "{} <{}:{}>",
      std::any::type_name::<Self>(),
      self.parent_field.name(),
      v
    )
  }
}

#[cfg(test)]
impl Clone for FloatField {
  fn clone(&self) -> Self {
    Self {
      parent_field: self.parent_field.clone(),
      stored_value: self.stored_value.clone(),
    }
  }
}

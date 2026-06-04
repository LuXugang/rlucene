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
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Per-segment, per-document double values, which can be calculated at
/// search-time.
pub trait DoubleValues {
  /// Get the double value for the current document.
  fn double_value(&mut self) -> Result<f64>;

  /// Advance this instance to the given document id.
  ///
  /// # Returns
  /// `true` if there is a value for this document.
  fn advance_exact(&mut self, doc: i32) -> Result<bool>;
}

/// Wraps a [`DoubleValues`] instance, returning a default if the wrapped
/// instance has no value.
pub fn with_default<T>(in_: T, missing_value: f64) -> WithDefaultDoubleValues<T>
where
  T: DoubleValues,
{
  WithDefaultDoubleValues {
    in_,
    missing_value,
    has_value: false,
  }
}

pub struct WithDefaultDoubleValues<T>
where
  T: DoubleValues,
{
  in_: T,
  missing_value: f64,
  has_value: bool,
}

impl<T> DoubleValues for WithDefaultDoubleValues<T>
where
  T: DoubleValues,
{
  fn double_value(&mut self) -> Result<f64> {
    if self.has_value {
      self.in_.double_value()
    } else {
      Ok(self.missing_value)
    }
  }

  fn advance_exact(&mut self, doc: i32) -> Result<bool> {
    self.has_value = self.in_.advance_exact(doc)?;
    Ok(true)
  }
}

/// An empty [`DoubleValues`] instance that always returns `false` from
/// [`advance_exact`](DoubleValues::advance_exact).
#[derive(Debug, Clone, Copy, Default)]
pub struct EmptyDoubleValues;

impl DoubleValues for EmptyDoubleValues {
  fn double_value(&mut self) -> Result<f64> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn advance_exact(&mut self, _doc: i32) -> Result<bool> {
    Ok(false)
  }
}

pub const EMPTY: EmptyDoubleValues = EmptyDoubleValues;

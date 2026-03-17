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
use strum_macros::{Display, FromRepr};

use crate::core::index::doc_values_type::DocValuesType;

/// Options for skip indexes on doc values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr, Hash, Display)]
#[repr(u8)]
pub enum DocValuesSkipIndexType {
  /// No skip index should be created.
  None,
  /// Record range of values. Suitable for:
  /// - `Numeric`
  /// - `SortedNumeric`
  /// - `Sorted`
  /// - `SortedSet`
  ///
  /// Records min/max values per range of doc IDs.
  Range,
}

impl DocValuesSkipIndexType {
  /// Checks compatibility with a specific doc values type
  pub fn is_compatible_with(&self, dv_type: DocValuesType) -> bool {
    match self {
      Self::None => true,
      Self::Range => matches!(
        dv_type,
        DocValuesType::Numeric
          | DocValuesType::SortedNumeric
          | DocValuesType::Sorted
          | DocValuesType::SortedSet
      ),
    }
  }
}
/// Use Default for padding
impl Default for DocValuesSkipIndexType {
  fn default() -> Self {
    Self::None
  }
}

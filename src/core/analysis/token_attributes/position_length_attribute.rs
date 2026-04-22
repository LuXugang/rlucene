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
use crate::core::util::attribute::Attribute;
use crate::core::util::error::lucene_error::Result;

/// Determines how many positions this token spans. Very few analyzer components actually produce
/// this attribute, and indexing ignores it, but it's useful to express the graph structure naturally
/// produced by decompounding, word splitting/joining, synonym filtering, etc.
///
/// **Note:** this is optional, and most analyzers don’t change the default value (`1`).
pub trait PositionLengthAttribute: Attribute {
  #[cfg(test)]
  const ATTRIBUTE_NAME: &'static str = "PositionLengthAttribute";

  /// Set the position length of this Token.
  ///
  /// The default value is `1`.
  ///
  /// # Parameters
  ///
  /// - `position_length`: how many positions this token spans.
  ///
  /// # Error
  ///
  /// Error if `position_length <= 0`.
  ///
  /// # See
  ///
  /// [`get_position_length`](PositionLengthAttribute::get_position_length)
  fn set_position_length(&mut self, position_length: i32) -> Result<()>;

  /// Returns the position length of this Token.
  ///
  /// # See
  ///
  /// [`set_position_length`](PositionLengthAttribute::set_position_length)
  fn get_position_length(&self) -> i32;
}

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

/// The start and end character offset of a token.
pub trait OffsetAttribute: Attribute {
  #[cfg(test)]
  const ATTRIBUTE_NAME: &'static str = "OffsetAttribute";

  /// Returns this token's starting offset, the position of the first
  /// character in the source text.
  ///
  /// See also: [`Self::set_offset`]
  fn start_offset(&self) -> i32;

  /// Sets the starting and ending offset.
  ///
  /// # Errors
  ///
  /// Implementations should throw errors if `start_offset` or `end_offset`
  /// are negative, or if `start_offset > end_offset`.
  ///
  /// See also: [`Self::start_offset`], [`Self::end_offset`]
  fn set_offset(&mut self, start_offset: i32, end_offset: i32) -> Result<()>;

  /// Returns this token's ending offset, one greater than the position of the
  /// last character in the source text.
  ///
  /// The length of the token in the source text is `end_offset() -
  /// start_offset()`.
  ///
  /// See also: [`Self::set_offset`]
  fn end_offset(&self) -> i32;
}

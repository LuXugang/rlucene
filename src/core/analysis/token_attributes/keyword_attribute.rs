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

/// This attribute can be used to mark a token as a keyword. Keyword aware [`TokenStream`](crate::core::analysis::token_stream::TokenStream)s can
/// decide to modify a token based on the return value of [`is_keyword`](KeywordAttribute::is_keyword)
/// if the token is modified. Stemming filters for instance can use this attribute to conditionally
/// skip a term if [`is_keyword`](KeywordAttribute::is_keyword) returns `true`.
pub trait KeywordAttribute: Attribute {
  #[cfg(debug_assertions)]
  const ATTRIBUTE_NAME: &'static str = "KeywordAttribute";

  /// Returns `true` if the current token is a keyword, otherwise `false`.
  ///
  /// # See
  ///
  /// [`set_keyword`](KeywordAttribute::set_keyword)
  fn is_keyword(&self) -> Result<bool>;

  /// Marks the current token as keyword if set to `true`.
  ///
  /// # Parameters
  ///
  /// - `is_keyword`: `true` if the current token is a keyword, otherwise `false`.
  ///
  /// # See
  ///
  /// [`is_keyword`](KeywordAttribute::is_keyword)
  fn set_keyword(&mut self, is_keyword: bool) -> Result<()>;
}

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

/// Add this [`Attribute`] to a [`TermsEnum`](crate::core::index::terms_enum::TermsEnum) returned by
/// `MultiTermQuery::get_terms_enum` and update the boost on each returned term.
/// This enables to control the boost factor for each matching term in
/// `MultiTermQuery::SCORING_BOOLEAN_REWRITE` or `TopTermsRewrite` mode.
/// `FuzzyQuery` is using this to take the edit distance into account.
///
/// **Please note:** This attribute is intended to be added only by the `TermsEnum`
/// to itself in its constructor and consumed by the `MultiTermQuery::RewriteMethod`.
///
/// @lucene.internal
pub trait BoostAttribute: Attribute {
  #[cfg(test)]
  const ATTRIBUTE_NAME: &'static str = "BoostAttribute";

  /// Sets the boost in this attribute.
  fn set_boost(&mut self, boost: f32);

  /// Retrieves the boost, default is `1.0`.
  fn get_boost(&self) -> f32;
}
/// Default boost value = `1.0`.
pub(crate) const DEFAULT_BOOST: f32 = 1.0;

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
use crate::core::index::BytesRef;
use crate::core::util::attribute::Attribute;

/// Add this `Attribute` to a fresh `AttributeSource` before calling
/// `MultiTermQuery::get_terms_enum`. `FuzzyQuery` is using this to control
/// its internal behaviour to only return competitive terms.
///
/// **Please note:** This attribute is intended to be added by the
/// `MultiTermQueryRewriteMethod` to an empty `AttributeSource` that is shared
/// for all segments during query rewrite. This attribute source is passed to all
/// segment enums on `MultiTermQuery::get_terms_enum`. `TopTermsRewrite` uses
/// this attribute to inform all enums about the current boost, that is not
/// competitive.
///
/// @lucene.internal
pub trait MaxNonCompetitiveBoostAttribute: Attribute {
  /// This is the maximum boost that would not be competitive.
  fn set_max_non_competitive_boost(&mut self, max_non_competitive_boost: f32);

  /// This is the maximum boost that would not be competitive. Default is
  /// negative infinity, which means every term is competitive.
  fn get_max_non_competitive_boost(&self) -> f32;

  /// This is the term or `None` of the term that triggered the boost change.
  fn set_competitive_term(&mut self, competitive_term: Option<BytesRef<Vec<u8>>>);

  /// This is the term or `None` of the term that triggered the boost change.
  /// Default is `None`, which means every term is competitoive.
  fn get_competitive_term(&self) -> Option<&BytesRef<Vec<u8>>>;
}

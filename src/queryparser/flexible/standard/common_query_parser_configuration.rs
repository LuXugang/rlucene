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
use crate::core::analysis::analyzer::AnalyzerEnum;
use crate::core::document::date_tools::Resolution;
use crate::core::search::multi_term_query::RewriteMethodEnum;

/// Locale used by date range parsing.
///
/// Java uses `java.util.Locale` here. Rust Lucene does not yet have a locale
/// abstraction, so this type keeps the locale identifier while preserving a
/// distinct configuration type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Locale(pub String);

/// Time zone used by date range parsing.
///
/// Java uses `java.util.TimeZone` here. Rust Lucene does not yet have a
/// time-zone database abstraction in the query parser layer, so this type keeps
/// the time-zone identifier while preserving a distinct configuration type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeZone(pub String);

/// Configuration options common across queryparser implementations.
pub trait CommonQueryParserConfiguration {
  /// Set to `true` to allow leading wildcard characters.
  ///
  /// When set, `*` or `?` are allowed as the first character of a prefix query
  /// and wildcard query. Note that this can produce very slow queries on big
  /// indexes.
  ///
  /// Default: `false`.
  fn set_allow_leading_wildcard(&mut self, allow_leading_wildcard: bool);

  /// Set to `true` to enable position increments in result query.
  ///
  /// When set, result phrase and multi-phrase queries will be aware of position
  /// increments. This is useful when, for example, a stop filter increases the
  /// position increment of the token that follows an omitted token.
  ///
  /// Default: `false`.
  fn set_enable_position_increments(&mut self, enabled: bool);

  /// Returns whether position increments are enabled in result query.
  ///
  /// See [`set_enable_position_increments`](Self::set_enable_position_increments).
  fn get_enable_position_increments(&self) -> bool;

  /// Sets the rewrite method used when creating multi-term queries.
  ///
  /// By default query parsers use a constant-score blended rewrite method when
  /// creating prefix, wildcard or term range queries. This implementation is
  /// generally preferable because it runs faster, does not have the scarcity of
  /// terms unduly influence score, and avoids too-many-clauses errors.
  ///
  /// Applications that need boolean expansion rewriting can change the rewrite
  /// method through this setting. As another alternative, all terms can be
  /// rewritten as a filter up-front with a constant-score rewrite.
  fn set_multi_term_rewrite_method<R>(&mut self, method: R)
  where
    R: Into<RewriteMethodEnum>;

  /// Returns the rewrite method used when creating multi-term queries.
  ///
  /// See [`set_multi_term_rewrite_method`](Self::set_multi_term_rewrite_method).
  fn get_multi_term_rewrite_method(&self) -> Option<&RewriteMethodEnum>;

  /// Set the prefix length for fuzzy queries.
  ///
  /// Default is `0`.
  fn set_fuzzy_prefix_length(&mut self, fuzzy_prefix_length: i32);

  /// Set locale used by date range parsing.
  fn set_locale(&mut self, locale: Locale);

  /// Returns current locale, allowing access by subclasses.
  fn get_locale(&self) -> Option<&Locale>;

  fn set_time_zone(&mut self, time_zone: TimeZone);

  fn get_time_zone(&self) -> Option<&TimeZone>;

  /// Sets the default slop for phrases.
  ///
  /// If zero, then exact phrase matches are required. Default value is zero.
  fn set_phrase_slop(&mut self, default_phrase_slop: i32);

  fn get_analyzer(&self) -> Option<&AnalyzerEnum>;

  /// Returns whether leading wildcard characters are allowed.
  ///
  /// See [`set_allow_leading_wildcard`](Self::set_allow_leading_wildcard).
  fn get_allow_leading_wildcard(&self) -> bool;

  /// Get the minimal similarity for fuzzy queries.
  fn get_fuzzy_min_sim(&self) -> f32;

  /// Get the prefix length for fuzzy queries.
  fn get_fuzzy_prefix_length(&self) -> i32;

  /// Gets the default slop for phrases.
  fn get_phrase_slop(&self) -> i32;

  /// Set the minimum similarity for fuzzy queries.
  ///
  /// Default is defined by fuzzy query defaults.
  fn set_fuzzy_min_sim(&mut self, fuzzy_min_sim: f32);

  /// Sets the default [`Resolution`] used for certain field when no
  /// [`Resolution`] is defined for this field.
  fn set_date_resolution(&mut self, date_resolution: Resolution);
}

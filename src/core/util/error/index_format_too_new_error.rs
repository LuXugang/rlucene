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
use std::fmt;

use crate::core::util::error::lucene_error::LuceneError;

/// This error is returned when Lucene detects an index that is newer than this
/// Lucene version.
#[derive(Debug, Clone)]
pub struct IndexFormatTooNewError {
  resource_description: String,
  version: i32,
  min_version: i32,
  max_version: i32,
  source: Option<Box<LuceneError>>,
}

impl IndexFormatTooNewError {
  /// Creates an [`IndexFormatTooNewError`].
  ///
  /// `resource_description` describes the file that was too new, `version` is
  /// the version of the file that was too new, and `min_version` and
  /// `max_version` are the minimum and maximum versions accepted.
  pub fn new(
    resource_description: impl Into<String>,
    version: i32,
    min_version: i32,
    max_version: i32,
  ) -> Self {
    Self {
      resource_description: resource_description.into(),
      version,
      min_version,
      max_version,
      source: None,
    }
  }

  /// Creates an [`IndexFormatTooNewError`].
  ///
  /// `input` is the open file that is too new, `version` is the version of the
  /// file that was too new, and `min_version` and `max_version` are the minimum
  /// and maximum versions accepted.
  pub fn from_input(
    input: &impl fmt::Display,
    version: i32,
    min_version: i32,
    max_version: i32,
  ) -> Self {
    Self::new(input.to_string(), version, min_version, max_version)
  }

  /// Returns a description of the file that was too new.
  pub fn get_resource_description(&self) -> &str {
    &self.resource_description
  }

  /// Returns the version of the file that was too new.
  pub fn get_version(&self) -> i32 {
    self.version
  }

  /// Returns the maximum version accepted.
  pub fn get_max_version(&self) -> i32 {
    self.max_version
  }

  /// Returns the minimum version accepted.
  pub fn get_min_version(&self) -> i32 {
    self.min_version
  }

  pub fn add_suppressed(&mut self, source: LuceneError) {
    self.source = Some(Box::new(source));
  }

  pub fn get_suppressed(&self) -> Option<&LuceneError> {
    self.source.as_deref()
  }
}

impl fmt::Display for IndexFormatTooNewError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Format version is not supported (resource {}): {} (needs to be between {} and {})",
      self.resource_description, self.version, self.min_version, self.max_version
    )
  }
}

impl std::error::Error for IndexFormatTooNewError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    self
      .source
      .as_deref()
      .map(|error| error as &dyn std::error::Error)
  }
}

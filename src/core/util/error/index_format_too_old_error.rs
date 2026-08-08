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
use crate::core::util::version::MIN_SUPPORTED_MAJOR;

/// This error is returned when Lucene detects an index that is too old for
/// this Lucene version.
#[derive(Debug, Clone)]
pub struct IndexFormatTooOldError {
  resource_description: String,
  reason: Option<String>,
  version: Option<i32>,
  min_version: Option<i32>,
  max_version: Option<i32>,
  source: Option<Box<LuceneError>>,
}

impl IndexFormatTooOldError {
  /// Creates an [`IndexFormatTooOldError`].
  ///
  /// `resource_description` describes the file that was too old, and `reason`
  /// is the reason for this error if the version is not available.
  pub fn new(resource_description: impl Into<String>, reason: impl Into<String>) -> Self {
    Self {
      resource_description: resource_description.into(),
      reason: Some(reason.into()),
      version: None,
      min_version: None,
      max_version: None,
      source: None,
    }
  }

  /// Creates an [`IndexFormatTooOldError`].
  ///
  /// `input` is the open file that is too old, and `reason` is the reason for
  /// this error if the version is not available.
  pub fn from_input(input: &impl fmt::Display, reason: impl Into<String>) -> Self {
    Self::new(input.to_string(), reason)
  }

  /// Creates an [`IndexFormatTooOldError`].
  ///
  /// `resource_description` describes the file that was too old, `version` is
  /// the version of the file that was too old, and `min_version` and
  /// `max_version` are the minimum and maximum versions accepted.
  pub fn with_version(
    resource_description: impl Into<String>,
    version: i32,
    min_version: i32,
    max_version: i32,
  ) -> Self {
    Self {
      resource_description: resource_description.into(),
      reason: None,
      version: Some(version),
      min_version: Some(min_version),
      max_version: Some(max_version),
      source: None,
    }
  }

  /// Creates an [`IndexFormatTooOldError`].
  ///
  /// `input` is the open file that is too old, `version` is the version of the
  /// file that was too old, and `min_version` and `max_version` are the minimum
  /// and maximum versions accepted.
  pub fn from_input_with_version(
    input: &impl fmt::Display,
    version: i32,
    min_version: i32,
    max_version: i32,
  ) -> Self {
    Self::with_version(input.to_string(), version, min_version, max_version)
  }

  /// Returns a description of the file that was too old.
  pub fn get_resource_description(&self) -> &str {
    &self.resource_description
  }

  /// Returns an optional reason for this error if the version information was
  /// not available. Otherwise returns [`None`].
  pub fn get_reason(&self) -> Option<&str> {
    self.reason.as_deref()
  }

  /// Returns the version of the file that was too old. This method returns
  /// [`None`] if an alternative [`get_reason`](Self::get_reason) is provided.
  pub fn get_version(&self) -> Option<i32> {
    self.version
  }

  /// Returns the maximum version accepted. This method returns [`None`] if an
  /// alternative [`get_reason`](Self::get_reason) is provided.
  pub fn get_max_version(&self) -> Option<i32> {
    self.max_version
  }

  /// Returns the minimum version accepted. This method returns [`None`] if an
  /// alternative [`get_reason`](Self::get_reason) is provided.
  pub fn get_min_version(&self) -> Option<i32> {
    self.min_version
  }

  pub fn add_suppressed(&mut self, source: LuceneError) {
    match self.source.as_mut() {
      Some(suppressed) => suppressed.add_suppressed(source),
      None => self.source = Some(Box::new(source)),
    }
  }

  pub fn get_suppressed(&self) -> Option<&LuceneError> {
    self.source.as_deref()
  }
}

impl fmt::Display for IndexFormatTooOldError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Format version is not supported (resource {}): ",
      self.resource_description
    )?;
    if let Some(reason) = &self.reason {
      write!(
        f,
        "{}. This version of Lucene only supports indexes created with release {}.0 and later by default.",
        reason, *MIN_SUPPORTED_MAJOR
      )
    } else {
      write!(
        f,
        "{} (needs to be between {} and {}). This version of Lucene only supports indexes created with release {}.0 and later.",
        self.version.unwrap(),
        self.min_version.unwrap(),
        self.max_version.unwrap(),
        *MIN_SUPPORTED_MAJOR
      )
    }
  }
}

impl std::error::Error for IndexFormatTooOldError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    self
      .source
      .as_deref()
      .map(|error| error as &dyn std::error::Error)
  }
}

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
use crate::core::index::query_timeout::QueryTimeout;
use std::time::{Duration, Instant};

/// An implementation of [`QueryTimeout`] that can be used by the `ExitableDirectoryReader`
/// class to time out and exit out when a query takes a long time to rewrite.
pub struct QueryTimeoutImpl {
  /// The local variable to store the time beyond which, the processing should exit.
  timeout_at: Option<Instant>,
}

impl QueryTimeoutImpl {
  /// Sets the time at which to time out by adding the given `time_allowed` to the current time.
  ///
  /// # Arguments
  ///
  /// * `time_allowed` — Number of milliseconds after which to time out.
  ///   Use `i64::MAX` to effectively never time out.
  pub fn new(mut time_allowed: i64) -> Self {
    if time_allowed < 0 {
      time_allowed = i64::MAX;
    }

    let duration = if time_allowed == i64::MAX {
      None
    } else {
      Some(Duration::from_millis(time_allowed as u64))
    };

    let timeout_at = duration.map(|d| Instant::now() + d);
    Self { timeout_at }
  }

  /// Returns time at which to time out, in [`Instant`].
  /// Can be compared directly with `Instant::now()`.
  pub fn get_timeout_at(&self) -> Option<Instant> {
    self.timeout_at
  }

  /// Reset the timeout value.
  pub fn reset(&mut self) {
    self.timeout_at = None;
  }
}

impl std::fmt::Display for QueryTimeoutImpl {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "timeout_at: {:?} (Instant::now(): {:?})",
      self.timeout_at,
      Instant::now()
    )
  }
}

impl QueryTimeout for QueryTimeoutImpl {
  /// Return `true` if [`reset`](Self::reset) has not been called and
  /// the elapsed time has exceeded the time allowed.
  fn should_exit(&self) -> bool {
    if let Some(timeout_at) = self.timeout_at {
      Instant::now() > timeout_at
    } else {
      false
    }
  }
}

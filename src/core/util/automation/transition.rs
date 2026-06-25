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
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::Result;

/// Holds one transition from an automaton. This is typically used temporarily
/// when iterating through transitions via
/// [`TransitionAccessor::init_transition`](crate::core::util::automation::transition_accessor::TransitionAccessor::init_transition)
/// and [`TransitionAccessor::get_next_transition`](crate::core::util::automation::transition_accessor::TransitionAccessor::get_next_transition).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Transition {
  /// Source state.
  pub source: i32,
  /// Destination state.
  pub dest: i32,
  /// Minimum accepted label (inclusive).
  pub min: i32,
  /// Maximum accepted label (inclusive).
  pub max: i32,
  /// Remembers where we are in the iteration; initialized to -1 to provoke
  /// an error if `get_next_transition` is called without first
  /// `init_transition`.
  pub transition_upto: i32,
}
/// Inline size of a `Transition` instance.
pub const BYTES_USED: usize = size_of::<Transition>();

impl Default for Transition {
  /// Creates a `Transition` with zeroed fields and `transition_upto` set to
  /// -1.
  fn default() -> Self {
    Transition {
      source: 0,
      dest: 0,
      min: 0,
      max: 0,
      transition_upto: -1,
    }
  }
}

impl Accountable for Transition {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(0)
  }
}

impl std::fmt::Display for Transition {
  /// Formats the transition as `source --> dest minChar-maxChar`.
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{} --> {} {}-{}",
      self.source, self.dest, self.min as u8 as char, self.max as u8 as char
    )
  }
}

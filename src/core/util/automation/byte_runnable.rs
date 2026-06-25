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
use crate::core::util::error::lucene_error::Result;

/// A runnable automaton accepting byte array as input
pub trait ByteRunnable {
  /// Returns the state obtained by reading the given byte from the given
  /// state.
  ///
  /// Returns -1 if not obtaining any such state.
  ///
  /// # Parameters
  /// - `state`: the last state
  /// - `c`: the input codepoint
  ///
  /// # Returns
  /// The next state, or -1 if no such transition.
  fn step(&mut self, state: i32, c: i32) -> Result<i32>;

  /// Returns acceptance status for given state.
  ///
  /// # Parameters
  /// - `state`: the state
  ///
  /// # Returns
  /// Whether the state is accepted.
  fn is_accept(&self, state: i32) -> Result<bool>;

  /// Returns number of states this automaton has.
  ///
  /// Note: This may not be an accurate number in case of an NFA.
  ///
  /// # Returns
  /// Number of states.
  fn get_size(&self) -> i32;

  /// Returns true if the given byte array is accepted by this automaton.
  ///
  /// # Parameters
  /// - `s`: input byte slice
  /// - `offset`: start index
  /// - `length`: number of bytes to read
  ///
  /// # Returns
  /// Whether the automaton accepts the input.
  fn run(&mut self, s: &[u8], offset: usize, length: usize) -> Result<bool> {
    let mut p = 0;
    let end = offset + length;
    for &b in &s[offset..end] {
      p = self.step(p, b as i32)?;
      if p == -1 {
        return Ok(false);
      }
    }
    self.is_accept(p)
  }
}

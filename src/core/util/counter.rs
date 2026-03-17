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
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;
use std::sync::atomic::AtomicI64;

pub trait Counter {
  /// Adds the given delta to the counter's current value.
  ///
  /// # Arguments
  /// * `delta` - The delta to add.
  ///
  /// # Returns
  /// The counter's updated value.
  fn add_and_get(&self, delta: i64) -> i64;

  fn add_and_get_mut(&mut self, _delta: i64) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }
  /// Returns the counter's current value.
  ///
  /// # Returns
  /// The counter's current value.
  fn get(&self) -> i64;
}
#[derive(Debug)]
pub struct AtomicCounter {
  count: AtomicI64,
}
impl Default for AtomicCounter {
  fn default() -> Self {
    Self::new()
  }
}

impl AtomicCounter {
  pub fn new() -> AtomicCounter {
    AtomicCounter {
      count: AtomicI64::new(0),
    }
  }
}
impl Counter for AtomicCounter {
  fn add_and_get(&self, delta: i64) -> i64 {
    self
      .count
      .fetch_add(delta, std::sync::atomic::Ordering::Relaxed)
      + delta
  }
  fn get(&self) -> i64 {
    self.count.load(std::sync::atomic::Ordering::Relaxed)
  }
}

pub struct SerialCounter {
  count: i64,
}
impl Default for SerialCounter {
  fn default() -> Self {
    Self::new()
  }
}

impl SerialCounter {
  pub fn new() -> SerialCounter {
    SerialCounter { count: 0 }
  }
}
impl Counter for SerialCounter {
  fn add_and_get(&self, _delta: i64) -> i64 {
    unreachable!("")
  }

  fn add_and_get_mut(&mut self, delta: i64) -> Result<i64> {
    self.count += delta;
    Ok(self.count)
  }

  fn get(&self) -> i64 {
    self.count
  }
}

/// for multi-threaded scenarios
pub type SharedCounter = Arc<AtomicCounter>;

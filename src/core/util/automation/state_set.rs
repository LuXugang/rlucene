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
use crate::core::internal::hppc::bit_mixer::BitMixer;
#[cfg(test)]
use crate::core::util::automation::frozen_int_set::FrozenIntSet;
use crate::core::util::automation::int_set::IntSet;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// A thin wrapper mapping states to reference counts.
/// When a state's count drops to zero, it is removed.
#[derive(Clone)]
pub(crate) struct StateSet<A = Arc<Vec<i32>>> {
  inner: HashMap<i32, i32>,
  hash_code: i64,
  hash_updated: bool,
  array_updated: bool,
  array_cache: A,
}

impl StateSet {
  pub(crate) fn new(capacity: usize) -> Self {
    StateSet {
      inner: HashMap::with_capacity(capacity),
      hash_code: 0,
      hash_updated: true,
      array_updated: true,
      array_cache: Arc::new(Vec::new()),
    }
  }

  #[cfg(test)]
  pub(crate) fn freeze(&mut self, state: i32) -> FrozenIntSet {
    FrozenIntSet::new(self.get_array().clone(), self.long_hash_code(), state)
  }
}

impl<A> StateSet<A> {
  /// Add the state into this set, increasing its reference count by 1.
  pub(crate) fn incr(&mut self, state: i32) {
    let updated_value = self.inner.entry(state).and_modify(|v| *v += 1).or_insert(1);
    if *updated_value == 1 {
      self.key_changed()
    }
  }

  /// Decrease the reference count of the state.
  /// If it reaches 0, remove the state.
  pub(crate) fn decr(&mut self, state: i32) -> Result<()> {
    let std::collections::hash_map::Entry::Occupied(mut entry) = self.inner.entry(state) else {
      return Err(LuceneError::illegal_state(format!(
        "state {state} not found"
      )));
    };
    *entry.get_mut() -= 1;
    if *entry.get() == 0 {
      entry.remove();
      self.key_changed();
    }
    Ok(())
  }
  pub(crate) fn reset(&mut self) {
    self.inner.clear();
    self.key_changed();
  }

  fn key_changed(&mut self) {
    self.hash_updated = false;
    self.array_updated = false;
  }

  fn compute_hash_code(&mut self) -> i64 {
    if self.hash_updated {
      return self.hash_code;
    }

    let mut hash: i64 = self.inner.len() as i64;
    for &key in self.inner.keys() {
      hash = hash.wrapping_add(BitMixer::mix32(key as u32) as i64);
    }
    self.hash_code = hash;
    self.hash_updated = true;
    self.hash_code
  }
}

impl IntSet for StateSet {
  fn get_array(&mut self) -> &Arc<Vec<i32>> {
    if self.array_updated {
      return &self.array_cache;
    }

    if let Some(array) = Arc::get_mut(&mut self.array_cache) {
      if array.capacity() == self.inner.len() {
        array.clear();
        array.extend(self.inner.keys().copied());
      } else {
        *array = self.inner.keys().copied().collect();
      }
      array.sort_unstable();
    } else {
      let mut array: Vec<i32> = self.inner.keys().copied().collect();
      array.sort_unstable();
      self.array_cache = Arc::new(array);
    }
    self.array_updated = true;
    &self.array_cache
  }

  fn size(&self) -> usize {
    self.inner.len()
  }

  #[cfg(test)]
  fn long_hash_code(&mut self) -> i64 {
    self.compute_hash_code()
  }
}

impl StateSet<Vec<i32>> {
  /// Uses a reusable, exclusively owned array when frozen sets live in a separate arena.
  pub(crate) fn with_owned_array(capacity: usize) -> Self {
    Self {
      inner: HashMap::with_capacity(capacity),
      hash_code: 0,
      hash_updated: true,
      array_updated: true,
      array_cache: Vec::new(),
    }
  }

  pub(crate) fn size(&self) -> usize {
    self.inner.len()
  }

  pub(crate) fn long_hash_code(&mut self) -> i64 {
    self.compute_hash_code()
  }

  pub(crate) fn get_array(&mut self) -> &Vec<i32> {
    if !self.array_updated {
      self.array_cache.clear();
      self.array_cache.extend(self.inner.keys().copied());
      self.array_cache.sort_unstable();
      self.array_updated = true;
    }
    &self.array_cache
  }
}

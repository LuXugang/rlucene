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
use crate::core::util::automation::frozen_int_set::FrozenIntSet;
use crate::core::util::automation::int_set::IntSet;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// A thin wrapper mapping states to reference counts.
/// When a state's count drops to zero, it is removed.
#[derive(Clone)]
pub(crate) struct StateSet {
  inner: HashMap<i32, i32>,
  hash_code: i64,
  hash_updated: bool,
  array_updated: bool,
  array_cache: Arc<Vec<i32>>,
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

  /// Add the state into this set, increasing its reference count by 1.
  pub(crate) fn incr(&mut self, state: i32) {
    let updated_value = self.inner.entry(state).and_modify(|v| *v += 1).or_insert(1);
    if *updated_value == 1 {
      self.key_changed()
    }
  }

  /// Decrease the reference count of the state.
  /// If it reaches 0, remove the state.
  pub(crate) fn decr(&mut self, state: i32) {
    debug_assert!(self.inner.contains_key(&state));
    let entry = self.inner.get_mut(&state).expect("state must exist");
    *entry -= 1;
    if *entry == 0 {
      self.inner.remove(&state);
      self.key_changed();
    }
  }
  pub(crate) fn reset(&mut self) {
    self.inner.clear();
    self.key_changed();
  }

  pub(crate) fn freeze(&mut self, state: i32) -> FrozenIntSet {
    FrozenIntSet::new(self.get_array().clone(), self.long_hash_code(), state)
  }

  fn key_changed(&mut self) {
    self.hash_updated = false;
    self.array_updated = false;
  }
}

impl IntSet for StateSet {
  fn get_array(&mut self) -> &Arc<Vec<i32>> {
    if self.array_updated {
      return &self.array_cache;
    }

    let mut array: Vec<i32> = self.inner.keys().copied().collect();
    array.sort();

    self.array_cache = Arc::new(array);
    self.array_updated = true;
    &self.array_cache
  }

  fn size(&self) -> usize {
    self.inner.len()
  }

  fn long_hash_code(&mut self) -> i64 {
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

#[derive(Eq)]
pub(crate) struct StateSetHashKey {
  long_hash_code: i64,
  value: Arc<Vec<i32>>,
}
impl StateSetHashKey {
  pub(crate) fn new(long_hash_code: i64, value: Arc<Vec<i32>>) -> Self {
    StateSetHashKey {
      long_hash_code,
      value,
    }
  }
}
impl PartialEq for StateSetHashKey {
  fn eq(&self, other: &Self) -> bool {
    self.long_hash_code == other.long_hash_code && *self.value == *other.value
  }
}
impl Hash for StateSetHashKey {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.long_hash_code.hash(state);
  }
}

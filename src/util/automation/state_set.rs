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
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::internal::hppc::bit_mixer::BitMixer;
use crate::util::automation::frozen_int_set::FrozenIntSet;
use crate::util::automation::IntSet::IntSet;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;

/// A thin wrapper mapping states to reference counts.
/// When a state's count drops to zero, it is removed.
pub(crate) struct StateSet {
    inner: HashMap<i32, i32>,
    hash_code: i64,
    hash_updated: bool,
    array_updated: bool,
    array_cache: Rc<Vec<i32>>,
}

impl StateSet {
    pub(crate) fn new(capacity: usize) -> Self {
        StateSet {
            inner: HashMap::with_capacity(capacity),
            hash_code: 0,
            hash_updated: false,
            array_updated: false,
            array_cache: Rc::new(Vec::new()),
        }
    }

    /// Add the state into this set, increasing its reference count by 1.
    pub(crate) fn incr(&mut self, state: i32) {
        self.inner.insert(state, 1);
        self.key_changed()
    }

    /// Decrease the reference count of the state.
    /// If it reaches 0, remove the state.
    pub(crate) fn decr(&mut self, state: i32) -> Result<()> {
        debug_assert!(self.inner.contains_key(&state));
        match self.inner.get_mut(&state) {
            Some(entry) => {
                *entry -= 1;
                if *entry == 0 {
                    self.inner.remove(&state);
                    self.key_changed();
                }
                Ok(())
            },
            None => Err(LuceneError::illegal_state(format!(
                "State {} not found",
                state
            ))),
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
    fn get_array(&mut self) -> &Rc<Vec<i32>> {
        if self.array_updated {
            return &self.array_cache;
        }

        let mut array: Vec<i32> = self.inner.keys().copied().collect();
        array.sort_unstable();

        self.array_cache = Rc::new(array);
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
impl PartialEq for StateSet {
    fn eq(&self, other: &Self) -> bool {
        let this = self as *const _ as *mut Self;
        let other = other as *const _ as *mut Self;

        unsafe {
            let this_array = (*this).get_array();
            let other_array = (*other).get_array();
            let this_hash = (*this).long_hash_code();
            let other_hash = (*other).long_hash_code();

            this_hash == other_hash && **this_array == **other_array
        }
    }
}
impl Hash for StateSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let this = self as *const _ as *mut Self;

        unsafe {
            let hash_code = (*this).long_hash_code();
            hash_code.hash(state);
        }
    }
}

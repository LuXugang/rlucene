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
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::Arc;
/// Provide (read-and-write) striped locks for access to nodes of an
/// [`OnHeapHnswGraph`](crate::core::util::hnsw::on_heap_hnsw_graph::OnHeapHnswGraph).
/// Used by [`HnswConcurrentMergeBuilder`](crate::core::util::hnsw::hnsw_concurrent_merge_builder::HnswConcurrentMergeBuilder) and its `HnswGraphBuilders`.
#[derive(Clone)]
pub struct HnswLock {
  locks: Arc<Vec<RwLock<()>>>,
}

impl HnswLock {
  const NUM_LOCKS: usize = 512;
  pub fn new() -> Self {
    let mut locks = Vec::with_capacity(Self::NUM_LOCKS);
    for _ in 0..Self::NUM_LOCKS {
      locks.push(RwLock::new(()));
    }
    Self {
      locks: Arc::new(locks),
    }
  }

  fn hash(v1: usize, v2: usize) -> usize {
    v1.wrapping_mul(31).wrapping_add(v2)
  }

  pub fn read(&'_ self, level: usize, node: usize) -> RwLockReadGuard<'_, ()> {
    let lock_id = Self::hash(level, node) % Self::NUM_LOCKS;
    self.locks[lock_id].read()
  }

  pub fn write(&'_ self, level: usize, node: usize) -> RwLockWriteGuard<'_, ()> {
    let lock_id = Self::hash(level, node) % Self::NUM_LOCKS;
    self.locks[lock_id].write()
  }
}

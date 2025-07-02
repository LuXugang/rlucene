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
use std::rc::Rc;

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
/// Provide (read-and-write) striped locks for access to nodes of an
/// [`OnHeapHnswGraph`](crate::util::hnsw::on_heap_hnsw_graph::OnHeapHnswGraph).
/// Used by [`HnswConcurrentMergeBuilder`](crate::util::hnsw::hnsw_concurrent_merge_builder::HnswConcurrentMergeBuilder) and its `HnswGraphBuilders`.
pub(crate) struct HnswLock {
    locks: Rc<Vec<RwLock<()>>>,
}

impl HnswLock {
    const NUM_LOCKS: usize = 512;
    pub fn new() -> Self {
        let mut locks = Vec::with_capacity(Self::NUM_LOCKS);
        for _ in 0..Self::NUM_LOCKS {
            locks.push(RwLock::new(()));
        }
        Self {
            locks: Rc::new(locks),
        }
    }

    fn hash(v1: usize, v2: i32) -> usize {
        v1.wrapping_mul(31).wrapping_add(v2 as usize)
    }

    pub fn read(&self, level: usize, node: i32) -> RwLockReadGuard<()> {
        let lock_id = Self::hash(level, node) % Self::NUM_LOCKS;
        self.locks[lock_id].read()
    }

    pub fn write(&self, level: usize, node: i32) -> RwLockWriteGuard<()> {
        let lock_id = Self::hash(level, node) % Self::NUM_LOCKS;
        self.locks[lock_id].write()
    }
}

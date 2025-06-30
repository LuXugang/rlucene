/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
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

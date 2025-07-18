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
use crate::index::lockable_concurrent_approximate_priority_queue::Lock;
use crate::util::error::lucene_error::Result;
use std::collections::HashMap;
use std::vec::Vec;

/// An approximate priority queue, which attempts to poll items by decreasing
/// log of the weight, though exact ordering is not guaranteed. This struct
/// doesn't support null elements.
pub(crate) struct ApproximatePriorityQueue<T>
where
    T: Lock + IdentityId,
{
    /// Indexes between 0 and 63 are sparsely populated, and indexes that are
    /// greater than or equal to 64 are densely populated
    /// Items close to the beginning of this list are more likely to have a
    /// higher weight.
    pub(crate) slots: Vec<Option<T>>,
    /// A bitset where ones indicate that the corresponding index in `slots` is
    /// taken.
    used_slots: i64,
    pub(crate) map_to_idx: HashMap<String, usize>,
}
impl<T> ApproximatePriorityQueue<T>
where
    T: Lock + IdentityId,
{
    pub(crate) fn new() -> Self {
        let mut slots = Vec::with_capacity(i64::BITS as usize);
        slots.resize_with(i64::BITS as usize, || None);
        ApproximatePriorityQueue {
            slots,
            used_slots: 0,
            map_to_idx: HashMap::new(),
        }
    }
    /// Add an entry to this queue that has the provided weight.
    pub(crate) fn add(&mut self, entry: T, weight: i64) {
        // The expected slot of an item is the number of leading zeros of its
        // weight, ie. the larger the weight, the closer an item is to
        // the start of the array.
        let expected_slot = weight.leading_zeros() as usize;
        // If the slot is already taken, we look for the next one that is free.
        // The above bitwise operation is equivalent to looping over slots until
        // finding one that is free.
        let free_slots = !self.used_slots as u64;
        let offset = (free_slots >> expected_slot).trailing_zeros() as usize;
        let destination_slot = expected_slot + offset;

        if destination_slot < i64::BITS as usize {
            self.used_slots |= 1 << destination_slot;
            debug_assert!(self.slots[destination_slot].is_none());
            self.map_to_idx
                .insert(entry.id().to_string(), destination_slot);
            self.slots[destination_slot] = Some(entry);
            self.slots[destination_slot].as_mut().unwrap().unlock();
        } else {
            let len = self.slots.len();
            self.map_to_idx.insert(entry.id().to_string(), len);
            self.slots.push(Some(entry));
            self.slots[len].as_mut().unwrap().unlock();
        }
    }
    /// Return an entry matching the predicate. This will usually be one of the
    /// available entries that have the highest weight, though this is not
    /// guaranteed. This method returns {@code null} if no free entries are
    /// available.
    pub(crate) fn poll<F>(&mut self, predicate: F) -> Option<T>
    where
        F: Fn(&T) -> bool,
    {
        // Look at indexes 0..63 first, which are sparsely populated.
        let mut next_slot = 0;
        while next_slot < i64::BITS as usize {
            let next_used_slot =
                next_slot + (self.used_slots >> next_slot).trailing_zeros() as usize;
            if next_used_slot >= i64::BITS as usize {
                break;
            }
            if let Some(ref entry) = self.slots[next_used_slot] {
                if predicate(entry) {
                    self.used_slots &= !(1 << next_used_slot);
                    self.map_to_idx.remove(entry.id());
                    return self.slots[next_used_slot].take();
                } else {
                    next_slot = next_used_slot + 1;
                }
            }
        }
        // Then look at indexes 64.. which are densely populated.
        // Poll in descending order so that if the number of indexing threads
        // decreases, we keep using the same entry over and over again.
        // Resizing operations are also less costly on lists when items are
        // closer to the end of the list.
        for i in (i64::BITS as usize..self.slots.len()).rev() {
            if let Some(ref entry) = self.slots[i] {
                if predicate(entry) {
                    self.map_to_idx.remove(entry.id());
                    return self.slots.remove(i);
                }
            }
        }
        // No entry matching the predicate was found.
        None
    }
    // Only used for assertions
    pub(crate) fn contains(&self, o: &str) -> bool {
        self.map_to_idx.contains_key(o)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.used_slots == 0 && self.slots.len() == i64::BITS as usize
    }

    pub(crate) fn remove(&mut self, o: &str) -> Option<T> {
        match self.map_to_idx.get(o) {
            Some(&index) => {
                let t = if index < i64::BITS as usize {
                    self.used_slots &= !(1 << index);
                    self.slots[index].take().unwrap()
                } else {
                    self.slots.remove(index).unwrap()
                };
                self.map_to_idx.remove(o);
                Some(t)
            },
            None => None,
        }
    }
    pub(crate) fn get_idx(&self, id: &str) -> Option<usize> {
        self.map_to_idx.get(id).copied()
    }
    pub(crate) fn lock(&self, id: &str) -> Result<bool> {
        if let Some(&index) = self.map_to_idx.get(id) {
            self.slots[index].as_ref().unwrap().lock();
            return Ok(true);
        }
        Ok(false)
    }
    pub(crate) fn unlock(&self, id: &str) -> Result<bool> {
        if let Some(&index) = self.map_to_idx.get(id) {
            self.slots[index].as_ref().unwrap().unlock();
            return Ok(true);
        }
        Ok(false)
    }
}

pub(crate) trait IdentityId {
    fn id(&self) -> &str;
}
#[cfg(test)]
mod tests {
    use crate::index::approximate_priority_queue::{ApproximatePriorityQueue, IdentityId};
    use crate::index::lockable_concurrent_approximate_priority_queue::{FlushState, Lock};

    impl FlushState for i64 {}

    impl Lock for i64 {
        fn lock(&self) {
            unreachable!()
        }

        fn try_lock(&self) -> bool {
            unreachable!()
        }
        fn unlock(&self) {}

        fn is_locked(&self) -> bool {
            unreachable!()
        }
    }

    impl FlushState for u64 {}

    impl Lock for u64 {
        fn lock(&self) {
            unreachable!()
        }

        fn try_lock(&self) -> bool {
            unreachable!()
        }
        fn unlock(&self) {}

        fn is_locked(&self) -> bool {
            unreachable!()
        }
    }
    impl IdentityId for u64 {
        fn id(&self) -> &str {
            ""
        }
    }
    impl IdentityId for i64 {
        fn id(&self) -> &str {
            ""
        }
    }
    #[test]
    fn test_basics() {
        let mut pq = ApproximatePriorityQueue::<i64>::new();
        pq.add(8, 8);
        pq.add(32, 32);
        pq.add(0, 0);

        assert!(!pq.is_empty());
        assert_eq!(Some(32), pq.poll(|_| true));
        assert!(!pq.is_empty());
        assert_eq!(Some(8), pq.poll(|_| true));
        assert!(!pq.is_empty());
        assert_eq!(Some(0), pq.poll(|_| true));
        assert!(pq.is_empty());
        assert_eq!(None, pq.poll(|_| true));
    }
    #[test]
    fn test_poll_then_add() {
        let mut pq = ApproximatePriorityQueue::<u64>::new();
        pq.add(8, 8);
        assert_eq!(Some(8), pq.poll(|_| true));
        assert_eq!(None, pq.poll(|_| true));

        pq.add(0, 0);
        assert_eq!(Some(0), pq.poll(|_| true));
        assert_eq!(None, pq.poll(|_| true));

        pq.add(0, 0);
        assert_eq!(Some(0), pq.poll(|_| true));
        assert_eq!(None, pq.poll(|_| true));
    }

    #[test]
    fn test_collision() {
        let mut pq = ApproximatePriorityQueue::<u64>::new();
        pq.add(2, 2);
        pq.add(1, 1);
        pq.add(0, 0);
        pq.add(3, 3);

        assert!(!pq.is_empty());
        assert_eq!(Some(2), pq.poll(|_| true));
        assert!(!pq.is_empty());
        assert_eq!(Some(1), pq.poll(|_| true));
        assert!(!pq.is_empty());
        assert_eq!(Some(3), pq.poll(|_| true));
        assert!(!pq.is_empty());
        assert_eq!(Some(0), pq.poll(|_| true));
        assert!(pq.is_empty());
        assert_eq!(None, pq.poll(|_| true));
    }

    #[test]
    fn test_poll_with_predicate() {
        let mut pq = ApproximatePriorityQueue::<u64>::new();
        pq.add(8, 8);
        pq.add(32, 32);
        pq.add(0, 0);

        assert_eq!(Some(8), pq.poll(|x| *x == 8));
        assert_eq!(None, pq.poll(|x| *x == 8));
        assert!(!pq.is_empty());
    }

    #[test]
    fn test_collision_poll_with_predicate() {
        let mut pq = ApproximatePriorityQueue::<u64>::new();
        pq.add(2, 2);
        pq.add(1, 1);
        pq.add(0, 0);
        pq.add(3, 3);

        assert_eq!(Some(1), pq.poll(|x| *x % 2 == 1));
        assert_eq!(Some(3), pq.poll(|x| *x % 2 == 1));
        assert_eq!(None, pq.poll(|x| *x % 2 == 1));
        assert!(!pq.is_empty());
    }

    #[test]
    fn test_remove() {
        struct U64Wrapper {
            data: u64,
            id: String,
        }
        impl U64Wrapper {
            fn new(data: u64) -> Self {
                U64Wrapper {
                    data,
                    id: data.to_string(),
                }
            }
        }
        impl FlushState for U64Wrapper {}
        impl Lock for U64Wrapper {
            fn lock(&self) {
                unreachable!()
            }

            fn try_lock(&self) -> bool {
                unreachable!()
            }
            fn unlock(&self) {}

            fn is_locked(&self) -> bool {
                unreachable!()
            }
        }
        impl IdentityId for U64Wrapper {
            fn id(&self) -> &str {
                &self.id
            }
        }
        impl PartialEq for U64Wrapper {
            fn eq(&self, other: &Self) -> bool {
                self.data == other.data
            }
        }
        let mut pq = ApproximatePriorityQueue::<U64Wrapper>::new();
        pq.add(U64Wrapper::new(8), 8);
        pq.add(U64Wrapper::new(32), 32);
        pq.add(U64Wrapper::new(0), 0);

        assert!(pq.remove(&U64Wrapper::new(16).id).is_none());
        assert!(pq.remove(&U64Wrapper::new(9).id).is_none());
        assert!(pq.remove(&U64Wrapper::new(8).id).is_some());
        assert!(pq.remove(&U64Wrapper::new(0).id).is_some());
        assert!(pq.remove(&U64Wrapper::new(0).id).is_none());
        assert!(pq.remove(&U64Wrapper::new(32).id).is_some());
        assert!(pq.is_empty());
        assert!(pq.map_to_idx.is_empty());
    }
}

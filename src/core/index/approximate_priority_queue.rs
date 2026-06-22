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
use std::vec::Vec;

/// An approximate priority queue, which attempts to poll items by decreasing
/// log of the weight, though exact ordering is not guaranteed. This struct
/// doesn't support None elements.
pub(crate) struct ApproximatePriorityQueue<T>
where
  T: IdentityId,
{
  /// Indexes between 0 and 63 are sparsely populated, and indexes that are
  /// greater than or equal to 64 are densely populated
  /// Items close to the beginning of this list are more likely to have a
  /// higher weight.
  pub(crate) slots: Vec<Option<T>>,
  /// A bitset where ones indicate that the corresponding index in `slots` is
  /// taken.
  used_slots: i64,
}
impl<T> ApproximatePriorityQueue<T>
where
  T: IdentityId,
{
  pub(crate) fn new() -> Self {
    let mut slots = Vec::with_capacity(i64::BITS as usize);
    slots.resize_with(i64::BITS as usize, || None);
    ApproximatePriorityQueue {
      slots,
      used_slots: 0,
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
    let offset = free_slots
      .wrapping_shr(expected_slot as u32)
      .trailing_zeros() as usize;
    let destination_slot = expected_slot + offset;

    if destination_slot < i64::BITS as usize {
      self.used_slots |= 1 << destination_slot;
      debug_assert!(self.slots[destination_slot].is_none());
      self.slots[destination_slot] = Some(entry);
    } else {
      self.slots.push(Some(entry));
    }
  }
  /// Return an entry matching the predicate. This will usually be one of the
  /// available entries that have the highest weight, though this is not
  /// guaranteed. This method returns `None` if no free entries are
  /// available.
  pub(crate) fn poll<F>(&mut self, predicate: F) -> Option<T>
  where
    F: Fn(&T) -> bool,
  {
    // Look at indexes 0..63 first, which are sparsely populated.
    let mut next_slot = 0;
    while next_slot < i64::BITS as usize {
      let next_used_slot =
        next_slot + (self.used_slots as u64 >> next_slot).trailing_zeros() as usize;
      if next_used_slot >= i64::BITS as usize {
        break;
      }
      if let Some(ref entry) = self.slots[next_used_slot] {
        if predicate(entry) {
          self.used_slots &= !(1 << next_used_slot);
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
      if let Some(ref entry) = self.slots[i]
        && predicate(entry)
      {
        return self.slots.remove(i);
      }
    }
    // No entry matching the predicate was found.
    None
  }
  // Only used for assertions
  pub(crate) fn contains(&self, o: &str) -> bool {
    self
      .slots
      .iter()
      .any(|slot| slot.as_ref().is_some_and(|v| v.id() == o))
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.used_slots == 0 && self.slots.len() == i64::BITS as usize
  }

  pub(crate) fn remove(&mut self, o: &str) -> Option<T> {
    let index = self
      .slots
      .iter()
      .position(|slot| slot.as_ref().is_some_and(|v| v.id() == o))?;

    if index < i64::BITS as usize {
      self.used_slots &= !(1i64 << index);
      self.slots[index].take()
    } else {
      self.slots.remove(index)
    }
  }
}

pub(crate) trait IdentityId {
  fn id(&self) -> &str;
}

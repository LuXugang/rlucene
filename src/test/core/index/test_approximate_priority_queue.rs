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
use crate::core::index::approximate_priority_queue::{ApproximatePriorityQueue, IdentityId};
use crate::core::index::lockable_concurrent_approximate_priority_queue::Lock;

#[allow(dead_code)] // for quick search
struct TestApproximatePriorityQueue;
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
}

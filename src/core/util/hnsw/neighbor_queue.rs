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
use crate::core::util::error::lucene_error::Result;
use crate::core::util::long_heap::LongHeap;
use crate::core::util::numeric_utils::NumericUtils;
/// `NeighborQueue` uses a [`LongHeap`] to store lists of arcs in an HNSW graph,
/// represented as a neighbor node ID with an associated score packed together
/// as a sortable `i64`, which is sorted primarily by score.
///
/// The queue provides both fixed-size and unbounded operations via
/// [`insert_with_overflow(i32, f32)`](NeighborQueue::insert_with_overflow) and
/// [`add(i32, f32)`](NeighborQueue::add), and provides MIN and MAX heap
/// SubStruct.
pub struct NeighborQueue {
    pub heap: LongHeap,
    pub order: Order,
    // Used to track the number of neighbors visited during a single graph traversal
    pub visited_count: i32,
    // Whether the search stopped early because it reached the visited nodes limit
    pub incomplete: bool,
}
impl NeighborQueue {
    pub fn new(initial_size: usize, max_heap: bool) -> Result<Self> {
        Ok(NeighborQueue {
            heap: LongHeap::new(initial_size)?,
            order: if max_heap {
                Order::MaxHeap
            } else {
                Order::MinHeap
            },
            visited_count: 0,
            incomplete: false,
        })
    }
    /// return the number of elements in the heap
    pub fn size(&self) -> usize {
        self.heap.size()
    }
    /// Adds a new graph arc, extending the storage as needed.
    ///
    /// # Arguments
    ///
    /// * `new_node` - The neighbor node ID.
    /// * `new_score` - The score of the neighbor, relative to some other node.
    pub fn add(&mut self, new_node: usize, new_score: f32) {
        let encoded = self.encode(new_node, new_score);
        self.heap.push(encoded);
    }
    /// If the heap is not full (size is less than the `initial_size` provided
    /// to the constructor), adds a new node-and-score element. If the heap
    /// is full, compares the score against the current top score, and
    /// replaces the top element if `new_score` is better than (greater than
    /// unless the heap is reversed) the current top score.
    ///
    /// # Arguments
    ///
    /// * `new_node` - The neighbor node ID.
    /// * `new_score` - The score of the neighbor, relative to some other node.
    pub fn insert_with_overflow(&mut self, new_node: usize, new_score: f32) -> bool {
        let encoded = self.encode(new_node, new_score);
        self.heap.insert_with_overflow(encoded)
    }
    /// Encodes the node ID and its similarity score as a `u64`, preserving the
    /// Lucene tie-breaking rule: when two scores are equal, the smaller
    /// node ID must win.
    ///
    /// The most significant 32 bits represent the float score, encoded as a
    /// sortable `i32`.
    ///
    /// The less significant 32 bits represent the node ID, but the bits are
    /// complemented to ensure that smaller node IDs are preferred when
    /// scores are equal.
    ///
    /// The bitwise AND with `0xFFFF_FFFFu64` is necessary to extract a `u64`
    /// where:
    ///
    /// - The most significant 32 bits are zero
    /// - The least significant 32 bits represent the complemented node ID
    ///
    /// # Arguments
    ///
    /// * `node` - The node ID
    /// * `score` - The node's similarity score
    ///
    /// # Returns
    ///
    /// A 64-bit encoded representation combining score and node ID.
    fn encode(&self, node: usize, score: f32) -> i64 {
        debug_assert!(node <= i64::MAX as usize);
        let encoded_score = NumericUtils::float_to_sortable_int(score) as i64;
        let encoded_node = !(node as i64) & 0xFFFF_FFFF;
        self.order.apply((encoded_score << 32) | encoded_node)
    }

    fn decode_score(&self, heap_value: i64) -> f32 {
        let sortable = (self.order.apply(heap_value) >> 32) as i32;
        NumericUtils::sortable_int_to_float(sortable)
    }

    fn decode_node_id(&self, heap_value: i64) -> usize {
        let v = (!self.order.apply(heap_value)) as i32;
        v as usize
    }
    /// Removes the top element and returns its node id.
    pub fn pop(&mut self) -> Result<usize> {
        let v = self.heap.pop()?;
        Ok(self.decode_node_id(v))
    }

    pub fn nodes(&self) -> Vec<usize> {
        let size = self.size();
        let mut nodes = Vec::with_capacity(size);
        for i in 0..size {
            nodes.push(self.decode_node_id(self.heap.get(i + 1)));
        }
        nodes
    }

    /// Returns the top element's node id.
    pub fn top_node(&self) -> usize {
        self.decode_node_id(self.heap.top())
    }
    /// Returns the top element's node score.
    ///
    /// - For a min-heap, this is the minimum score.
    /// - For a max-heap, this is the maximum score.
    pub fn top_score(&self) -> f32 {
        self.decode_score(self.heap.top())
    }

    pub fn clear(&mut self) {
        self.heap.clear();
        self.visited_count = 0;
        self.incomplete = false;
    }

    pub fn visited_count(&self) -> i32 {
        self.visited_count
    }

    pub fn set_visited_count(&mut self, visited_count: i32) {
        self.visited_count = visited_count;
    }

    pub fn incomplete(&self) -> bool {
        self.incomplete
    }

    pub fn mark_incomplete(&mut self) {
        self.incomplete = true;
    }
}
impl std::fmt::Display for NeighborQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Neighbors[{}]", self.heap.size())
    }
}

pub enum Order {
    MinHeap,
    MaxHeap,
}

impl Order {
    pub fn apply(&self, v: i64) -> i64 {
        match self {
            Order::MinHeap => v,
            Order::MaxHeap => -1 - v,
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::RngExt;

    use crate::core::util::error::lucene_error::Result;
    use crate::core::util::hnsw::neighbor_queue::NeighborQueue;
    use crate::test::util::lucene_test_case::lucene_test_case_util::random;

    #[allow(dead_code)] // for quick search
    struct TestNeighborQueue;
    #[test]
    fn test_neighbors_product() -> Result<()> {
        let mut nn = NeighborQueue::new(2, false)?;

        assert!(nn.insert_with_overflow(2, 0.5));
        assert!(nn.insert_with_overflow(1, 0.2));
        assert!(nn.insert_with_overflow(3, 1.0));

        assert!((nn.top_score() - 0.5).abs() < f32::EPSILON);
        nn.pop()?;
        assert!((nn.top_score() - 1.0).abs() < f32::EPSILON);
        nn.pop()?;
        Ok(())
    }
    #[test]
    fn test_neighbors_max_heap() -> Result<()> {
        let mut nn = NeighborQueue::new(2, true)?;

        assert!(nn.insert_with_overflow(2, 2.0));
        assert!(nn.insert_with_overflow(1, 1.0));
        assert!(!nn.insert_with_overflow(3, 3.0));

        assert!((nn.top_score() - 2.0).abs() < f32::EPSILON);
        nn.pop()?;
        assert!((nn.top_score() - 1.0).abs() < f32::EPSILON);

        Ok(())
    }
    #[test]
    fn test_top_max_heap() -> Result<()> {
        let mut nn = NeighborQueue::new(2, true)?;

        nn.add(1, 2.0);
        nn.add(2, 1.0);

        assert!((nn.top_score() - 2.0).abs() < f32::EPSILON);
        assert_eq!(nn.top_node(), 1);

        Ok(())
    }
    #[test]
    fn test_top_min_heap() -> Result<()> {
        let mut nn = NeighborQueue::new(2, false)?;

        nn.add(1, 0.5);
        nn.add(2, -0.5);

        assert!((nn.top_score() + 0.5).abs() < f32::EPSILON);
        assert_eq!(nn.top_node(), 2);

        Ok(())
    }
    #[test]
    fn test_visited_count() -> Result<()> {
        let mut nn = NeighborQueue::new(2, false)?;

        nn.set_visited_count(100);
        assert_eq!(nn.visited_count(), 100);

        Ok(())
    }
    #[test]
    fn test_clear() -> Result<()> {
        let mut nn = NeighborQueue::new(2, false)?;

        nn.add(1, 1.1);
        nn.add(2, -2.2);
        nn.set_visited_count(42);
        nn.mark_incomplete();
        nn.clear();

        assert_eq!(nn.size(), 0);
        assert_eq!(nn.visited_count(), 0);
        assert!(!nn.incomplete());

        Ok(())
    }
    #[test]
    fn test_max_size_queue() -> Result<()> {
        let mut nn = NeighborQueue::new(2, false)?;

        nn.add(1, 1.0);
        nn.add(2, 2.0);
        assert_eq!(nn.size(), 2);
        assert_eq!(nn.top_node(), 1);
        // insertWithOverflow does not extend the queue
        assert!(nn.insert_with_overflow(3, 3.0));
        assert_eq!(nn.size(), 2);
        assert_eq!(nn.top_node(), 2);
        // add does extend the queue beyond maxSize
        nn.add(4, 1.0);
        assert_eq!(nn.size(), 3);

        Ok(())
    }

    #[test]
    fn test_unbounded_queue() -> Result<()> {
        let mut random = random();
        let mut nn = NeighborQueue::new(1, true)?;
        let mut max_score = -2.0f32;
        let mut max_node: Option<usize> = None;

        for i in 0..256 {
            // initial size is 32
            let score: f32 = random.random();
            if score > max_score {
                max_score = score;
                max_node = Some(i);
            }
            nn.add(i, score);
        }

        assert!((nn.top_score() - max_score).abs() < f32::EPSILON);
        assert_eq!(Some(nn.top_node()), max_node);

        Ok(())
    }
    #[test]
    fn test_invalid_arguments() {
        let result = NeighborQueue::new(0, false);
        assert!(result.is_err());
    }
    #[test]
    fn test_to_string() -> Result<()> {
        let nn = NeighborQueue::new(2, false)?;
        assert_eq!(nn.to_string(), "Neighbors[0]");
        Ok(())
    }
}

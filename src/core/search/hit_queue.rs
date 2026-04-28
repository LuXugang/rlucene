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
use crate::core::search::score_doc::{ScoreDoc, ScoreDocLike};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::priority_queue::{Compare, PriorityQueue};
/// Creates a new instance with `size` elements.
/// If `pre_populate` is set to `true`, the queue will pre-populate itself with sentinel objects
/// and set its [`size`] `size`.
/// In that case, you should not rely on [`size`] to get the number of actual elements
/// that were added to the queue, but keep track yourself.
///
/// **NOTE:** This struct pre-allocates a full array of length `size`.
///
/// # Parameters
/// - `size`: the requested size of this queue.
/// - `pre_populate`: specifies whether to pre-populate the queue with sentinel values.
pub struct HitQueue;
pub fn new(size: usize, pre_populate: bool) -> Result<PriorityQueue<ScoreDoc, HitQueueComparator>> {
  PriorityQueue::with_sentinel_object(
    size,
    || {
      if pre_populate {
        Some(ScoreDoc::new(i32::MAX, f32::NEG_INFINITY))
      } else {
        None
      }
    },
    HitQueueComparator,
  )
}
pub struct HitQueueComparator;
impl Compare<ScoreDoc> for HitQueueComparator {
  fn less_than(&self, hit_a: &ScoreDoc, hit_b: &ScoreDoc) -> Result<bool> {
    if hit_a.score() == hit_b.score() {
      Ok(hit_a.doc() > hit_b.doc())
    } else {
      Ok(hit_a.score() < hit_b.score())
    }
  }
}

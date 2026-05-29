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
use crate::core::search::knn::multi_leaf_knn_collector::MultiLeafKnnCollector;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::top_knn_collector::TopKnnCollector;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::blocking_float_heap::BlockingFloatHeap;

#[allow(dead_code)] // for quick search
struct TestMultiLeafKnnCollector;

/** Validates a fix for GH#13462 */
#[test]
fn test_global_score_coordination() -> Result<()> {
  let k = 7;
  let global_heap = BlockingFloatHeap::new(k);
  let mut collector1 =
    MultiLeafKnnCollector::new(k, &global_heap, TopKnnCollector::new(k, i32::MAX as usize)?)?;
  let mut collector2 =
    MultiLeafKnnCollector::new(k, &global_heap, TopKnnCollector::new(k, i32::MAX as usize)?)?;

  // Collect k (7) hits in collector1 with scores [100, 106]:
  for i in 0..k {
    collector1.collect(0, 100.0 + i as f32)?;
  }

  // The global heap should be updated since k hits were collected, and have a min score of
  // 100:
  assert_eq!(100.0, global_heap.peek());
  assert_eq!(100.0, collector1.min_competitive_similarity()?);

  // Collect k (7) hits in collector2 with only two that are competitive (200 and 300),
  // which also forces an update of the global heap with collector2's hits. This is a tricky
  // case where the heap will not be fully ordered, so it ensures global queue updates don't
  // incorrectly short-circuit (see GH#13462):
  collector2.collect(0, 10.0)?;
  collector2.collect(0, 11.0)?;
  collector2.collect(0, 12.0)?;
  collector2.collect(0, 13.0)?;
  collector2.collect(0, 200.0)?;
  collector2.collect(0, 14.0)?;
  collector2.collect(0, 300.0)?;

  // At this point, our global heap should contain [102, 103, 104, 105, 106, 200, 300] since
  // values 200 and 300 from collector2 should have pushed out 100 and 101 from collector1.
  // The min value on the global heap should be 102:
  assert_eq!(102.0, global_heap.peek());
  assert_eq!(102.0, collector2.min_competitive_similarity()?);
  Ok(())
}

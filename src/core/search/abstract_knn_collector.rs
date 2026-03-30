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
use crate::core::search::knn_collector::KnnCollector;
///  AbstractKnnCollector is the default implementation for a knn collector used
///  for gathering kNN results and providing topDocs from the gathered neighbors
pub trait AbstractKnnCollector: KnnCollector {
  fn num_collected(&self) -> usize;
  fn base(&self) -> &AbstractKnnCollectorBase;
  fn base_mut(&mut self) -> &mut AbstractKnnCollectorBase;

  fn early_terminated(&self) -> bool {
    self.base().visited_count >= self.base().visit_limit
  }

  fn inc_visited_count(&mut self, count: usize) {
    self.base_mut().visited_count += count;
  }

  fn visited_count(&self) -> usize {
    self.base().visited_count
  }

  fn visit_limit(&self) -> usize {
    self.base().visit_limit
  }

  fn k(&self) -> usize {
    self.base().k
  }
}

pub struct AbstractKnnCollectorBase {
  visited_count: usize,
  visit_limit: usize,
  k: usize,
}
impl AbstractKnnCollectorBase {
  pub fn new(k: usize, visit_limit: usize) -> Self {
    Self {
      visited_count: 0,
      visit_limit,
      k,
    }
  }
}

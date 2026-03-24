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
use crate::core::search::dummy::dummy_score_doc_like::DummyScoreDocLike;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::top_docs::TopDocs;
use crate::core::util::error::lucene_error::{LuceneError, Result};
///  AbstractKnnCollector is the default implementation for a knn collector used
///  for gathering kNN results and providing topDocs from the gathered neighbors
pub struct AbstractKnnCollector {
  visited_count: usize,
  visit_limit: usize,
  k: usize,
}

impl AbstractKnnCollector {
  pub fn new(k: usize, visit_limit: usize) -> Self {
    Self {
      visited_count: 0,
      visit_limit,
      k,
    }
  }
}
impl KnnCollector for AbstractKnnCollector {
  fn early_terminated(&self) -> bool {
    self.visited_count >= self.visit_limit
  }

  fn inc_visited_count(&mut self, count: usize) {
    self.visited_count += count;
  }

  fn visited_count(&self) -> usize {
    self.visited_count
  }

  fn visit_limit(&self) -> usize {
    self.visit_limit
  }

  fn k(&self) -> usize {
    self.k
  }

  fn collect(&mut self, _doc_id: usize, _similarity: f32) -> Result<bool> {
    Err(LuceneError::unreachable(""))
  }

  fn min_competitive_similarity(&self) -> Result<f32> {
    Err(LuceneError::unreachable(""))
  }

  type ScoreDocLike = DummyScoreDocLike;

  fn top_docs(&mut self) -> Result<TopDocs<Self::ScoreDocLike>> {
    Err(LuceneError::unreachable(""))
  }
}

pub trait AbstractKnnCollectorBase {
  fn num_collected(&self) -> usize;
}

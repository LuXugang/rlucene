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
use crate::core::search::top_docs::TopDocs;
use crate::core::search::total_hits::Relation::{EqualTo, GreaterThanOrEqualTo};
use crate::core::search::total_hits::TotalHits;
use crate::core::util::error::lucene_error::Result;
/// Wraps a provided KnnCollector object, translating the provided vectorId ordinal to a documentId
pub struct OrdinalTranslatedKnnCollector<'a, K, F>
where
  K: KnnCollector,
  F: Fn(usize) -> Result<usize>,
{
  in_: &'a mut K,
  vector_ordinal_to_doc_id: F,
}
impl<'a, K, F> OrdinalTranslatedKnnCollector<'a, K, F>
where
  K: KnnCollector,
  F: Fn(usize) -> Result<usize>,
{
  pub fn new(in_: &'a mut K, vector_ordinal_to_doc_id: F) -> Self {
    OrdinalTranslatedKnnCollector {
      in_,
      vector_ordinal_to_doc_id,
    }
  }
}
impl<K, F> KnnCollector for OrdinalTranslatedKnnCollector<'_, K, F>
where
  K: KnnCollector,
  F: Fn(usize) -> Result<usize>,
{
  fn early_terminated(&self) -> bool {
    self.in_.early_terminated()
  }

  fn inc_visited_count(&mut self, count: usize) {
    self.in_.inc_visited_count(count);
  }

  fn visited_count(&self) -> usize {
    self.in_.visited_count()
  }

  fn visit_limit(&self) -> usize {
    self.in_.visit_limit()
  }

  fn k(&self) -> usize {
    self.in_.k()
  }

  fn collect(&mut self, vector_id: usize, similarity: f32) -> Result<bool> {
    let v = (self.vector_ordinal_to_doc_id)(vector_id)?;
    self.in_.collect(v, similarity)
  }

  fn min_competitive_similarity(&self) -> Result<f32> {
    self.in_.min_competitive_similarity()
  }

  type ScoreDocLike = K::ScoreDocLike;

  fn top_docs(&mut self) -> Result<TopDocs<Self::ScoreDocLike>> {
    let td = self.in_.top_docs()?;
    let relation = if self.early_terminated() {
      GreaterThanOrEqualTo
    } else {
      EqualTo
    };
    let total_hits = TotalHits::new(self.visited_count(), relation);
    Ok(TopDocs::new(total_hits, td.score_docs))
  }
}

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
use crate::search::knn_collector::KnnCollector;
use crate::search::top_docs::TopDocs;
///  AbstractKnnCollector is the default implementation for a knn collector used
///  for gathering kNN results and providing topDocs from the gathered neighbors
pub struct AbstractKnnCollector<S>
where
    S: KnnCollector + AbstractKnnCollectorBase,
{
    visited_count: usize,
    visit_limit: usize,
    k: usize,
    sub: S,
}

impl<S> AbstractKnnCollector<S>
where
    S: KnnCollector + AbstractKnnCollectorBase,
{
    pub fn new(k: usize, visit_limit: usize, sub: S) -> Self {
        Self {
            visited_count: 0,
            visit_limit,
            k,
            sub,
        }
    }
}
impl<S> KnnCollector for AbstractKnnCollector<S>
where
    S: KnnCollector + AbstractKnnCollectorBase,
{
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

    fn collect(&mut self, doc_id: i32, similarity: f32) -> bool {
        self.sub.collect(doc_id, similarity)
    }

    fn min_competitive_similarity(&self) -> f32 {
        self.sub.min_competitive_similarity()
    }

    fn top_docs(self) -> TopDocs {
        self.sub.top_docs()
    }
}

pub trait AbstractKnnCollectorBase {
    fn num_collected(&self) -> usize;
}

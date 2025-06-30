/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use crate::search::knn_collector::KnnCollector;
use crate::search::top_docs::TopDocs;
use crate::util::error::lucene_error::Result;
///  AbstractKnnCollector is the default implementation for a knn collector used
///  for gathering kNN results and providing topDocs from the gathered neighbors
pub struct AbstractKnnCollector {
    visited_count: usize,
    visit_limit: usize,
    k: i32,
}

impl AbstractKnnCollector {
    pub fn new(k: i32, visit_limit: usize) -> Self {
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

    fn k(&self) -> i32 {
        self.k
    }

    fn collect(&mut self, _doc_id: i32, _similarity: f32) -> bool {
        unimplemented!()
    }

    fn min_competitive_similarity(&self) -> f32 {
        unimplemented!()
    }

    fn top_docs(&mut self) -> Result<TopDocs> {
        unimplemented!()
    }
}

pub trait AbstractKnnCollectorBase {
    fn num_collected(&self) -> usize;
}

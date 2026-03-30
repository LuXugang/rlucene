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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::blocking_float_heap::BlockingFloatHeap;

pub struct TopKnnCollectorManager {
  k: usize,
  global_score_queue: Option<BlockingFloatHeap>,
}
impl TopKnnCollectorManager {
  pub fn new<IRC>(k: usize, index_searcher: &IndexSearcher<IRC>) -> Result<TopKnnCollectorManager>
  where
    IRC: IndexReaderContext,
  {
    let is_multi_segments = index_searcher.get_leaf_contexts()?.len() > 1;
    let global_score_queue = if is_multi_segments {
      Some(BlockingFloatHeap::new(k))
    } else {
      None
    };
    Ok(Self {
      k,
      global_score_queue,
    })
  }
}

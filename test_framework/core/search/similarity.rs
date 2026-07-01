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
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::explanation::Explanation;
use crate::core::search::similarities_impl::classic_similarity::idf_explain;
use crate::core::search::similarities_impl::tf_idf_similarity::{
  TFIDFSimilarity, TFIDFSimilarityBase, TFIDFSubEnum,
};
use crate::core::search::term_statistics::TermStatistics;

#[derive(Clone, Default)]
pub struct SimpleSimilarity1;

pub fn new_simple_similarity1() -> TFIDFSimilarity {
  let v = TFIDFSubEnum::Simple1(SimpleSimilarity1);
  TFIDFSimilarity::new(v)
}

impl TFIDFSimilarityBase for SimpleSimilarity1 {
  fn tf(&self, freq: f32) -> f32 {
    freq
  }

  fn idf_explain(
    &self,
    _collection_stats: &CollectionStatistics,
    _term_stats: &TermStatistics,
  ) -> Explanation {
    Explanation::match_(1.0f32, "Inexplicable".to_string(), vec![])
  }

  fn idf(&self, _doc_freq: i64, _doc_count: i64) -> f32 {
    1.0f32
  }

  fn length_norm(&self, _length: i32) -> f32 {
    1.0f32
  }
}

#[derive(Clone)]
pub struct SimpleSimilarity;

pub fn new_simple_similarity() -> TFIDFSimilarity {
  let v = TFIDFSubEnum::Simple(SimpleSimilarity);
  TFIDFSimilarity::new(v)
}

impl TFIDFSimilarityBase for SimpleSimilarity {
  fn tf(&self, freq: f32) -> f32 {
    freq
  }

  fn idf_explain(
    &self,
    collection_stats: &CollectionStatistics,
    term_stats: &TermStatistics,
  ) -> Explanation {
    idf_explain(self, collection_stats, term_stats)
  }

  fn idf_explain_from_multi_ts(
    &self,
    _collection_stats: &CollectionStatistics,
    _term_stats: &[TermStatistics],
  ) -> Explanation {
    Explanation::match_no_details(1.0f32, "Inexplicable")
  }

  fn idf(&self, _doc_freq: i64, _doc_count: i64) -> f32 {
    1f32
  }

  fn length_norm(&self, _length: i32) -> f32 {
    1f32
  }
}

#[derive(Clone, Default)]
pub struct TestSimilarity;

pub fn new_test_similarity() -> TFIDFSimilarity {
  let v = TFIDFSubEnum::Test(TestSimilarity);
  TFIDFSimilarity::new(v)
}

impl TFIDFSimilarityBase for TestSimilarity {
  fn tf(&self, freq: f32) -> f32 {
    if freq > 0.0 { 1.0 } else { 0.0 }
  }

  fn idf_explain(
    &self,
    collection_stats: &CollectionStatistics,
    term_stats: &TermStatistics,
  ) -> Explanation {
    idf_explain(self, collection_stats, term_stats)
  }

  fn idf(&self, _doc_freq: i64, _doc_count: i64) -> f32 {
    1f32
  }

  fn length_norm(&self, _length: i32) -> f32 {
    1f32
  }
}

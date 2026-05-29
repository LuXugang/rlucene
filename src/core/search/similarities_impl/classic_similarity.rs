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
use crate::core::search::similarities_impl::tf_idf_similarity::{
  TFIDFSimilarity, TFIDFSimilarityBase, TFIDFSubEnum,
};
use crate::core::search::term_statistics::TermStatistics;
/// Expert: Historical scoring implementation. You might want to consider using
/// [`BM25Similarity`](crate::core::search::similarities_impl::bm25_similarity::BM25Similarity) instead, which is generally considered superior to TF-IDF.
#[derive(Clone)]
pub struct ClassicSimilarity;
pub fn new() -> TFIDFSimilarity {
  let v = TFIDFSubEnum::Classic(ClassicSimilarity);
  TFIDFSimilarity::new(v)
}
pub fn with_discount_overlaps(discount_overlaps: bool) -> TFIDFSimilarity {
  let v = TFIDFSubEnum::Classic(ClassicSimilarity);
  TFIDFSimilarity::with_discount_overlaps(v, discount_overlaps)
}
impl TFIDFSimilarityBase for ClassicSimilarity {
  fn tf(&self, freq: f32) -> f32 {
    freq.sqrt()
  }

  fn idf_explain(
    &self,
    collection_stats: &CollectionStatistics,
    term_stats: &TermStatistics,
  ) -> Explanation {
    idf_explain(self, collection_stats, term_stats)
  }

  fn idf(&self, doc_freq: i64, doc_count: i64) -> f32 {
    (((doc_count + 1) as f64 / (doc_freq + 1) as f64).ln() + 1.0) as f32
  }

  fn length_norm(&self, num_terms: i32) -> f32 {
    (1.0f64 / (num_terms as f64).sqrt()) as f32
  }
}
pub fn idf_explain<T>(
  s: &T,
  collection_stats: &CollectionStatistics,
  term_stats: &TermStatistics,
) -> Explanation
where
  T: TFIDFSimilarityBase,
{
  let df = term_stats.get_doc_freq();
  let doc_count = collection_stats.get_doc_count();
  let idf = s.idf(df, doc_count);

  Explanation::match_(
    idf,
    "idf, computed as log((docCount+1)/(docFreq+1)) + 1 from:".to_string(),
    vec![
      Explanation::match_(
        df,
        "docFreq, number of documents containing term".to_string(),
        vec![],
      ),
      Explanation::match_(
        doc_count,
        "docCount, total number of documents with field".to_string(),
        vec![],
      ),
    ],
  )
}

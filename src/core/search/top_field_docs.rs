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
use crate::core::search::field_value_hit_queue::TopFieldScoreDoc;
use crate::core::search::sort_field_enum::SortFieldEnum;
use crate::core::search::top_docs::{TopDocs, TopDocsLike};
use crate::core::search::total_hits::TotalHits;

/// Represents sorted hits returned by an `IndexSearcher`.
#[derive(Default)]
pub struct TopFieldDocs {
  pub base: TopDocs<TopFieldScoreDoc>,
  /// The fields which were used to sort results by.
  pub fields: Vec<SortFieldEnum>,
}
impl Default for TopDocs<TopFieldScoreDoc> {
  fn default() -> Self {
    Self {
      total_hits: TotalHits::default(),
      score_docs: Vec::new(),
    }
  }
}
#[cfg(test)]
impl Clone for TopFieldDocs {
  fn clone(&self) -> Self {
    Self {
      base: self.base.clone(),
      fields: self.fields.clone(),
    }
  }
}
impl TopFieldDocs {
  /// Creates one of these objects.
  ///
  /// # Parameters
  /// - `total_hits`: Total number of hits for the query.
  /// - `score_docs`: The top hits for the query.
  /// - `fields`: The sort criteria used to find the top hits.
  pub fn new(
    total_hits: TotalHits,
    score_docs: Vec<TopFieldScoreDoc>,
    fields: Vec<SortFieldEnum>,
  ) -> Self {
    let base = TopDocs::new(total_hits, score_docs);
    Self { base, fields }
  }
}

impl TopDocsLike for TopFieldDocs {
  fn total_hits(&self) -> &TotalHits {
    &self.base.total_hits
  }

  type ScoreDocLike = TopFieldScoreDoc;

  fn score_docs(&self) -> &[Self::ScoreDocLike] {
    &self.base.score_docs
  }

  fn score_docs_mut(&mut self) -> &mut [Self::ScoreDocLike] {
    &mut self.base.score_docs
  }

  fn take_score_docs(&mut self) -> Vec<Self::ScoreDocLike> {
    std::mem::take(&mut self.base.score_docs)
  }
}

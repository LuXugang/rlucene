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
use crate::core::search::similarities_impl::similarities::{SimScorer, Similarity};
use crate::core::search::term_statistics::TermStatistics;
use std::fmt::{Display, Formatter};

/// Similarity that returns the raw TF as score.
#[derive(Debug, Clone)]
pub struct RawTFSimilarity {
    discount_overlaps: bool,
}

/// Default constructor: parameter-free
impl Default for RawTFSimilarity {
    fn default() -> Self {
        Self {
            discount_overlaps: true,
        }
    }
}

impl RawTFSimilarity {
    /// Primary constructor
    pub fn with_discount_overlaps(discount_overlaps: bool) -> Self {
        Self { discount_overlaps }
    }
}

impl Display for RawTFSimilarity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RawTFSimilarity")
    }
}

impl Similarity for RawTFSimilarity {
    type SimScorer = RawTFSimScorer;

    fn scorer(
        &self,
        boost: f32,
        _collection_stats: &CollectionStatistics,
        _term_stats: &[TermStatistics],
    ) -> Self::SimScorer {
        RawTFSimScorer { boost }
    }
}

#[derive(Debug, Clone)]
pub struct RawTFSimScorer {
    boost: f32,
}

impl SimScorer for RawTFSimScorer {
    fn score(&self, freq: f32, _norm: i64) -> f32 {
        self.boost * freq
    }
}

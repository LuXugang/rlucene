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
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::Scorer;

pub struct WANDScorer<S>
where
    S: Scorer,
{
    /// scalingFactor in Lucene
    scaling_factor: i32,
    /// scaled min competitive score
    min_competitive_score: i64,
    all_scorers: Vec<S>,
    /// list of scorers which 'lead' the iteration and are currently
    /// positioned on 'doc'. This is sometimes called the 'pivot' in
    /// some descriptions of WAND (Weak AND).
    pub(crate) lead: Option<usize>,
    /// current doc ID of the leads
    pub(crate) doc: i32,
    /// score of the leads
    pub(crate) lead_score: f64,
    /// priority queue of scorers that are too advanced compared to the current
    /// doc. Ordered by doc ID.
    pub(crate) head: Option<usize>,
    /// priority queue of scorers which are behind the current doc.
    /// Ordered by maxScore.
    pub(crate) tail: Vec<usize>,
    /// sum of max scores of scorers in tail
    pub(crate) tail_max_score: i64,
    pub(crate) tail_size: i32,
    /// cost from Lucene
    pub(crate) cost: i64,
    /// upper bound for which max scores are valid
    pub(crate) up_to: i32,
    pub(crate) min_should_match: i32,
    pub(crate) freq: i32,
    pub(crate) score_mode: ScoreMode,
    pub(crate) lead_cost: i64,
}

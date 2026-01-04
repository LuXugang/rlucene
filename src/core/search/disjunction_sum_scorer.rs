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
use crate::core::search::disi_wrapper::DisiWrapper;
use crate::core::search::disjunction_scorer::DisjunctionScorerBase;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::scorer::Scorer;
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::math_util::MathUtil;

/// A Scorer for OR like queries, counterpart of [`ConjunctionScorer`](crate::core::search::conjunction_scorer::ConjunctionScorer).
#[derive(Default)]
pub struct DisjunctionSumScorer;
impl DisjunctionScorerBase for DisjunctionSumScorer {
    fn score<S>(&self, disi_wrapper: &mut [DisiWrapper<S>], top_list: Option<usize>) -> Result<f32>
    where
        S: Scorer,
    {
        let mut score: f64 = 0.0;
        let mut w = match top_list {
            Some(idx) => &mut disi_wrapper[idx],
            None => return Ok(score as f32),
        };
        loop {
            let sub_score = w.scorer.score()? as f64;
            score += sub_score;

            match w.next {
                Some(idx) => {
                    w = &mut disi_wrapper[idx];
                },
                None => break,
            }
        }

        Ok(score as f32)
    }

    fn advance_shallow<S>(
        &mut self,
        target: i32,
        disi_wrapper: &mut [DisiWrapper<S>],
    ) -> Result<i32>
    where
        S: Scorer,
    {
        let mut min = NO_MORE_DOCS;

        for w in disi_wrapper.iter_mut() {
            if w.scorer.doc_id()? <= target {
                min = std::cmp::min(min, w.scorer.advance_shallow(target)?);
            }
        }

        Ok(min)
    }

    fn get_max_score<S>(&mut self, up_to: i32, disi_wrapper: &mut [DisiWrapper<S>]) -> Result<f32>
    where
        S: Scorer,
    {
        let mut sum: f64 = 0.0;

        for w in disi_wrapper.iter_mut() {
            if w.scorer.doc_id()? <= up_to {
                let v = w.scorer.get_max_score(up_to)? as f64;
                sum += v;
            }
        }

        let result = MathUtil::sum_upper_bound(sum, disi_wrapper.len().try_convert()?);

        Ok(result as f32)
    }

    fn set_min_competitive_score<S>(
        &mut self,
        _min_score: f32,
        _disi_wrapper: &mut [DisiWrapper<S>],
    ) -> Result<()>
    where
        S: Scorer,
    {
        Ok(())
    }
}

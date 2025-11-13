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
use crate::core::search::query::Query;
use crate::core::search::score_doc::ScoreDocLike;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;

pub struct CheckHits;
impl CheckHits {
    pub fn check_equal<S>(query: &Query, hits1: &[S], hits2: &[S]) -> Result<()>
    where
        S: ScoreDocLike,
    {
        const SCORE_TOLERANCE: f32 = 1.0e-6;

        if hits1.len() != hits2.len() {
            return Err(LuceneError::illegal_argument(format!(
                "Unequal lengths: hits1={}, hits2={}",
                hits1.len(),
                hits2.len()
            )));
        }

        for (i, (h1, h2)) in hits1.iter().zip(hits2.iter()).enumerate() {
            if h1.doc() != h2.doc() {
                return Err(LuceneError::illegal_argument(format!(
                    "Hit {i} docnumbers don't match\nhits1={:?}\nhits2={:?}\nfor query: {:?}",
                    hits1, hits2, query
                )));
            }

            if (h1.doc() != h2.doc()) || (h1.score() - h2.score()).abs() > SCORE_TOLERANCE {
                return Err(LuceneError::illegal_argument(format!(
                    "Hit {i}, doc nrs {} and {}\nunequal: {}\nand: {}\nfor query: {:?}",
                    h1.doc(),
                    h2.doc(),
                    h1.score(),
                    h2.score(),
                    query
                )));
            }
        }

        Ok(())
    }
}

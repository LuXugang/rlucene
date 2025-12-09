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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::scorer::Scorer;
use crate::core::util::error::lucene_error::Result;

pub struct ReqOptSumScorer;

struct DocIdSetIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    upto: i32,
    max_score: f32,
    opt_is_required: bool,
    min_score: f32,
    req_scorer: S1,
    opt_scorer: S2,
    req_max_score: f32,
}
impl<S1, S2> DocIdSetIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    fn move_to_next_block(&mut self, target: i32) -> Result<()> {
        self.upto = self.advance_shallow(target)?;
        let req_max_score_block = self.req_scorer.get_max_score(self.upto)?;
        self.max_score = self.get_max_score(self.upto)?;
        self.opt_is_required = req_max_score_block < self.min_score;
        Ok(())
    }
    fn advance_shallow(&mut self, target: i32) -> Result<i32> {
        let mut up_to = self.req_scorer.advance_shallow(target)?;

        let opt_doc = {
            let it = self.opt_scorer.iterator();
            it.doc_id()
        };

        if opt_doc <= target {
            let v = self.opt_scorer.advance_shallow(target)?;
            up_to = up_to.min(v);
        } else if opt_doc != NO_MORE_DOCS {
            up_to = up_to.min(opt_doc - 1);
        }

        Ok(up_to)
    }
    fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
        let mut max_score = self.req_scorer.get_max_score(up_to)?;

        let opt_doc = {
            let it = self.opt_scorer.iterator();
            it.doc_id()
        };

        if opt_doc <= up_to {
            max_score += self.opt_scorer.get_max_score(up_to)?;
        }

        Ok(max_score)
    }
    fn advance_impacts(&mut self, mut target: i32) -> Result<i32> {
        if target > self.upto {
            self.move_to_next_block(target)?;
        }

        loop {
            if self.max_score >= self.min_score {
                return Ok(target);
            }

            if self.upto == NO_MORE_DOCS {
                return Ok(NO_MORE_DOCS);
            }

            target = self.upto + 1;

            self.move_to_next_block(target)?;
        }
    }
    fn advance_internal(&mut self, target: i32) -> Result<i32> {
        if target == NO_MORE_DOCS {
            self.req_scorer.iterator_mut().advance(target)?;
            return Ok(NO_MORE_DOCS);
        }

        let mut req_doc = target;

        'advance_head: loop {
            if self.min_score != 0.0 {
                req_doc = self.advance_impacts(req_doc)?;
            }

            {
                let mut req_it = self.req_scorer.iterator_mut();
                if req_it.doc_id() < req_doc {
                    req_doc = req_it.advance(req_doc)?;
                }
            }

            if req_doc == NO_MORE_DOCS || !self.opt_is_required {
                return Ok(req_doc);
            }

            let upper_bound = if self.req_max_score < self.min_score {
                NO_MORE_DOCS
            } else {
                self.upto
            };

            if req_doc > upper_bound {
                continue;
            }
            // Find the next common doc within the current block
            let mut opt_it = self.opt_scorer.iterator_mut();
            let mut req_it = self.req_scorer.iterator_mut();
            loop {
                let mut opt_doc = opt_it.doc_id();

                if opt_doc < req_doc {
                    opt_doc = opt_it.advance(req_doc)?;
                }

                if opt_doc > upper_bound {
                    req_doc = upper_bound + 1;
                    continue 'advance_head;
                }

                if opt_doc != req_doc {
                    req_doc = req_it.advance(opt_doc)?;
                    if req_doc > upper_bound {
                        continue 'advance_head;
                    }
                }

                if req_doc == NO_MORE_DOCS || opt_doc == req_doc {
                    return Ok(req_doc);
                }
            }
        }
    }
}
impl<S1, S2> DocIdSetIterator for DocIdSetIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    fn doc_id(&self) -> i32 {
        self.req_scorer.iterator().doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let next = self.req_scorer.iterator().doc_id() + 1;
        self.advance_internal(next)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.advance_internal(target)
    }

    fn cost(&self) -> Result<i64> {
        self.req_scorer.iterator().cost()
    }
}

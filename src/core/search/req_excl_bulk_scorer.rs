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
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorer::Scorer;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;

pub(crate) struct ReqExclBulkScorer<BS, DISI, TPI>
where
    BS: BulkScorer,
    DISI: DocIdSetIterator,
    TPI: TwoPhaseIterator,
{
    req: BS,
    excl_two_phase: Option<TPI>,
    excl_approximation: Option<DISI>,
}
impl<BS, DISI, TPI> ReqExclBulkScorer<BS, DISI, TPI>
where
    BS: BulkScorer,
    DISI: DocIdSetIterator,
    TPI: TwoPhaseIterator,
{
    pub(crate) fn with_scorer<S>(req: BS, excl: S) -> Self
    where
        S: Scorer<TwoPhaseIter = TPI, DocIdSetIterator = DISI>,
    {
        match excl.has_two_phase_iterator() {
            true => Self {
                req,
                excl_two_phase: Some(excl.take_two_phase_iterator().unwrap()),
                excl_approximation: None,
            },
            false => Self {
                req,
                excl_two_phase: None,
                excl_approximation: Some(excl.take_iterator()),
            },
        }
    }
    pub(crate) fn with_disi(req: BS, disi: DISI) -> Self {
        Self {
            req,
            excl_two_phase: None,
            excl_approximation: Some(disi),
        }
    }
    pub(crate) fn with_two_phase(req: BS, two_phase: TPI) -> Self {
        Self {
            req,
            excl_two_phase: Some(two_phase),
            excl_approximation: None,
        }
    }
}
impl<BS, DISI, TPI> BulkScorer for ReqExclBulkScorer<BS, DISI, TPI>
where
    BS: BulkScorer,
    DISI: DocIdSetIterator,
    TPI: TwoPhaseIterator,
{
    fn score<LC, B>(
        &mut self,
        collector: &mut LC,
        accept_docs: Option<&B>,
        min: i32,
        max: i32,
    ) -> Result<i32>
    where
        LC: LeafCollector,
        B: Bits,
    {
        let mut upto = min;

        let mut excl_doc = match self.excl_approximation {
            Some(ref approx) => approx.doc_id(),
            None => self
                .excl_two_phase
                .as_mut()
                .unwrap()
                .approximation_mut()
                .doc_id(),
        };

        while upto < max {
            if excl_doc < upto {
                excl_doc = match self.excl_approximation {
                    Some(ref mut approx) => approx.advance(upto)?,
                    None => self
                        .excl_two_phase
                        .as_mut()
                        .unwrap()
                        .approximation_mut()
                        .advance(upto)?,
                };
            }

            if excl_doc == upto {
                let excluded = match &mut self.excl_two_phase {
                    None => true,
                    Some(tpi) => tpi.matches()?,
                };

                if excluded {
                    upto += 1;
                }
                excl_doc = match self.excl_approximation {
                    Some(ref mut approx) => approx.next_doc()?,
                    None => self
                        .excl_two_phase
                        .as_mut()
                        .unwrap()
                        .approximation_mut()
                        .next_doc()?,
                };
            } else {
                let limit = excl_doc.min(max);
                upto = self.req.score(collector, accept_docs, upto, limit)?;
            }
        }
        if upto == max {
            upto = self.req.score(collector, accept_docs, upto, upto)?;
        }

        Ok(upto)
    }

    fn cost(&mut self) -> Result<i64> {
        self.req.cost()
    }
}

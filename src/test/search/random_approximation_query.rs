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
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::util::lucene_test_case::lucene_test_case_util::random_from_seed;
use rand::Rng;
use rand::prelude::StdRng;

pub struct RandomApproximationQuery;

pub struct RandomTwoPhaseView<DISI>
where
    DISI: DocIdSetIterator,
{
    approximation: RandomApproximation<StdRng, DISI>,
    last_doc: i32,
    random_match_cost: f32,
}
impl<DISI> RandomTwoPhaseView<DISI>
where
    DISI: DocIdSetIterator,
{
    pub fn new<R: Rng + ?Sized>(random: &mut R, disi: DISI) -> Self {
        let seed = random.random();
        let new_random = random_from_seed(seed);
        let random_approximation = RandomApproximation::new(new_random, disi);
        Self {
            approximation: random_approximation,
            last_doc: -1,
            random_match_cost: random.random::<f32>() * 200f32,
        }
    }
}
impl<DISI> TwoPhaseIterator for RandomTwoPhaseView<DISI>
where
    DISI: DocIdSetIterator,
{
    fn approximation_mut(&mut self) -> Result<Box<dyn DocIdSetIterator + '_>> {
        Ok(Box::new(&mut self.approximation))
    }

    fn approximation(&self) -> Result<Box<dyn DocIdSetIterator + '_>> {
        Ok(Box::new(&self.approximation))
    }

    fn matches(&mut self) -> Result<bool> {
        let approx_doc = self.approximation.doc_id();

        if approx_doc == -1 || approx_doc == NO_MORE_DOCS {
            return Err(LuceneError::illegal_state(format!(
                "matches() should not be called on doc ID {}",
                approx_doc
            )));
        }

        if self.last_doc == approx_doc {
            return Err(LuceneError::illegal_state(format!(
                "matches() has been called twice on doc ID {}",
                approx_doc
            )));
        }
        self.last_doc = approx_doc;
        Ok(approx_doc == self.approximation.disi.doc_id())
    }

    fn match_cost(&self) -> f32 {
        self.random_match_cost
    }
}
pub struct RandomApproximation<RNG, DISI>
where
    RNG: Rng,
    DISI: DocIdSetIterator,
{
    random: RNG,
    disi: DISI,
    doc: i32,
}

impl<RNG, DISI> RandomApproximation<RNG, DISI>
where
    RNG: Rng,
    DISI: DocIdSetIterator,
{
    pub fn new(random: RNG, disi: DISI) -> Self {
        Self {
            random,
            disi,
            doc: -1,
        }
    }
}

impl<RNG, DISI> DocIdSetIterator for RandomApproximation<RNG, DISI>
where
    RNG: Rng,
    DISI: DocIdSetIterator,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc + 1)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        if self.disi.doc_id() < target {
            self.disi.advance(target)?;
        }
        let disi_doc = self.disi.doc_id();
        if disi_doc == NO_MORE_DOCS {
            self.doc = NO_MORE_DOCS;
            return Ok(self.doc);
        }

        let picked = self.random.random_range(target..=disi_doc);
        self.doc = picked;
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        self.disi.cost()
    }
}

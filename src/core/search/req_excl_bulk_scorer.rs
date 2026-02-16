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
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;

pub struct ReqExclBulkScorer<BS>
where
    BS: BulkScorer,
{
    req: BS,
    excl_two_phase: Option<Box<dyn TwoPhaseIterator>>,
    excl_approximation: Option<Box<dyn DocIdSetIterator>>,
}
impl<BS> ReqExclBulkScorer<BS>
where
    BS: BulkScorer,
{
    pub(crate) fn from_scorer<S>(req: BS, excl: S) -> Result<Self>
    where
        S: Scorer,
    {
        Ok(match excl.has_two_phase_iterator() == TwoPhaseState::Yes {
            true => Self {
                req,
                excl_two_phase: Some(Box::new(excl).take_two_phase_iterator().unwrap()),
                excl_approximation: None,
            },
            false => Self {
                req,
                excl_two_phase: None,
                excl_approximation: Some(Box::new(excl).take_iterator()),
            },
        })
    }
}
impl<BS> BulkScorer for ReqExclBulkScorer<BS>
where
    BS: BulkScorer,
{
    fn score(
        &mut self,
        collector: &mut dyn LeafCollector,
        accept_docs: Option<&dyn Bits>,
        min: i32,
        max: i32,
    ) -> Result<i32> {
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
#[cfg(test)]
mod tests {
    use crate::core::search::bulk_scorer::BulkScorer;
    use crate::core::search::doc_id_set::DocIdSet;
    use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;

    use crate::core::search::dummy::dummy_scorer::DummyScorer;
    use crate::core::search::leaf_collector::LeafCollector;
    use crate::core::search::req_excl_bulk_scorer::ReqExclBulkScorer;
    use crate::core::search::scorable::Scorable;
    use crate::core::util::bit_set::BitSet;
    use crate::core::util::bits::Bits;
    use crate::core::util::doc_id_set_builder::DocIdSetBuilder;

    use crate::core::search::scorer::{Scorer, TwoPhaseState};
    use crate::core::search::two_phase_iterator::TwoPhaseIterator;
    use crate::core::util::error::lucene_error::Result;
    use crate::core::util::fixed_bit_set::FixedBitSet;
    use crate::test::search::random_approximation_query::RandomTwoPhaseView;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        at_least, random, random_from_seed,
    };
    use rand::Rng;
    use std::fmt::{Display, Formatter};

    #[allow(dead_code)] // for quick search
    struct TestReqExclBulkScorer;
    #[test]
    fn test_random() -> Result<()> {
        let mut random = random();
        let iters = at_least(&mut random, 10);
        for _ in 0..iters {
            do_test_random(false)?;
        }
        Ok(())
    }

    #[test]
    fn test_random_two_phase() -> Result<()> {
        let mut random = random();
        let iters = at_least(&mut random, 10);
        for _ in 0..iters {
            do_test_random(true)?;
        }
        Ok(())
    }

    fn do_test_random(two_phase: bool) -> Result<()> {
        let mut random = random();
        let max_doc = random.random_range(1..=1000);

        let mut req_builder = DocIdSetBuilder::new(max_doc);
        let mut excl_builder = DocIdSetBuilder::new(max_doc);

        let num_included_docs = random.random_range(1..=max_doc);
        let num_excluded_docs = random.random_range(1..=max_doc);

        req_builder.grow(num_included_docs);
        for _ in 0..num_included_docs {
            req_builder.add_doc(random.random_range(0..max_doc));
        }
        excl_builder.grow(num_excluded_docs);
        for _ in 0..num_excluded_docs {
            excl_builder.add_doc(random.random_range(0..max_doc));
        }

        let req = req_builder.build()?;
        let excl = excl_builder.build()?;

        let req_iter = req.iterator()?.unwrap();
        let req_bulk_scorer = BulkScorerImpl::new(req_iter);

        let excl_iter = excl.iterator()?.unwrap();
        let mut req_excl = if two_phase {
            let scorer = ScorerImpl {
                disi: excl_iter,
                random_seed: random.random(),
                has_tpi: true,
            };
            ReqExclBulkScorer::from_scorer(req_bulk_scorer, scorer)?
        } else {
            let scorer = ScorerImpl {
                disi: excl_iter,
                random_seed: random.random(),
                has_tpi: false,
            };
            ReqExclBulkScorer::from_scorer(req_bulk_scorer, scorer)?
        };

        let mut actual_matches = FixedBitSet::new(max_doc as usize);

        if random.random_bool(0.5) {
            req_excl.score(
                &mut LeafCollectorImpl::new(&mut actual_matches),
                None::<&dyn Bits>,
                0,
                NO_MORE_DOCS,
            )?;
        } else {
            let mut next = 0;
            while next < max_doc {
                let min = next;
                let max = min + random.random_range(0..10);
                next = req_excl.score(
                    &mut LeafCollectorImpl::new(&mut actual_matches),
                    None::<&dyn Bits>,
                    min,
                    max,
                )?;
                assert!(next >= max);
            }
        }

        let mut expected_matches = FixedBitSet::new(max_doc as usize);
        BitSet::or(&mut expected_matches, &mut req.iterator()?.unwrap())?;

        let mut excluded_set = FixedBitSet::new(max_doc as usize);
        BitSet::or(&mut excluded_set, &mut excl.iterator()?.unwrap())?;

        expected_matches.and_not_fixed_bit_set(&excluded_set);
        assert_eq!(expected_matches.get_bits(), actual_matches.get_bits());
        Ok(())
    }

    struct ScorerImpl<DISI>
    where
        DISI: DocIdSetIterator,
    {
        disi: DISI,
        random_seed: u64,
        has_tpi: bool,
    }

    impl<DISI> Scorable for ScorerImpl<DISI>
    where
        DISI: DocIdSetIterator,
    {
        fn score(&mut self) -> Result<f32> {
            unreachable!("")
        }
    }
    impl<DISI> Scorer for ScorerImpl<DISI>
    where
        DISI: DocIdSetIterator + 'static,
    {
        fn doc_id(&mut self) -> Result<i32> {
            unreachable!("")
        }

        fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
            unreachable!("")
        }

        fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
            unreachable!("")
        }

        fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
            let scorer = *self;
            Box::new(scorer.disi)
        }

        fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
            unreachable!("")
        }

        fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
            unreachable!("")
        }

        fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>> {
            let seed = self.random_seed;
            let scorer = *self;
            let mut random = random_from_seed(seed);
            let v = RandomTwoPhaseView::new(&mut random, scorer.disi);
            Some(Box::new(v))
        }

        fn get_max_score(&mut self, _upto: i32) -> Result<f32> {
            unreachable!("")
        }

        fn has_two_phase_iterator(&self) -> TwoPhaseState {
            if self.has_tpi {
                TwoPhaseState::Yes
            } else {
                TwoPhaseState::No
            }
        }

        fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
            unreachable!("")
        }

        fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
            unreachable!("")
        }
    }

    struct LeafCollectorImpl<'a> {
        actual_matches: &'a mut FixedBitSet,
    }
    impl<'a> LeafCollectorImpl<'a> {
        fn new(actual_matches: &'a mut FixedBitSet) -> Self {
            Self { actual_matches }
        }
    }

    impl Display for LeafCollectorImpl<'_> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", std::any::type_name::<Self>(),)
        }
    }

    impl<'a> LeafCollector for LeafCollectorImpl<'a> {
        fn collect(&mut self, doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
            self.actual_matches.set(doc as usize);
            Ok(())
        }
    }
    struct BulkScorerImpl<DISI>
    where
        DISI: DocIdSetIterator,
    {
        iterator: DISI,
    }
    impl<DISI> BulkScorerImpl<DISI>
    where
        DISI: DocIdSetIterator,
    {
        fn new(iterator: DISI) -> Self {
            Self { iterator }
        }
    }
    impl<DISI> BulkScorer for BulkScorerImpl<DISI>
    where
        DISI: DocIdSetIterator,
    {
        fn score(
            &mut self,
            collector: &mut dyn LeafCollector,
            accept_docs: Option<&dyn Bits>,
            min: i32,
            max: i32,
        ) -> Result<i32> {
            let mut doc = self.iterator.doc_id();

            if doc < min {
                doc = self.iterator.advance(min)?;
            }
            while doc < max {
                let accept = match accept_docs {
                    None => true,
                    Some(bits) => bits.get(doc as usize)?,
                };
                if accept {
                    collector.collect(doc, &mut DummyScorer)?;
                }

                doc = self.iterator.next_doc()?;
            }
            Ok(doc)
        }

        fn cost(&mut self) -> Result<i64> {
            self.iterator.cost()
        }
    }
}

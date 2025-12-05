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
use crate::core::search::boolean_scorer::BooleanScorer;
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::dummy::dummy_scorable::DummyScorable;
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::search::scorer::Scorer;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::dummy::dummy_bits::DummyBits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::int_array_doc_id_set::{IntArrayDocIdSet, IntArrayDocIdSetIterator};
use crate::test::util::lucene_test_case::lucene_test_case_util::random;
use rand::prelude::SliceRandom;
use std::fmt::{Display, Formatter};

#[allow(dead_code)]
pub struct TestBooleanOr;

fn scorer(mut matches: Vec<i32>) -> Result<ScorerImpl> {
    let mut len = matches.len();
    ArrayUtil::grow_exact(&mut matches, len + 1)?;
    len = matches.len();
    matches[len - 1] = NO_MORE_DOCS;
    let it = IntArrayDocIdSet::new(matches, (len - 1).try_into()?)?
        .iterator()?
        .unwrap();
    Ok(ScorerImpl::new(it))
}
#[test]
fn test_sub_scorer_next_is_not_match() -> Result<()> {
    let mut random = random();
    let mut optional_scorers = vec![
        scorer(vec![100000, 1000001, 9_999_999])?,
        scorer(vec![4000, 1000051])?,
        scorer(vec![5000, 100000, 9_999_998, 9_999_999])?,
    ];
    optional_scorers.shuffle(&mut random);

    let needs_scores = rand::random::<bool>();
    let mut bs = BooleanScorer::new(optional_scorers, 1, needs_scores)?;

    let matches = Vec::new();
    let mut collector = LeafCollectorImpl::new(matches);

    bs.score(&mut collector, None::<&DummyBits>, 0, NO_MORE_DOCS)?;

    let expected = vec![4000, 5000, 100000, 1000001, 1000051, 9_999_998, 9_999_999];
    assert_eq!(collector.matches, expected);

    Ok(())
}

struct LeafCollectorImpl {
    matches: Vec<i32>,
}
impl LeafCollectorImpl {
    fn new(matches: Vec<i32>) -> LeafCollectorImpl {
        LeafCollectorImpl { matches }
    }
}

impl Display for LeafCollectorImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::any::type_name::<Self>(),)
    }
}

impl LeafCollector for LeafCollectorImpl {
    fn collect<S>(&mut self, doc: i32, _scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        self.matches.push(doc);
        Ok(())
    }

    type DocIdSetIteratorRef<'a>
        = DummyDISI
    where
        Self: 'a;
}

struct ScorerImpl {
    it: IntArrayDocIdSetIterator,
}
impl ScorerImpl {
    fn new(it: IntArrayDocIdSetIterator) -> ScorerImpl {
        Self { it }
    }
}

impl Scorable for ScorerImpl {
    fn score(&mut self) -> Result<f32> {
        Ok(0.0)
    }

    type Scorable = DummyScorable;
}

impl Scorer for ScorerImpl {
    type DocIdSetIterator = IntArrayDocIdSetIterator;
    type DocIdSetIteratorRef<'a>
        = &'a IntArrayDocIdSetIterator
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>
        = &'a mut IntArrayDocIdSetIterator
    where
        Self: 'a;
    type TwoPhaseIter = DummyTwoPhaseIterator;
    type TwoPhaseIterRef<'a>
        = DummyTwoPhaseIterator
    where
        Self: 'a;

    fn doc_id(&mut self) -> Result<i32> {
        Ok(self.it.doc_id())
    }

    fn iterator_ref(&self) -> Self::DocIdSetIteratorRef<'_> {
        &self.it
    }

    fn iterator(&mut self) -> Self::DocIdSetIteratorMut<'_> {
        &mut self.it
    }

    fn take_iterator(self) -> Self::DocIdSetIterator {
        unreachable!()
    }

    fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
        Ok(f32::MAX)
    }

    fn has_two_phase_iterator(&self) -> bool {
        false
    }
}

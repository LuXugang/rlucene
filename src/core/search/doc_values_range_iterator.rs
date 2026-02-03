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
use crate::core::index::doc_values_skipper::DocValuesSkipper;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// Wrapper around a [`TwoPhaseIterator`] used by doc-values range queries to
/// accelerate matching by leveraging a [`DocValuesSkipper`].
pub struct DocValuesRangeIterator<TPI, DVS>
where
    TPI: TwoPhaseIterator,
    DVS: DocValuesSkipper,
{
    approximation: Approximation<TPI, DVS>,
}
impl<TPI, DVS> DocValuesRangeIterator<TPI, DVS>
where
    TPI: TwoPhaseIterator,
    DVS: DocValuesSkipper,
{
    pub fn new(
        inner_approximation: TPI,
        skipper: DVS,
        lower_value: i64,
        upper_value: i64,
        has_gaps: bool,
    ) -> Self {
        let sub = if has_gaps {
            ApproximationBaseEnum::RangeWithGaps(RangeWithGapsApproximation)
        } else {
            ApproximationBaseEnum::RangeNoGaps(RangeNoGapsApproximation)
        };
        let approximation =
            Approximation::new(inner_approximation, skipper, lower_value, upper_value, sub);
        Self { approximation }
    }
}
impl<TPI, DVS> TwoPhaseIterator for DocValuesRangeIterator<TPI, DVS>
where
    TPI: TwoPhaseIterator,
    DVS: DocValuesSkipper,
{
    type DocIdSetIteratorRef<'a>
        = &'a Approximation<TPI, DVS>
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>
        = &'a mut Approximation<TPI, DVS>
    where
        Self: 'a;

    fn approximation_mut(&mut self) -> Result<Self::DocIdSetIteratorMut<'_>> {
        Ok(&mut self.approximation)
    }

    fn approximation(&self) -> Result<Self::DocIdSetIteratorRef<'_>> {
        Ok(&self.approximation)
    }

    fn matches(&mut self) -> Result<bool> {
        match self.approximation.match_ {
            Match::YES => Ok(true),
            Match::IfDocHasValue => Ok(true),
            Match::MAYBE => self.approximation.inner_approximation.matches(),
            Match::NO => Err(LuceneError::illegal_state("Unpositioned approximation")),
        }
    }

    fn match_cost(&self) -> f32 {
        self.approximation.inner_approximation.match_cost()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Match {
    /// None of the documents in the range match
    NO,

    /// Document values need to be checked to verify matches
    MAYBE,

    /// All documents in the range that have a value match
    IfDocHasValue,

    /// All documents in this range match unconditionally.
    YES,
}
pub struct Approximation<TPI, DVS>
where
    TPI: TwoPhaseIterator,
    DVS: DocValuesSkipper,
{
    inner_approximation: TPI,
    skipper: DVS,
    lower_value: i64,
    upper_value: i64,
    doc: i32,
    match_: Match,
    upto: i32,
    sub: ApproximationBaseEnum,
}
impl<TPI, DVS> Approximation<TPI, DVS>
where
    TPI: TwoPhaseIterator,
    DVS: DocValuesSkipper,
{
    pub fn new(
        inner_approximation: TPI,
        skipper: DVS,
        lower_value: i64,
        upper_value: i64,
        sub: ApproximationBaseEnum,
    ) -> Self {
        Self {
            inner_approximation,
            skipper,
            lower_value,
            upper_value,
            doc: -1,
            match_: Match::MAYBE,
            upto: -1,
            sub,
        }
    }
}
impl<TPI, DVS> DocIdSetIterator for Approximation<TPI, DVS>
where
    TPI: TwoPhaseIterator,
    DVS: DocValuesSkipper,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc_id() + 1)
    }

    fn advance(&mut self, mut target: i32) -> Result<i32> {
        loop {
            if target > self.upto {
                self.skipper.advance(target)?;
                // If target doesn't have a value and is between two blocks, advance()
                // might have moved to a block that doesn't contain `target`.
                target = target.max(self.skipper.min_doc_id_with_level(0));
                if target == NO_MORE_DOCS {
                    self.doc = NO_MORE_DOCS;
                    return Ok(self.doc);
                }
                self.upto = self.skipper.max_doc_id_with_level(0);
                self.match_ = self.sub.match_(0, self)?;

                // If we have a YES or NO decision, see if we still have the same decision on a higher
                // level (= on a wider range of doc IDs)
                let mut next_level = 1;
                while self.match_ != Match::MAYBE
                    && next_level < self.skipper.num_levels()
                    && self.match_ == self.sub.match_(next_level, self)?
                {
                    self.upto = self.skipper.max_doc_id_with_level(next_level);
                    next_level += 1;
                }
            }

            match self.match_ {
                Match::YES => {
                    self.doc = target;
                    return Ok(self.doc);
                },
                Match::MAYBE | Match::IfDocHasValue => {
                    let mut inner_approximation = self.inner_approximation.approximation_mut()?;
                    if target > inner_approximation.doc_id() {
                        target = inner_approximation.advance(target)?;
                    }
                    if target <= self.upto {
                        self.doc = target;
                        return Ok(self.doc);
                    }
                    // Otherwise we are breaking the invariant that `doc` must always be <= upTo, so let
                    // the loop run one more iteration to advance the skipper.
                },
                Match::NO => {
                    if self.upto == NO_MORE_DOCS {
                        self.doc = NO_MORE_DOCS;
                        return Ok(self.doc);
                    }
                    target = self.upto + 1;
                },
            }
        }
    }

    fn cost(&self) -> Result<i64> {
        self.inner_approximation.approximation()?.cost()
    }
}

pub struct RangeNoGapsApproximation;
impl ApproximationBase for RangeNoGapsApproximation {
    fn match_<TPI, DVS>(&self, level: usize, base: &Approximation<TPI, DVS>) -> Result<Match>
    where
        TPI: TwoPhaseIterator,
        DVS: DocValuesSkipper,
    {
        let min_value = base.skipper.min_value_with_level(level);
        let max_value = base.skipper.max_value_with_level(level);

        if min_value > base.upper_value || max_value < base.lower_value {
            Ok(Match::NO)
        } else if min_value >= base.lower_value && max_value <= base.upper_value {
            let doc_count = base.skipper.doc_count_with_level(level);
            let max_doc_id = base.skipper.max_doc_id_with_level(level);
            let min_doc_id = base.skipper.min_doc_id_with_level(level);

            if doc_count == max_doc_id - min_doc_id + 1 {
                Ok(Match::YES)
            } else {
                Ok(Match::IfDocHasValue)
            }
        } else {
            Ok(Match::MAYBE)
        }
    }
}
pub struct RangeWithGapsApproximation;
impl ApproximationBase for RangeWithGapsApproximation {
    fn match_<TPI, DVS>(&self, level: usize, base: &Approximation<TPI, DVS>) -> Result<Match>
    where
        TPI: TwoPhaseIterator,
        DVS: DocValuesSkipper,
    {
        let min_value = base.skipper.min_value_with_level(level);
        let max_value = base.skipper.max_value_with_level(level);

        if min_value > base.upper_value || max_value < base.lower_value {
            Ok(Match::NO)
        } else {
            Ok(Match::MAYBE)
        }
    }
}
pub enum ApproximationBaseEnum {
    RangeNoGaps(RangeNoGapsApproximation),
    RangeWithGaps(RangeWithGapsApproximation),
}
impl ApproximationBase for ApproximationBaseEnum {
    fn match_<TPI, DVS>(&self, level: usize, base: &Approximation<TPI, DVS>) -> Result<Match>
    where
        TPI: TwoPhaseIterator,
        DVS: DocValuesSkipper,
    {
        match self {
            ApproximationBaseEnum::RangeNoGaps(inner) => inner.match_(level, base),
            ApproximationBaseEnum::RangeWithGaps(inner) => inner.match_(level, base),
        }
    }
}
pub trait ApproximationBase {
    fn match_<TPI, DVS>(&self, level: usize, base: &Approximation<TPI, DVS>) -> Result<Match>
    where
        TPI: TwoPhaseIterator,
        DVS: DocValuesSkipper;
}
#[cfg(test)]
mod tests {
    use crate::core::index::doc_values_iterator::DocValuesIterator;
    use crate::core::index::doc_values_skipper::DocValuesSkipper;
    use crate::core::index::numeric_doc_values::NumericDocValues;
    use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::core::search::doc_values_range_iterator::{DocValuesRangeIterator, Match};
    use crate::core::search::two_phase_iterator::TwoPhaseIterator;
    use crate::core::util::error::lucene_error::{LuceneError, Result};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[allow(dead_code)] // for quick search
    struct TestDocValuesRangeIterator;
    #[test]
    fn test_single_level() -> Result<()> {
        do_test_basics(false)
    }

    #[test]
    fn test_multiple_levels() -> Result<()> {
        do_test_basics(true)
    }

    fn do_test_basics(do_levels: bool) -> Result<()> {
        let query_min: i64 = 10;
        let query_max: i64 = 20;
        // Test with both gaps and no-gaps in the ranges:
        let values = doc_values(query_min, query_max);
        let values2 = doc_values(query_min, query_max);

        let two_phase_called = Arc::new(AtomicBool::new(false));
        let two_phase = two_phase_iterator(values, query_min, query_max, two_phase_called.clone());
        let two_phase_called2 = Arc::new(AtomicBool::new(false));
        let two_phase2 =
            two_phase_iterator(values2, query_min, query_max, two_phase_called2.clone());

        let skipper = doc_values_skipper(query_min, query_max, do_levels);
        let skipper2 = doc_values_skipper(query_min, query_max, do_levels);

        let mut range_iterator =
            DocValuesRangeIterator::new(two_phase, skipper, query_min, query_max, false);
        let mut range_iterator_with_gaps =
            DocValuesRangeIterator::new(two_phase2, skipper2, query_min, query_max, true);

        assert_eq!(100, range_iterator.approximation_mut()?.advance(100)?);
        assert_eq!(
            100,
            range_iterator_with_gaps.approximation_mut()?.advance(100)?
        );
        assert_eq!(Match::YES, range_iterator.approximation.match_);
        assert_eq!(Match::MAYBE, range_iterator_with_gaps.approximation.match_);
        assert_eq!(255, range_iterator.approximation.upto);
        if do_levels {
            assert_eq!(127, range_iterator_with_gaps.approximation.upto);
        } else {
            assert_eq!(255, range_iterator_with_gaps.approximation.upto);
        }

        assert!(range_iterator.matches()?);
        assert!(range_iterator_with_gaps.matches()?);

        assert!(
            range_iterator.approximation.inner_approximation.values.doc
                < range_iterator.approximation.doc
        );
        assert_eq!(
            range_iterator_with_gaps
                .approximation
                .inner_approximation
                .values
                .doc,
            range_iterator_with_gaps.approximation.doc
        );
        assert!(!two_phase_called.load(Ordering::SeqCst));
        assert!(two_phase_called2.load(Ordering::SeqCst));
        two_phase_called2.store(false, Ordering::SeqCst);

        assert_eq!(768, range_iterator.approximation_mut()?.advance(300)?);
        assert_eq!(
            768,
            range_iterator_with_gaps.approximation_mut()?.advance(300)?
        );
        assert_eq!(Match::MAYBE, range_iterator.approximation.match_);
        assert_eq!(Match::MAYBE, range_iterator_with_gaps.approximation.match_);

        if do_levels {
            assert_eq!(831, range_iterator.approximation.upto);
            assert_eq!(831, range_iterator_with_gaps.approximation.upto);
        } else {
            assert_eq!(1023, range_iterator.approximation.upto);
            assert_eq!(1023, range_iterator_with_gaps.approximation.upto);
        }

        for _ in 0..10 {
            assert_eq!(
                range_iterator.approximation.inner_approximation.values.doc,
                range_iterator.approximation.doc
            );
            assert_eq!(
                range_iterator_with_gaps
                    .approximation
                    .inner_approximation
                    .values
                    .doc,
                range_iterator_with_gaps.approximation.doc
            );
            assert_eq!(
                range_iterator.approximation.inner_approximation.matches()?,
                range_iterator.matches()?
            );
            assert_eq!(
                range_iterator_with_gaps
                    .approximation
                    .inner_approximation
                    .matches()?,
                range_iterator_with_gaps.matches()?
            );
            assert!(two_phase_called.load(Ordering::SeqCst));
            assert!(two_phase_called2.load(Ordering::SeqCst));
            two_phase_called.store(false, Ordering::SeqCst);
            two_phase_called2.store(false, Ordering::SeqCst);
            range_iterator.approximation_mut()?.next_doc()?;
            range_iterator_with_gaps.approximation_mut()?.next_doc()?;
        }

        assert_eq!(1100, range_iterator.approximation_mut()?.advance(1099)?);
        assert_eq!(
            1100,
            range_iterator_with_gaps
                .approximation_mut()?
                .advance(1099)?
        );
        assert_eq!(Match::IfDocHasValue, range_iterator.approximation.match_);
        assert_eq!(Match::MAYBE, range_iterator_with_gaps.approximation.match_);
        assert_eq!(1024 + 256 - 1, range_iterator.approximation.upto);
        if do_levels {
            assert_eq!(1024 + 128 - 1, range_iterator_with_gaps.approximation.upto);
        } else {
            assert_eq!(1024 + 256 - 1, range_iterator_with_gaps.approximation.upto);
        }
        assert_eq!(
            range_iterator.approximation.inner_approximation.values.doc,
            range_iterator.approximation.doc
        );
        assert_eq!(
            range_iterator_with_gaps
                .approximation
                .inner_approximation
                .values
                .doc,
            range_iterator_with_gaps.approximation.doc
        );
        assert!(range_iterator.matches()?);
        assert!(range_iterator_with_gaps.matches()?);
        assert!(!two_phase_called.load(Ordering::SeqCst));
        assert!(two_phase_called2.load(Ordering::SeqCst));
        two_phase_called2.store(false, Ordering::SeqCst);

        assert_eq!(
            1024 + 768,
            range_iterator.approximation_mut()?.advance(1024 + 300)?
        );
        assert_eq!(
            1024 + 768,
            range_iterator_with_gaps
                .approximation_mut()?
                .advance(1024 + 300)?
        );
        assert_eq!(Match::MAYBE, range_iterator.approximation.match_);
        assert_eq!(Match::MAYBE, range_iterator_with_gaps.approximation.match_);

        if do_levels {
            assert_eq!(1024 + 831, range_iterator.approximation.upto);
            assert_eq!(1024 + 831, range_iterator_with_gaps.approximation.upto);
        } else {
            assert_eq!(2047, range_iterator.approximation.upto);
            assert_eq!(2047, range_iterator_with_gaps.approximation.upto);
        }

        for _ in 0..10 {
            assert_eq!(
                range_iterator.approximation.inner_approximation.values.doc,
                range_iterator.approximation.doc
            );
            assert_eq!(
                range_iterator_with_gaps
                    .approximation
                    .inner_approximation
                    .values
                    .doc,
                range_iterator_with_gaps.approximation.doc
            );
            assert_eq!(
                range_iterator.approximation.inner_approximation.matches()?,
                range_iterator.matches()?
            );
            assert_eq!(
                range_iterator_with_gaps
                    .approximation
                    .inner_approximation
                    .matches()?,
                range_iterator_with_gaps.matches()?
            );
            assert!(two_phase_called.load(Ordering::SeqCst));
            assert!(two_phase_called2.load(Ordering::SeqCst));
            two_phase_called.store(false, Ordering::SeqCst);
            two_phase_called2.store(false, Ordering::SeqCst);
            range_iterator.approximation_mut()?.next_doc()?;
            range_iterator_with_gaps.approximation_mut()?.next_doc()?;
        }

        assert_eq!(
            NO_MORE_DOCS,
            range_iterator.approximation_mut()?.advance(2048)?
        );
        assert_eq!(
            NO_MORE_DOCS,
            range_iterator_with_gaps
                .approximation_mut()?
                .advance(2048)?
        );

        Ok(())
    }

    // Fake numeric doc values so that:
    // docs 0-256 all match
    // docs in 256-512 are all greater than queryMax
    // docs in 512-768 are all less than queryMin
    // docs in 768-1024 have some docs that match the range, others not
    // docs in 1024-2048 follow a similar pattern as docs in 0-1024 except that not all docs have a
    // value
    fn doc_values(query_min: i64, query_max: i64) -> NumericDocValuesImpl {
        NumericDocValuesImpl::new(query_min, query_max)
    }

    fn two_phase_iterator<NDV>(
        values: NDV,
        query_min: i64,
        query_max: i64,
        two_phase_called: Arc<AtomicBool>,
    ) -> TwoPhaseIteratorImpl<NDV>
    where
        NDV: NumericDocValues,
    {
        TwoPhaseIteratorImpl::new(values, query_min, query_max, two_phase_called)
    }

    fn doc_values_skipper(query_min: i64, query_max: i64, do_levels: bool) -> DocValuesSkipperImpl {
        DocValuesSkipperImpl::new(query_min, query_max, do_levels)
    }

    struct NumericDocValuesImpl {
        doc: i32,
        query_min: i64,
        query_max: i64,
    }
    impl NumericDocValuesImpl {
        pub fn new(query_min: i64, query_max: i64) -> Self {
            Self {
                doc: -1,
                query_min,
                query_max,
            }
        }
    }

    impl DocValuesIterator for NumericDocValuesImpl {
        fn advance_exact(&mut self, _target: i32) -> Result<bool> {
            Err(LuceneError::unsupported_operation(""))
        }
    }

    impl DocIdSetIterator for NumericDocValuesImpl {
        fn doc_id(&self) -> i32 {
            self.doc
        }

        fn next_doc(&mut self) -> Result<i32> {
            self.advance(self.doc + 1)
        }

        fn advance(&mut self, target: i32) -> Result<i32> {
            let new_doc = if target < 1024 {
                // dense up to 1024
                target
            } else if self.doc < 2047 {
                // 50% docs have a value up to 2048
                target + (target & 1)
            } else {
                NO_MORE_DOCS
            };

            self.doc = new_doc;
            Ok(self.doc)
        }

        fn cost(&self) -> Result<i64> {
            Ok(42)
        }
    }

    impl NumericDocValues for NumericDocValuesImpl {
        fn long_value(&mut self) -> Result<i64> {
            let d = self.doc % 1024;

            let v = if d < 128 {
                (self.query_min + self.query_max) >> 1
            } else if d < 256 {
                self.query_max + 1
            } else if d < 512 {
                self.query_min - 1
            } else {
                match (d / 2) % 3 {
                    0 => self.query_min - 1,
                    1 => self.query_max + 1,
                    2 => (self.query_min + self.query_max) >> 1,
                    _ => unreachable!(),
                }
            };

            Ok(v)
        }
    }

    struct TwoPhaseIteratorImpl<NDV>
    where
        NDV: NumericDocValues,
    {
        two_phase_called: Arc<AtomicBool>,
        query_min: i64,
        query_max: i64,
        values: NDV,
    }
    impl<NDV> TwoPhaseIteratorImpl<NDV>
    where
        NDV: NumericDocValues,
    {
        pub fn new(
            values: NDV,
            query_min: i64,
            query_max: i64,
            two_phase_called: Arc<AtomicBool>,
        ) -> Self {
            Self {
                two_phase_called,
                query_min,
                query_max,
                values,
            }
        }
    }
    impl<NDV> TwoPhaseIterator for TwoPhaseIteratorImpl<NDV>
    where
        NDV: NumericDocValues,
    {
        type DocIdSetIteratorRef<'a>
            = &'a NDV
        where
            Self: 'a;
        type DocIdSetIteratorMut<'a>
            = &'a mut NDV
        where
            Self: 'a;

        fn approximation_mut(&mut self) -> Result<Self::DocIdSetIteratorMut<'_>> {
            Ok(&mut self.values)
        }

        fn approximation(&self) -> Result<Self::DocIdSetIteratorRef<'_>> {
            Ok(&self.values)
        }

        fn matches(&mut self) -> Result<bool> {
            self.two_phase_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
            let value = self.values.long_value()?;
            Ok(value >= self.query_min && value <= self.query_max)
        }

        fn match_cost(&self) -> f32 {
            2.0
        }
    }

    struct DocValuesSkipperImpl {
        doc: i32,
        do_levels: bool,
        query_min: i64,
        query_max: i64,
    }
    impl DocValuesSkipperImpl {
        pub fn new(query_min: i64, query_max: i64, do_levels: bool) -> Self {
            Self {
                doc: -1,
                do_levels,
                query_min,
                query_max,
            }
        }

        fn range_log(&self, level: usize) -> i32 {
            (9 - self.num_levels() + level) as i32
        }
    }

    impl DocValuesSkipper for DocValuesSkipperImpl {
        fn advance(&mut self, target: i32) -> Result<()> {
            self.doc = target;
            Ok(())
        }

        fn num_levels(&self) -> usize {
            if self.do_levels { 3 } else { 1 }
        }

        fn min_doc_id_with_level(&self, level: usize) -> i32 {
            let range_log = self.range_log(level);

            if self.doc < 0 {
                -1
            } else if self.doc >= 2048 {
                NO_MORE_DOCS
            } else {
                let mask = (1 << range_log) - 1;
                self.doc & !mask
            }
        }

        fn max_doc_id_with_level(&self, level: usize) -> i32 {
            let range_log = self.range_log(level);
            let min_doc_id = self.min_doc_id_with_level(level);

            match min_doc_id {
                -1 => -1,
                x if x == NO_MORE_DOCS => NO_MORE_DOCS,
                _ => min_doc_id + ((1 << range_log) - 1),
            }
        }
        #[allow(clippy::if_same_then_else)]
        fn min_value_with_level(&self, _level: usize) -> i64 {
            let d = self.doc % 1024;
            if d < 128 {
                self.query_min
            } else if d < 256 {
                self.query_max + 1
            } else if d < 768 {
                self.query_min - 1
            } else {
                self.query_min - 1
            }
        }

        fn max_value_with_level(&self, _level: usize) -> i64 {
            let d = self.doc % 1024;
            if d < 128 {
                self.query_max
            } else if d < 256 {
                self.query_max + 1
            } else if d < 768 {
                self.query_min - 1
            } else {
                self.query_max + 1
            }
        }

        fn doc_count_with_level(&self, level: usize) -> i32 {
            let range_log = self.range_log(level);

            if self.doc < 1024 {
                1 << range_log
            } else {
                (1 << range_log) >> 1
            }
        }

        fn min_value(&self) -> i64 {
            i64::MIN
        }

        fn max_value(&self) -> i64 {
            i64::MAX
        }

        fn doc_count(&self) -> i32 {
            1024 + 1024 / 2
        }
    }
}

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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::search::field_comparator::FieldComparatorEnum;
use crate::core::search::leaf_field_comparator::{
    LeafFieldComparator, LeafFieldComparatorDocIdSetIteratorRef, LeafFieldComparatorEnum,
};
use crate::core::search::scorable::Scorable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::rc::Rc;

pub struct MultiLeafFieldComparator<LR>
where
    LR: LeafReader,
{
    comparators: Vec<LeafFieldComparatorEnum<LR>>,
    reverse_mul: Rc<Vec<i32>>,
}
impl<LR> MultiLeafFieldComparator<LR>
where
    LR: LeafReader,
{
    pub(crate) fn new(
        comparators: Vec<LeafFieldComparatorEnum<LR>>,
        reverse_mul: Rc<Vec<i32>>,
    ) -> Result<Self> {
        if comparators.len() != reverse_mul.len() {
            return Err(LuceneError::illegal_argument(format!(
                "Must have the same number of comparators and reverse_mul, got {} and {}",
                comparators.len(),
                reverse_mul.len()
            )));
        }

        Ok(Self {
            comparators,
            reverse_mul,
        })
    }
    pub(crate) fn set_bottom(
        &mut self,
        slot: usize,
        comparators: &mut [FieldComparatorEnum],
    ) -> Result<()> {
        debug_assert_eq!(
            self.comparators.len(),
            comparators.len(),
            "comparators length mismatch"
        );

        for (comp, c) in self.comparators.iter_mut().zip(comparators.iter_mut()) {
            comp.set_bottom(slot, c)?;
        }

        Ok(())
    }

    pub(crate) fn compare_bottom<S>(
        &mut self,
        doc: i32,
        scorer: &mut S,
        comparators: &mut [FieldComparatorEnum],
    ) -> Result<i32>
    where
        S: Scorable,
    {
        debug_assert!(
            !self.comparators.is_empty(),
            "comparators list should not be empty"
        );
        debug_assert_eq!(
            self.comparators.len(),
            comparators.len(),
            "comparators length mismatch"
        );
        let cmp = self.reverse_mul[0]
            * self.comparators[0].compare_bottom(doc, scorer, &mut comparators[0])?;
        if cmp != 0 {
            return Ok(cmp);
        }
        for ((reverse, comp_self), comp_arg) in self
            .reverse_mul
            .iter()
            .zip(self.comparators.iter_mut())
            .zip(comparators.iter_mut())
            .skip(1)
        {
            let cmp = *reverse * comp_self.compare_bottom(doc, scorer, comp_arg)?;
            if cmp != 0 {
                return Ok(cmp);
            }
        }

        Ok(0)
    }

    pub(crate) fn compare_top<S>(
        &mut self,
        doc: i32,
        scorer: &mut S,
        comparators: &mut [FieldComparatorEnum],
    ) -> Result<i32>
    where
        S: Scorable,
    {
        debug_assert!(
            !self.comparators.is_empty(),
            "comparators list should not be empty"
        );
        debug_assert_eq!(
            self.comparators.len(),
            comparators.len(),
            "comparators length mismatch"
        );

        let mut cmp = self.reverse_mul[0]
            * self.comparators[0].compare_top(doc, scorer, &mut comparators[0])?;
        if cmp != 0 {
            return Ok(cmp);
        }

        for ((reverse, comp_self), comp_arg) in self
            .reverse_mul
            .iter()
            .zip(self.comparators.iter_mut())
            .zip(comparators.iter_mut())
            .skip(1)
        {
            cmp = *reverse * comp_self.compare_top(doc, scorer, comp_arg)?;
            if cmp != 0 {
                return Ok(cmp);
            }
        }

        Ok(0)
    }

    pub(crate) fn copy<S>(
        &mut self,
        slot: usize,
        doc: i32,
        scorer: &mut S,
        comparators: &mut [FieldComparatorEnum],
    ) -> Result<()>
    where
        S: Scorable,
    {
        debug_assert_eq!(
            self.comparators.len(),
            comparators.len(),
            "comparators length mismatch"
        );

        for (comp_self, comp_arg) in self.comparators.iter_mut().zip(comparators.iter_mut()) {
            comp_self.copy(slot, doc, scorer, comp_arg)?;
        }

        Ok(())
    }

    pub(crate) fn set_scorer<S>(
        &mut self,
        scorer: &mut S,
        comparators: &mut [FieldComparatorEnum],
    ) -> Result<()>
    where
        S: Scorable,
    {
        debug_assert_eq!(
            self.comparators.len(),
            comparators.len(),
            "comparators length mismatch"
        );

        for (comp_self, comp_arg) in self.comparators.iter_mut().zip(comparators.iter_mut()) {
            comp_self.set_scorer(scorer, comp_arg)?;
        }

        Ok(())
    }

    pub(crate) fn competitive_iterator(
        &mut self,
        comparators: &mut [FieldComparatorEnum],
    ) -> Result<Option<LeafFieldComparatorDocIdSetIteratorRef<'_, LR>>> {
        debug_assert!(!comparators.is_empty());
        self.comparators[0].competitive_iterator(&mut comparators[0])
    }

    pub(crate) fn set_hits_threshold_reached(
        &mut self,
        comparators: &mut [FieldComparatorEnum],
    ) -> Result<()> {
        // this is needed for skipping functionality that is only relevant for the 1st comparator
        self.comparators[0].set_hits_threshold_reached(&mut comparators[0])
    }
}

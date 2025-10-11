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
    LeafFieldComparator, LeafFieldComparatorDocIdSetIterator, LeafFieldComparatorEnum,
};
use crate::core::search::scorable::Scorable;
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub struct MultiLeafFieldComparator<LR>
where
    LR: LeafReader,
{
    comparators: Vec<LeafFieldComparatorEnum<LR>>,
    reverse_mul: Vec<i32>,
}
impl<LR> MultiLeafFieldComparator<LR>
where
    LR: LeafReader,
{
    pub fn new(
        comparators: Vec<LeafFieldComparatorEnum<LR>>,
        reverse_mul: Vec<i32>,
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
}
impl<LR> LeafFieldComparator for MultiLeafFieldComparator<LR>
where
    LR: LeafReader,
{
    type FieldComparator = FieldComparatorEnum;
    fn set_bottom(&mut self, slot: usize, comparator: &mut Self::FieldComparator) -> Result<()> {
        for comp in &mut self.comparators {
            comp.set_bottom(slot, comparator)?;
        }
        Ok(())
    }

    fn compare_bottom<S>(
        &mut self,
        doc: i32,
        scorer: &mut S,
        comparator: &mut Self::FieldComparator,
    ) -> Result<i32>
    where
        S: Scorable,
    {
        debug_assert!(
            !self.comparators.is_empty(),
            "comparators list should not be empty"
        );

        let mut cmp =
            self.reverse_mul[0] * self.comparators[0].compare_bottom(doc, scorer, comparator)?;
        if cmp != 0 {
            return Ok(cmp);
        }

        for i in 1..self.comparators.len() {
            cmp = self.reverse_mul[i]
                * self.comparators[i].compare_bottom(doc, scorer, comparator)?;
            if cmp != 0 {
                return Ok(cmp);
            }
        }

        Ok(0)
    }

    fn compare_top<S>(
        &mut self,
        doc: i32,
        scorer: &mut S,
        comparator: &mut Self::FieldComparator,
    ) -> Result<i32>
    where
        S: Scorable,
    {
        debug_assert!(
            !self.comparators.is_empty(),
            "comparators list should not be empty"
        );

        let mut cmp =
            self.reverse_mul[0] * self.comparators[0].compare_top(doc, scorer, comparator)?;
        if cmp != 0 {
            return Ok(cmp);
        }

        for i in 1..self.comparators.len() {
            cmp = self.reverse_mul[i] * self.comparators[i].compare_top(doc, scorer, comparator)?;
            if cmp != 0 {
                return Ok(cmp);
            }
        }

        Ok(0)
    }

    fn copy<S>(
        &mut self,
        slot: usize,
        doc: i32,
        scorer: &mut S,
        comparator: &mut Self::FieldComparator,
    ) -> Result<()>
    where
        S: Scorable,
    {
        for comp in &mut self.comparators {
            comp.copy(slot, doc, scorer, comparator)?;
        }
        Ok(())
    }

    fn set_scorer<S>(
        &mut self,
        scorer: &mut S,
        comparator: &mut Self::FieldComparator,
    ) -> Result<()>
    where
        S: Scorable,
    {
        for comp in &mut self.comparators {
            comp.set_scorer(scorer, comparator)?;
        }
        Ok(())
    }

    type DocIdSetIterator = LeafFieldComparatorDocIdSetIterator<LR>;

    fn competitive_iterator(
        &mut self,
        comparator: &mut Self::FieldComparator,
    ) -> Option<Self::DocIdSetIterator> {
        self.comparators[0].competitive_iterator(comparator)
    }

    fn set_hits_threshold_reached(&mut self, comparator: &mut Self::FieldComparator) -> Result<()> {
        self.comparators[0].set_hits_threshold_reached(comparator)
    }
}

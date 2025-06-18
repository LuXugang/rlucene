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
use std::rc::Rc;

use crate::search::doc_id_set::DocIdSet;
use crate::util::accountable::Accountable;
use crate::util::bit_set::BitSet;
use crate::util::bit_set_iterator::BitSetIterator;
use crate::util::error::lucene_error::{LuceneError, Result};

//TODO
#[allow(unused)]
const BASE_RAM_BYTES_USED: i64 = 0;

/// Implementation of the [`DocIdSet`] interface on top of a [`BitSet`].
///
/// # Note
/// This is an internal API.
pub struct BitDocIdSet<T: BitSet> {
    set: Option<Rc<T>>,
    pub(crate) cost: i64,
}
/// Wraps the given [`BitSet`] as a [`DocIdSet`].
/// The provided [`BitSet`] must not be modified afterwards.
impl<T: BitSet> BitDocIdSet<T> {
    pub fn with_cost(set: Option<T>, cost: i64) -> Result<BitDocIdSet<T>> {
        if cost < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "cost must be >= 0, got {}",
                cost
            )));
        }
        Ok(BitDocIdSet {
            set: Some(Rc::new(set.unwrap())),
            cost,
        })
    }
    /// Same as [`BitDocIdSet`] but uses the set's
    /// [`BitSet::approximate_cardinality`] as a cost.
    pub fn new(set: Option<T>) -> Result<BitDocIdSet<T>> {
        let cost = set.as_ref().unwrap().approximate_cardinality();
        Self::with_cost(set, cost as i64)
    }
}

impl<T> Accountable for BitDocIdSet<T>
where
    T: BitSet + Clone,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        self.set.as_ref().unwrap().ram_bytes_used()
    }
}

impl<T> DocIdSet for BitDocIdSet<T>
where
    T: BitSet + Clone + 'static,
{
    type DocIdSetIterator<'a> = BitSetIterator<T>;

    fn iterator(&self) -> Result<Option<Self::DocIdSetIterator<'_>>> {
        Ok(self
            .set
            .as_ref()
            .map(|set| BitSetIterator::new(set.clone(), self.cost).unwrap()))
    }

    type BitType = T;

    fn bits(&self) -> Option<Rc<Self::BitType>> {
        self.set.clone()
    }
}

#[cfg(test)]
mod tests {

    use rand::Rng;

    use crate::search::doc_id_set::DocIdSet;
    use crate::test::util::base_doc_id_set_test_case::{
        BaseDocIdSetTestCase, BaseDocIdSetTestCaseSupperImpl,
    };
    use crate::test::util::lucene_test_case::random;
    use crate::util::bit_doc_id_set::BitDocIdSet;
    use crate::util::bit_set::BitSet;
    use crate::util::error::lucene_error::Result;
    use crate::util::fixed_bit_set::FixedBitSet;

    impl BaseDocIdSetTestCase for TestFixedBitDocIdSet {
        fn copy_of(&self, bs: &bit_set::BitSet, length: i32) -> impl DocIdSet {
            let mut set = FixedBitSet::new(length);
            let iter = bs.iter();
            for doc in iter {
                set.set(doc as i32);
            }
            let result = BitDocIdSet::new(Some(set));
            assert!(result.is_ok());
            result.unwrap()
        }

        fn assert_equals<R: Rng + ?Sized>(
            &self,
            random: &mut R,
            num_bits: i32,
            ds1: &bit_set::BitSet,
            ds2: impl DocIdSet,
        ) -> Result<()> {
            BaseDocIdSetTestCaseSupperImpl::assert_equals(self, random, num_bits, ds1, ds2)
        }
    }

    pub struct TestFixedBitDocIdSet;

    #[test]
    fn test_bit_0() -> Result<()> {
        let test_case = TestFixedBitDocIdSet;
        let mut random = random();
        test_case.test_bit_0(&mut random)
    }
    #[test]
    fn test_bit_1() -> Result<()> {
        let test_case = TestFixedBitDocIdSet;
        let mut random = random();
        test_case.test_bit_1(&mut random)
    }
    #[test]
    fn test_bit_2() -> Result<()> {
        let test_case = TestFixedBitDocIdSet;
        let mut random = random();
        test_case.test_bit_2(&mut random)
    }
    #[test]
    fn test_against_bit_set() -> Result<()> {
        let test_case = TestFixedBitDocIdSet;
        let mut random = random();
        test_case.test_against_bit_set(&mut random)
    }
    #[test]
    fn test_ram_bytes_used() {
        let test_case = TestFixedBitDocIdSet;
        let mut random = random();
        test_case.test_ram_bytes_used(&mut random);
    }

    impl BaseDocIdSetTestCaseSupperImpl for TestFixedBitDocIdSet {}
}

/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/
use std::rc::Rc;

use crate::search::doc_id_set::DocIdSet;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::accountable::Accountable;
use crate::util::bits::MatchNoBits;
use crate::util::error::lucene_error::{LuceneError, Result};

// TODO
#[allow(unused)]
const BASE_RAM_BYTES_USED: i64 = 0;

/// A doc id set based on a sorted `Vec<i32>`.
///
/// # Note
/// This is an internal API.
pub struct IntArrayDocIdSet {
    docs: Rc<Vec<i32>>,
    length: i32,
}
/// Builds an `IntArrayDocIdSet` from an `i32` array and its length.
///
/// # Arguments
/// * `docs` - A docs array whose length must be greater than the `len`
///   parameter. The array needs to be sorted from 0 (inclusive) to `len`
///   (exclusive), and the `len`-th doc in `docs` must be
///   [`DocIdSetIterator::NO_MORE_DOCS`](NO_MORE_DOCS).
/// * `len` - The valid docs length in the array.
impl IntArrayDocIdSet {
    pub fn new(docs: Vec<i32>, length: i32) -> Result<IntArrayDocIdSet> {
        if docs[length as usize] != NO_MORE_DOCS {
            return Err(LuceneError::illegal_argument(format!(
                "last value must be {}",
                NO_MORE_DOCS
            )));
        }
        debug_assert!(
            assert_array_sorted(&docs),
            "IntArrayDocIdSet need docs to be sorted:{}",
            docs.iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        );
        Ok(IntArrayDocIdSet {
            docs: Rc::new(docs),
            length,
        })
    }
}
fn assert_array_sorted(docs: &[i32]) -> bool {
    docs.windows(2).all(|w| w[0] < w[1])
}

impl DocIdSet for IntArrayDocIdSet {
    type DocIdSetIterator = IntArrayDocIdSetIterator;

    fn iterator(&self) -> Result<Option<Self::DocIdSetIterator>> {
        Ok(Some(IntArrayDocIdSetIterator::new(
            self.docs.clone(),
            self.length,
        )))
    }

    type BitType = MatchNoBits;

    fn bits(&self) -> Option<Rc<Self::BitType>> {
        None
    }
}

impl Accountable for IntArrayDocIdSet {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

pub struct IntArrayDocIdSetIterator {
    docs: Rc<Vec<i32>>,
    length: i32,
    i: i32,
    doc: i32,
}
impl IntArrayDocIdSetIterator {
    pub fn new(docs: Rc<Vec<i32>>, length: i32) -> IntArrayDocIdSetIterator {
        IntArrayDocIdSetIterator {
            docs,
            length,
            i: 0,
            doc: -1,
        }
    }
}
impl DocIdSetIterator for IntArrayDocIdSetIterator {
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.doc = self.docs[self.i as usize];
        self.i += 1;
        Ok(self.doc)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        let mut bound = 1;
        // given that we use this for small arrays only, this is very unlikely
        // to overflow
        while (self.i + bound < self.length)
            && (self.docs[self.i as usize + bound as usize] < target)
        {
            bound *= 2;
        }
        let mut start = self.i as usize + (bound / 2) as usize;
        let end = std::cmp::min(self.i + bound + 1, self.length - 1) as usize;
        let index = self.docs[start..end]
            .binary_search(&target)
            .unwrap_or_else(|index| index);
        start += index;
        self.doc = self.docs[start];
        self.i = start as i32 + 1;
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.length as i64)
    }
}

#[cfg(test)]
mod tests {

    use rand::Rng;

    use crate::search::doc_id_set::DocIdSet;
    use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::test::util::base_doc_id_set_test_case::{
        BaseDocIdSetTestCase, BaseDocIdSetTestCaseSupperImpl,
    };
    use crate::test::util::lucene_test_case::random;
    use crate::util::error::lucene_error::Result;
    use crate::util::int_array_doc_id_set::IntArrayDocIdSet;

    struct TestIntArrayDocIdSet;
    impl BaseDocIdSetTestCase for TestIntArrayDocIdSet {
        fn copy_of(&self, bs: &bit_set::BitSet, _length: i32) -> impl DocIdSet {
            let mut docs: Vec<i32> = vec![];
            let iter = bs.iter();
            for doc in iter {
                docs.push(doc as i32);
            }
            let l = docs.len() as i32;
            docs.push(NO_MORE_DOCS);
            let result = IntArrayDocIdSet::new(docs, l);
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
    #[test]
    fn test_bit_0() -> Result<()> {
        let test_case = TestIntArrayDocIdSet;
        let mut random = random();
        test_case.test_bit_0(&mut random)
    }

    #[test]
    fn test_bit_1() -> Result<()> {
        let test_case = TestIntArrayDocIdSet;
        let mut random = random();
        test_case.test_bit_1(&mut random)
    }
    #[test]
    fn test_bit_2() -> Result<()> {
        let test_case = TestIntArrayDocIdSet;
        let mut random = random();
        test_case.test_bit_2(&mut random)
    }
    #[test]
    fn test_against_bit_set() -> Result<()> {
        let test_case = TestIntArrayDocIdSet;
        let mut random = random();
        test_case.test_against_bit_set(&mut random)
    }
    #[test]
    fn test_ram_bytes_used() {
        let test_case = TestIntArrayDocIdSet;
        let mut random = random();
        test_case.test_ram_bytes_used(&mut random);
    }

    impl BaseDocIdSetTestCaseSupperImpl for TestIntArrayDocIdSet {}
}

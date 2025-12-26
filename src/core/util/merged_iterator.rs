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
use crate::core::util::priority_queue::{Compare, PriorityQueue};

use crate::core::util::ToInt;
/// Provides a merged, sorted view over several sorted iterators.
///
/// If built with `remove_duplicates` set to `true` and an element appears in multiple iterators,
/// then it is deduplicated; in other words, this iterator returns the sorted union of elements.
///
/// If built with `remove_duplicates` set to `false`, then all elements from all iterators are
/// returned.
///
/// # Caveats
///
/// - The behavior is undefined if the iterators are not actually sorted.
/// - `None` (null) elements are unsupported.
/// - If `remove_duplicates` is set to `true` and a single iterator itself contains duplicates,
///   those duplicates will **not** be deduplicated.
/// - When elements are deduplicated, it is not defined which instance is returned.
/// - If `remove_duplicates` is set to `false`, the order in which duplicates are returned is
///   undefined.
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::iterator::IteratorExt;

pub struct MergedIterator<E>
where
    E: IteratorExt,
    E::Item: Ord + Clone,
{
    current: Option<E::Item>,
    queue: PriorityQueue<usize, TermMergeQueueCmp<E>>,
    top: Vec<usize>,
    remove_duplicates: bool,
    num_top: usize,
}
impl<E> MergedIterator<E>
where
    E: IteratorExt,
    E::Item: Ord + Clone,
{
    pub fn new(iterators: Vec<E>) -> Result<Self> {
        Self::with_remove_duplicates(true, iterators)
    }
    pub fn with_remove_duplicates(remove_duplicates: bool, iterators: Vec<E>) -> Result<Self> {
        let len = iterators.len();
        let mut sub_iterators = Vec::new();
        for (index, mut it) in iterators.into_iter().enumerate() {
            if it.has_next()? {
                let current = it
                    .next()?
                    .ok_or_else(|| LuceneError::illegal_state("has no next"))?;
                sub_iterators.push(SubIterator {
                    iterator: it,
                    current: Some(current),
                    index,
                });
            }
        }
        let sub_iterator_len = sub_iterators.len();
        let cmp = TermMergeQueueCmp {
            sub_iterator: sub_iterators,
        };

        let mut queue = PriorityQueue::new(len.try_into()?, cmp)?;

        for i in 0..sub_iterator_len {
            queue.add(i)?;
        }
        Ok(Self {
            current: None,
            queue,
            top: vec![0; sub_iterator_len],
            remove_duplicates,
            num_top: 0,
        })
    }
    fn pull_top(&mut self) -> Result<()> {
        debug_assert!(self.num_top == 0);
        let first = self
            .queue
            .pop()?
            .ok_or_else(|| LuceneError::illegal_state("queue is empty"))?;
        self.top[self.num_top] = first;
        self.num_top += 1;

        if self.remove_duplicates {
            // extract all subs from the queue that have the same top element
            while self.queue.size() > 0 {
                let top_idx = *self.queue.top().unwrap();
                let first_idx = self.top[0];

                if self.queue.compare.sub_iterator[top_idx].current
                    == self.queue.compare.sub_iterator[first_idx].current
                {
                    let idx = self.queue.pop()?.unwrap();
                    self.top[self.num_top] = idx;
                    self.num_top += 1;
                } else {
                    break;
                }
            }
        }
        let first_idx = self.top[0];
        self.current = self.queue.compare.sub_iterator[first_idx].current.clone();

        Ok(())
    }
    fn push_top(&mut self) -> Result<()> {
        // call next() on each top, and put back into queue
        for i in 0..self.num_top {
            let idx = self.top[i];
            let top = &mut self.queue.compare.sub_iterator[idx];

            if top.iterator.has_next()? {
                top.current = top.iterator.next()?;
                self.queue.add(idx)?;
            } else {
                top.current = None;
            }
        }

        self.num_top = 0;
        Ok(())
    }
}

impl<E> IteratorExt for MergedIterator<E>
where
    E: IteratorExt,
    E::Item: Ord + Clone,
{
    type Item = E::Item;

    fn next(&mut self) -> Result<Option<Self::Item>> {
        // restore queue
        self.push_top()?;

        // gather equal top elements
        if self.queue.size() > 0 {
            self.pull_top()?;
        } else {
            self.current = None;
        }
        match self.current {
            None => Err(LuceneError::no_such_element("no such element")),
            Some(_) => Ok(self.current.clone()),
        }
    }
    fn has_next(&self) -> Result<bool> {
        if self.queue.size() > 0 {
            return Ok(true);
        }

        for i in 0..self.num_top {
            let idx = self.top[i];
            if self.queue.compare.sub_iterator[idx].iterator.has_next()? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

pub struct TermMergeQueueCmp<E>
where
    E: IteratorExt,
    E::Item: Ord + Clone,
{
    sub_iterator: Vec<SubIterator<E>>,
}
impl<E> TermMergeQueueCmp<E>
where
    E: IteratorExt,
    E::Item: Ord + Clone,
{
    fn new(sub_iterator: Vec<SubIterator<E>>) -> TermMergeQueueCmp<E> {
        TermMergeQueueCmp { sub_iterator }
    }
}
impl<E> Compare<usize> for TermMergeQueueCmp<E>
where
    E: IteratorExt,
    E::Item: Ord + Clone,
{
    fn less_than(&self, a: &usize, b: &usize) -> Result<bool> {
        let cmp = self.sub_iterator[*a]
            .current
            .cmp(&self.sub_iterator[*b].current)
            .to_int();
        if cmp != 0 {
            Ok(cmp < 0)
        } else {
            Ok(self.sub_iterator[*a].index < self.sub_iterator[*b].index)
        }
    }
}

pub struct SubIterator<E>
where
    E: IteratorExt,
    E::Item: Ord + Clone,
{
    iterator: E,
    current: Option<E::Item>,
    index: usize,
}
#[cfg(test)]
mod tests {
    use crate::core::util::error::lucene_error::{LuceneError, Result};
    use crate::core::util::iterator::IteratorExt;
    use crate::core::util::merged_iterator::MergedIterator;
    use crate::test::util::lucene_test_case::lucene_test_case_util::random;
    use rand::Rng;

    #[allow(dead_code)] // for quick search
    struct TestMergedIterator;

    #[test]
    fn test_merge_empty() -> Result<()> {
        let merged: MergedIterator<EmptyIter<i32>> =
            MergedIterator::with_remove_duplicates(true, Vec::new())?;
        assert!(!merged.has_next()?);

        let empty = EmptyIter::<i32>::new();
        let merged = MergedIterator::with_remove_duplicates(true, vec![empty])?;
        assert!(!merged.has_next()?);

        let mut random = random();
        let n = random.random_range(0..100);
        let mut iters = Vec::with_capacity(n);
        for _ in 0..n {
            iters.push(EmptyIter::<i32>::new());
        }

        let merged = MergedIterator::with_remove_duplicates(true, iters)?;
        assert!(!merged.has_next()?);

        Ok(())
    }
    const VALS_TO_MERGE: usize = 15000;
    const REPEATS: usize = 2;

    #[test]
    fn test_no_dups_remove_dups() -> Result<()> {
        let mut random = random();
        for _ in 0..REPEATS {
            test_case(&mut random, 1, 1, true)?;
        }
        Ok(())
    }
    #[test]
    fn test_off_itr_dups_remove_dups() -> Result<()> {
        let mut random = random();
        for _ in 0..REPEATS {
            test_case(&mut random, 3, 1, true)?;
        }
        Ok(())
    }

    #[test]
    fn test_on_itr_dups_remove_dups() -> Result<()> {
        let mut random = random();
        for _ in 0..REPEATS {
            test_case(&mut random, 1, 3, true)?;
        }
        Ok(())
    }

    #[test]
    fn test_on_itr_random_dups_remove_dups() -> Result<()> {
        let mut random = random();
        for _ in 0..REPEATS {
            test_case(&mut random, 1, -3, true)?;
        }
        Ok(())
    }

    #[test]
    fn test_both_dups_remove_dups() -> Result<()> {
        let mut random = random();
        for _ in 0..REPEATS {
            test_case(&mut random, 3, 3, true)?;
        }
        Ok(())
    }

    #[test]
    fn test_both_dups_with_random_dups_remove_dups() -> Result<()> {
        let mut random = random();
        for _ in 0..REPEATS {
            test_case(&mut random, 3, -3, true)?;
        }
        Ok(())
    }

    #[test]
    fn test_no_dups_keep_dups() -> Result<()> {
        let mut random = random();
        for _ in 0..REPEATS {
            test_case(&mut random, 1, 1, false)?;
        }
        Ok(())
    }

    #[test]
    fn test_off_itr_dups_keep_dups() -> Result<()> {
        let mut random = random();
        for _ in 0..REPEATS {
            test_case(&mut random, 3, 1, false)?;
        }
        Ok(())
    }

    #[test]
    fn test_on_itr_dups_keep_dups() -> Result<()> {
        let mut random = random();
        for _ in 0..REPEATS {
            test_case(&mut random, 1, 3, false)?;
        }
        Ok(())
    }

    #[test]
    fn test_on_itr_random_dups_keep_dups() -> Result<()> {
        let mut random = random();
        for _ in 0..REPEATS {
            test_case(&mut random, 1, -3, false)?;
        }
        Ok(())
    }

    #[test]
    fn test_both_dups_keep_dups() -> Result<()> {
        let mut random = random();
        for _ in 0..REPEATS {
            test_case(&mut random, 3, 3, false)?;
        }
        Ok(())
    }

    #[test]
    fn test_both_dups_with_random_dups_keep_dups() -> Result<()> {
        let mut random = random();
        for _ in 0..REPEATS {
            test_case(&mut random, 3, -3, false)?;
        }
        Ok(())
    }

    fn test_case<R: Rng + ?Sized>(
        random: &mut R,
        itrs_with_val: usize,
        specified_vals_on_itr: i32,
        remove_dups: bool,
    ) -> Result<()> {
        // Build a random number of lists
        let mut expected: Vec<i32> = Vec::new();
        let num_lists = itrs_with_val + random.random_range(0..(1000 - itrs_with_val));
        let mut lists: Vec<Vec<i32>> = (0..num_lists).map(|_| Vec::new()).collect();

        let start = random.random_range(0..1_000_000);
        let end =
            start + VALS_TO_MERGE / itrs_with_val / specified_vals_on_itr.unsigned_abs() as usize;

        for i in start..end {
            let mut max_list = lists.len();
            let mut max_vals_on_itr = 0;
            let mut sum_vals_on_itr = 0;

            for _ in 0..itrs_with_val {
                let list_idx = random.random_range(0..max_list);

                let vals_on_itr = if specified_vals_on_itr < 0 {
                    1 + random.random_range(0..(-specified_vals_on_itr as usize))
                } else {
                    specified_vals_on_itr as usize
                };

                max_vals_on_itr = max_vals_on_itr.max(vals_on_itr);
                sum_vals_on_itr += vals_on_itr;

                for _ in 0..vals_on_itr {
                    lists[list_idx].push(i as i32);
                }

                max_list -= 1;
                lists.swap(list_idx, max_list);
            }

            let max_count = if remove_dups {
                max_vals_on_itr
            } else {
                sum_vals_on_itr
            };

            for _ in 0..max_count {
                expected.push(i as i32);
            }
        }

        // Now check that they get merged cleanly
        let itrs: Vec<ListIter<i32>> = lists.into_iter().map(ListIter::new).collect();

        let mut merged = MergedIterator::with_remove_duplicates(remove_dups, itrs)?;

        let mut expected_idx = 0;

        while expected_idx < expected.len() {
            assert!(merged.has_next()?);
            let v = merged
                .next()?
                .ok_or_else(|| LuceneError::illegal_state("expected value"))?;
            assert_eq!(expected[expected_idx], v);
            expected_idx += 1;
        }

        assert!(!merged.has_next()?);
        Ok(())
    }

    struct EmptyIter<T> {
        _phantom: std::marker::PhantomData<T>,
    }

    impl<T> EmptyIter<T> {
        fn new() -> Self {
            Self {
                _phantom: std::marker::PhantomData,
            }
        }
    }

    impl<T> IteratorExt for EmptyIter<T> {
        type Item = T;

        fn next(&mut self) -> Result<Option<Self::Item>> {
            Ok(None)
        }

        fn has_next(&self) -> Result<bool> {
            Ok(false)
        }
    }

    struct ListIter<T> {
        data: Vec<T>,
        pos: usize,
    }

    impl<T> ListIter<T> {
        fn new(data: Vec<T>) -> Self {
            Self { data, pos: 0 }
        }
    }

    impl<T: Clone> IteratorExt for ListIter<T> {
        type Item = T;

        fn next(&mut self) -> Result<Option<Self::Item>> {
            if self.pos < self.data.len() {
                let v = self.data[self.pos].clone();
                self.pos += 1;
                Ok(Some(v))
            } else {
                Ok(None)
            }
        }

        fn has_next(&self) -> Result<bool> {
            Ok(self.pos < self.data.len())
        }
    }
}

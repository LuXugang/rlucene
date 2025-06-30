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
use crate::util::comparator::Comparator;
use crate::util::error::lucene_error::Result;
use crate::util::intro_sorter::IntroSorter;
use crate::util::sorter::Sorter;

/// An [`IntroSorter`] for object arrays.
///
/// # Note
/// This is an internal API.
pub(crate) struct ArrayIntroSorter<'a, T, C: Comparator<T>> {
    pub arr: &'a mut [T],
    comparator: C,
    pivot: i32,
}

impl<'a, T, C> ArrayIntroSorter<'a, T, C>
where
    C: Comparator<T>,
{
    pub fn new(arr: &'a mut [T], comparator: C) -> ArrayIntroSorter<'a, T, C> {
        ArrayIntroSorter {
            arr,
            comparator,
            pivot: 0,
        }
    }
}

impl<T, C> Sorter for ArrayIntroSorter<'_, T, C>
where
    T: Ord,
    C: Comparator<T>,
{
    fn compare(&mut self, i: i32, j: i32) -> Result<i32> {
        self.comparator
            .compare(&self.arr[i as usize], &self.arr[j as usize])
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        // The data pointed to by the pivot has been swapped.
        // We need to adjust the pivot value to ensure that
        // the value corresponding to the pivot remains unchanged.
        // To avoid Copying the value, we just swap the pivot index.
        if self.pivot == i || self.pivot == j {
            self.pivot = if self.pivot == i { j } else { i };
        }
        self.arr.swap(i as usize, j as usize);
        Ok(())
    }

    fn set_pivot(&mut self, i: i32) -> Result<()> {
        self.pivot = i;
        Ok(())
    }

    fn compare_pivot(&mut self, i: i32) -> Result<i32> {
        self.comparator
            .compare(&self.arr[self.pivot as usize], &self.arr[i as usize])
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<()> {
        IntroSorter::sort_range(self, from, to)?;
        Ok(())
    }
}

impl<T, C: Comparator<T>> IntroSorter for ArrayIntroSorter<'_, T, C> where T: Ord {}

#[cfg(test)]
mod tests {

    use rand::Rng;

    use crate::test::util::base_sort_test_case::{BaseSortTestCase, Entry};
    use crate::test::util::lucene_test_case::random;
    use crate::util::{ArrayIntroSorter, Comparator, NaturalOrder, Sorter};

    const STABLE: bool = false;

    struct TestIntroSorter<T, C: Comparator<T>>
    where
        T: Ord,
    {
        _marker: std::marker::PhantomData<(T, C)>,
    }
    impl Default for TestIntroSorter<i32, NaturalOrder<i32>> {
        fn default() -> Self {
            TestIntroSorter {
                _marker: std::marker::PhantomData,
            }
        }
    }

    impl<T, C: Comparator<T>> BaseSortTestCase for TestIntroSorter<T, C>
    where
        T: Ord,
    {
        fn new_sorter<R: Rng + ?Sized>(
            &self,
            _random: &mut R,
            arr: &mut Vec<Entry>,
        ) -> impl Sorter {
            ArrayIntroSorter::new(arr, NaturalOrder::new())
        }

        fn get_stable(&self) -> bool {
            STABLE
        }
    }

    #[test]
    fn test_empty() {
        let mut random = random();
        let case = TestIntroSorter::default();
        case.test_empty(&mut random);
    }
    #[test]
    fn test_one() {
        let mut random = random();
        let case = TestIntroSorter::default();
        case.test_one(&mut random);
    }
    #[test]
    fn test_two() {
        let mut random = random();
        let case = TestIntroSorter::default();
        case.test_two(&mut random);
    }
    #[test]
    fn test_random() {
        let mut random = random();
        let case = TestIntroSorter::default();
        case.test_random(&mut random);
    }
    #[test]
    fn test_random_low_cardinality() {
        let mut random = random();
        let case = TestIntroSorter::default();
        case.test_random_low_cardinality(&mut random);
    }
    #[test]
    fn test_ascending() {
        let mut random = random();
        let case = TestIntroSorter::default();
        case.test_ascending(&mut random);
    }
    #[test]
    fn test_ascending_sequences() {
        let mut random = random();
        let case = TestIntroSorter::default();
        case.test_ascending_sequences(&mut random);
    }
    #[test]
    fn test_descending() {
        let mut random = random();
        let case = TestIntroSorter::default();
        case.test_descending(&mut random);
    }
    #[test]
    fn test_strictly_descending() {
        let mut random = random();
        let case = TestIntroSorter::default();
        case.test_strictly_descending(&mut random);
    }
}

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

use crate::util::error::lucene_error::LuceneError;
use crate::util::{check_range, MSBRadixSorterBase, Sorter, HISTOGRAM_SIZE};

pub struct StableMSBRadixSorter<T>
where
    T: StableMSBRadixSorterBase,
{
    delegate_sorter: T,
    fixed_start_offsets: Vec<i32>,
    max_length: i32,
}

impl<T> StableMSBRadixSorter<T>
where
    T: StableMSBRadixSorterBase,
{
    pub fn new(delegate_sorter: T, max_length: i32) -> StableMSBRadixSorter<T> {
        StableMSBRadixSorter {
            delegate_sorter,
            fixed_start_offsets: vec![0; HISTOGRAM_SIZE],
            max_length,
        }
    }
}

impl<T> Sorter for StableMSBRadixSorter<T> where T: StableMSBRadixSorterBase {}

impl<T> MSBRadixSorterBase for StableMSBRadixSorter<T>
where
    T: StableMSBRadixSorterBase,
{
    fn byte_at(&mut self, i: i32, k: i32) -> Result<i32, LuceneError> {
        self.delegate_sorter.byte_at(i, k)
    }

    fn get_fallback_sorter(&mut self, k: i32, _length: i32) -> impl Sorter {
        let delegate_sorter = MergeSorterImpl::new(k, self.max_length, &mut self.delegate_sorter);
        MergeSorter {
            delegate_sorter,
            pivot_index: 0,
        }
    }

    fn reorder(
        &mut self,
        from: i32,
        to: i32,
        start_offsets: &mut [i32],
        end_offsets: &mut [i32],
        k: i32,
    ) -> Result<(), LuceneError> {
        // Copy start_offsets to fixed_start_offsets
        self.fixed_start_offsets[..start_offsets.len()].copy_from_slice(start_offsets);

        for (i, &limit) in end_offsets.iter().enumerate().take(HISTOGRAM_SIZE) {
            let mut h1 = self.fixed_start_offsets[i];
            while h1 < limit {
                let b = self.get_bucket(from + h1, k)?;
                let h2 = start_offsets[b as usize];
                start_offsets[b as usize] += 1;
                self.delegate_sorter.save(from + h1, from + h2);
                h1 += 1;
            }
        }

        self.delegate_sorter.restore(from, to);
        Ok(())
    }
}

pub trait StableMSBRadixSorterBase: MSBRadixSorterBase {
    /// Save the i-th value into the j-th position in temporary storage.
    fn save(&mut self, i: i32, j: i32);
    /// Restore values between i-th and j-th(excluding) in temporary storage into original storage.
    fn restore(&mut self, i: i32, j: i32);
}

pub struct MergeSorter<T>
where
    T: Sorter + StableMSBRadixSorterBase,
{
    pub(crate) delegate_sorter: T,
    pub(crate) pivot_index: i32,
}

impl<T> MergeSorter<T>
where
    T: Sorter + StableMSBRadixSorterBase,
{
    fn merge_sort(&mut self, from: i32, to: i32) -> Result<(), LuceneError> {
        if to - from < Self::BINARY_SORT_THRESHOLD {
            self.binary_sort(from, to)
        } else {
            let mid = (from + to) / 2;
            self.merge_sort(from, mid)?;
            self.merge_sort(mid, to)?;
            self.merge(from, to, mid)
        }
    }
    /// We tried to expose this to implementations to get a bulk copy optimization.
    /// However, it did not bring a noticeable improvement in benchmarks as `len` is usually small.
    fn bulk_save(&mut self, from: i32, tmp_from: i32, len: i32) {
        for i in 0..len {
            self.delegate_sorter.save(from + i, tmp_from + i);
        }
    }
    fn merge(&mut self, from: i32, to: i32, mid: i32) -> Result<(), LuceneError> {
        debug_assert!(
            to > mid && mid > from,
            "Invalid indices: to={}, mid={}, from={}",
            to,
            mid,
            from
        );
        // If already sorted, return early
        if self.delegate_sorter.compare(mid - 1, mid)? <= 0 {
            return Ok(());
        }
        let mut left = from;
        let mut right = mid;
        let mut index = from;
        loop {
            let cmp = self.delegate_sorter.compare(left, right)?;

            if cmp <= 0 {
                self.delegate_sorter.save(left, index);
                left += 1;
                index += 1;

                if left == mid {
                    debug_assert_eq!(
                        index, right,
                        "Index mismatch: index={}, right={}",
                        index, right
                    );
                    self.bulk_save(right, index, to - right);
                    break;
                }
            } else {
                self.delegate_sorter.save(right, index);
                right += 1;
                index += 1;

                if right == to {
                    debug_assert_eq!(
                        to - index,
                        mid - left,
                        "Range mismatch: to-index={}, mid-left={}",
                        to - index,
                        mid - left
                    );
                    self.bulk_save(left, index, mid - left);
                    break;
                }
            }
        }
        self.delegate_sorter.restore(from, to);
        Ok(())
    }
}
impl<T> Sorter for MergeSorter<T>
where
    T: Sorter + StableMSBRadixSorterBase,
{
    fn compare(&mut self, i: i32, j: i32) -> Result<i32, LuceneError> {
        self.delegate_sorter.compare(i, j)
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        self.delegate_sorter.swap(i, j)
    }

    fn set_pivot(&mut self, i: i32) -> Result<(), LuceneError> {
        self.pivot_index = i;
        Ok(())
    }

    fn compare_pivot(&mut self, i: i32) -> Result<i32, LuceneError> {
        self.compare(self.pivot_index, i)
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<(), LuceneError> {
        check_range(from, to)?;
        self.merge_sort(from, to)?;
        Ok(())
    }
}

pub struct MergeSorterImpl<'a, T>
where
    T: Sorter + MSBRadixSorterBase + StableMSBRadixSorterBase,
{
    k: i32,
    max_length: i32,
    delegate_sorter: &'a mut T,
}
impl<'a, T> MergeSorterImpl<'a, T>
where
    T: Sorter + MSBRadixSorterBase + StableMSBRadixSorterBase,
{
    pub fn new(k: i32, max_length: i32, delegate_sorter: &'a mut T) -> MergeSorterImpl<'a, T>
    where
        T: Sorter + StableMSBRadixSorterBase,
    {
        MergeSorterImpl {
            k,
            max_length,
            delegate_sorter,
        }
    }
}
impl<T> Sorter for MergeSorterImpl<'_, T>
where
    T: Sorter + MSBRadixSorterBase + StableMSBRadixSorterBase,
{
    fn compare(&mut self, i: i32, j: i32) -> Result<i32, LuceneError> {
        for o in self.k..self.max_length {
            let b1 = self.delegate_sorter.byte_at(i, o)?;
            let b2 = self.delegate_sorter.byte_at(j, o)?;
            if b1 != b2 {
                return Ok(b1 - b2);
            } else if b1 == -1 {
                break;
            }
        }
        Ok(0)
    }
    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        self.delegate_sorter.swap(i, j)
    }
}

impl<T> MSBRadixSorterBase for MergeSorterImpl<'_, T> where
    T: MSBRadixSorterBase + Sorter + StableMSBRadixSorterBase
{
}

impl<T> StableMSBRadixSorterBase for MergeSorterImpl<'_, T>
where
    T: Sorter + MSBRadixSorterBase + StableMSBRadixSorterBase,
{
    fn save(&mut self, i: i32, j: i32) {
        self.delegate_sorter.save(i, j);
    }

    fn restore(&mut self, i: i32, j: i32) {
        self.delegate_sorter.restore(i, j);
    }
}

#[cfg(test)]
mod tests {
    use crate::index::{BytesRef, BytesRefBuilder};

    use rand::rngs::StdRng;
    use rand::{Rng, RngCore};

    use crate::test::util::common_method::assert_vecs_equal;
    use crate::test::util::lucene_test_case::{at_least, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::LuceneError;
    use crate::util::stable_msb_radix_sorter::{StableMSBRadixSorter, StableMSBRadixSorterBase};
    use crate::util::{MSBRadixSorter, MSBRadixSorterBase, Sorter};
    use std::collections::HashSet;

    #[allow(dead_code)] // for quick search
    struct TestStableMSBRadixSorter;

    fn test(refs: &[BytesRef], len: usize, random: &mut StdRng) -> Result<(), LuceneError> {
        let mut expected: Vec<BytesRef> = refs[..len].to_vec();
        expected.sort();

        let mut max_length = 0;
        for ref_item in &refs[..len] {
            max_length = max_length.max(ref_item.length);
        }

        match random.random_range(0..3) {
            0 => max_length += TestUtil::next_int(random, 1, 5),
            1 => max_length = i32::MAX,
            _ => {}
        }

        let final_max_length = max_length;
        let mut actual = refs[..len].to_vec();
        let delegate_sorter = StableMSBRadixSorterTestImpl::new(final_max_length, &mut actual);
        let stable_msb_radix_sorter = StableMSBRadixSorter::new(delegate_sorter, final_max_length);
        let mut msb_radix_sorter = MSBRadixSorter::new(max_length, stable_msb_radix_sorter);
        msb_radix_sorter.sort(0, len as i32)?;

        assert_vecs_equal(&expected, &actual);
        Ok(())
    }
    #[test]
    fn test_empty() -> Result<(), LuceneError> {
        let mut random = random();
        let refs: Vec<BytesRef> = vec![BytesRef::default(); random.random_range(0..5)];
        test(&refs, 0, &mut random)
    }
    #[test]
    fn test_one_value() -> Result<(), LuceneError> {
        let mut random = random();
        let bytes = BytesRef::from_string(&TestUtil::random_simple_string(&mut random));
        let refs = vec![bytes];
        test(&refs, 1, &mut random)
    }

    #[test]
    fn test_two_values() -> Result<(), LuceneError> {
        let mut random = random();
        let bytes1 = BytesRef::from_string(&TestUtil::random_simple_string(&mut random));
        let bytes2 = BytesRef::from_string(&TestUtil::random_simple_string(&mut random));
        let refs = vec![bytes1, bytes2];
        test(&refs, 2, &mut random)
    }

    fn test_random_impl(
        common_prefix_len: usize,
        max_len: usize,
        random: &mut StdRng,
    ) -> Result<(), LuceneError> {
        let mut common_prefix = vec![0u8; common_prefix_len];
        random.fill_bytes(&mut common_prefix);
        let len = random.random_range(0..100_000);
        let mut bytes: Vec<BytesRef> = Vec::with_capacity(len + random.random_range(0..50));
        for _ in 0..len {
            let mut b = vec![0u8; common_prefix_len + random.random_range(0..max_len)];
            random.fill_bytes(&mut b[common_prefix_len..]);
            b[..common_prefix_len].copy_from_slice(&common_prefix);
            bytes.push(BytesRef::from_bytes(b));
        }
        test(&bytes, len, random)
    }

    #[test]
    fn test_random() -> Result<(), LuceneError> {
        let mut random = random();
        for _ in 0..10 {
            test_random_impl(0, 10, &mut random)?;
        }
        Ok(())
    }

    #[test]
    fn test_random_with_lots_of_duplicates() -> Result<(), LuceneError> {
        let mut random = random();
        for _ in 0..10 {
            test_random_impl(0, 2, &mut random)?;
        }
        Ok(())
    }

    #[test]
    fn test_random_with_shared_prefix() -> Result<(), LuceneError> {
        let mut random = random();
        for _ in 0..10 {
            let common_prefix_len = TestUtil::next_int(&mut random, 1, 30);
            test_random_impl(common_prefix_len as usize, 10, &mut random)?;
        }
        Ok(())
    }

    #[test]
    fn test_random_with_shared_prefix_and_lots_of_duplicates() -> Result<(), LuceneError> {
        let mut random = random();
        for _ in 0..10 {
            let common_prefix_len = TestUtil::next_int(&mut random, 1, 30);
            test_random_impl(common_prefix_len as usize, 2, &mut random)?;
        }
        Ok(())
    }

    #[test]
    fn test_random2() -> Result<(), LuceneError> {
        let mut random = random();
        // how large our alphabet is
        let letter_count = TestUtil::next_int(&mut random, 2, 10);

        // how many substring fragments to use
        let substring_count = TestUtil::next_int(&mut random, 2, 10) as usize;
        let mut substrings_set = HashSet::new();

        // how many strings to make
        let string_count = at_least(&mut random, 10000) as usize;

        // Generate substring fragments
        while substrings_set.len() < substring_count {
            let length = TestUtil::next_int(&mut random, 2, 10) as usize;
            let mut bytes = vec![0u8; length];
            for byte in &mut bytes {
                *byte = random.random_range(0..letter_count) as u8;
            }
            substrings_set.insert(BytesRef::from_bytes(bytes));
        }

        let substrings: Vec<BytesRef> = substrings_set.into_iter().collect();
        let mut chance: Vec<f64> = Vec::with_capacity(substrings.len());
        let mut sum = 0.0;

        // Generate random chances
        for _ in &substrings {
            let value = random.random::<f64>();
            chance.push(value);
            sum += value;
        }

        // give each substring a random chance of occurring:
        let mut accum = 0.0;
        for value in &mut chance {
            accum += *value / sum;
            *value = accum;
        }

        let mut strings_set = HashSet::new();
        let mut iters = 0;

        while strings_set.len() < string_count && iters < string_count * 5 {
            let count = TestUtil::next_int(&mut random, 1, 5);
            let mut builder = BytesRefBuilder::new();

            for _ in 0..count {
                let v = random.random::<f64>();
                let mut accum = 0.0;
                for (j, substring) in substrings.iter().enumerate() {
                    accum += chance[j];
                    if accum >= v {
                        builder.append_ref(substring);
                        break;
                    }
                }
            }

            let br = builder.get_bytes_ref();
            strings_set.insert(br);
            iters += 1;
        }

        let strings_vec: Vec<BytesRef> = strings_set.into_iter().collect();
        test(&strings_vec, strings_vec.len(), &mut random)
    }

    struct StableMSBRadixSorterTestImpl<'a> {
        temp: Vec<BytesRef>,
        final_max_length: i32,
        refs: &'a mut [BytesRef],
    }
    impl<'a> StableMSBRadixSorterTestImpl<'a> {
        fn new(final_max_length: i32, refs: &'a mut Vec<BytesRef>) -> Self {
            StableMSBRadixSorterTestImpl {
                temp: vec![BytesRef::default(); refs.len()],
                final_max_length,
                refs,
            }
        }
    }

    impl Sorter for StableMSBRadixSorterTestImpl<'_> {
        fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
            self.refs.swap(i as usize, j as usize);
            Ok(())
        }
    }

    impl MSBRadixSorterBase for StableMSBRadixSorterTestImpl<'_> {
        fn byte_at(&mut self, i: i32, k: i32) -> Result<i32, LuceneError> {
            assert!(k < self.final_max_length, "k is out of bounds");
            let ref_item = &self.refs[i as usize];

            if ref_item.length <= k {
                return Ok(-1);
            }

            Ok(ref_item.bytes[ref_item.offset as usize + k as usize] as i32)
        }
    }
    impl StableMSBRadixSorterBase for StableMSBRadixSorterTestImpl<'_> {
        fn save(&mut self, i: i32, j: i32) {
            self.temp[j as usize] = self.refs[i as usize].clone();
        }

        fn restore(&mut self, i: i32, j: i32) {
            for idx in i..j {
                self.refs[idx as usize] = self.temp[idx as usize].clone();
            }
        }
    }
}

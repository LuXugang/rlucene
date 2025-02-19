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
pub trait IndexSorter {
    fn get_provider_name(&self) -> &str;
}

/// Sorts documents based on double values from a NumericDocValues instance.
pub struct DoubleSorter {
    pub(crate) provider_name: String,
}
impl IndexSorter for DoubleSorter {
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }
}

/// Sorts documents based on integer values from a NumericDocValues instance */
pub struct IntSorter {
    pub(crate) provider_name: String,
}
impl IndexSorter for IntSorter {
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }
}

/// Sorts documents based on long values from a NumericDocValues instance
pub struct LongSorter {
    pub(crate) provider_name: String,
}
impl IndexSorter for LongSorter {
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }
}

/// Sorts documents based on float values from a NumericDocValues instance
pub struct FloatSorter {
    pub(crate) provider_name: String,
}
impl IndexSorter for FloatSorter {
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }
}

/// Sorts documents based on short values from a NumericDocValues instance
pub struct StringSorter {
    pub(crate) provider_name: String,
}
impl IndexSorter for StringSorter {
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }
}

pub enum IndexSortEnum {
    DoubleSorter(DoubleSorter),
    IntSorter(IntSorter),
    LongSorter(LongSorter),
    FloatSorter(FloatSorter),
    StringSorter(StringSorter),
}
impl IndexSorter for IndexSortEnum {
    fn get_provider_name(&self) -> &str {
        match self {
            IndexSortEnum::DoubleSorter(sorter) => sorter.get_provider_name(),
            IndexSortEnum::IntSorter(sorter) => sorter.get_provider_name(),
            IndexSortEnum::LongSorter(sorter) => sorter.get_provider_name(),
            IndexSortEnum::FloatSorter(sorter) => sorter.get_provider_name(),
            IndexSortEnum::StringSorter(sorter) => sorter.get_provider_name(),
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::index::{BytesRef, BytesRefBuilder};
    use crate::test::util::common_method::assert_vecs_equal;

    use crate::util::bytes_ref_comparator::{BytesRefComparator, Natural};
    use rand::rngs::StdRng;
    use rand::{Rng, RngCore};

    use crate::test::util::lucene_test_case::{at_least, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::LuceneError;
    use crate::util::stable_string_sorter::{StableStringSorter, StableStringSorterBase};
    use crate::util::{
        Comparator, MSBRadixSorterBase, NaturalOrder, Sorter, StringSorter, StringSorterBase,
    };

    #[allow(dead_code)] // for quick search
    struct TestStringSorter;

    fn test(refs: Vec<BytesRef>, len: usize) -> Result<(), LuceneError> {
        test_impl(refs.clone(), len, Natural::default())?;
        test_impl(refs.clone(), len, NaturalOrder::default())?;
        test_stable(refs.clone(), len, Natural::default())?;
        test_stable(refs.clone(), len, NaturalOrder::default())?;
        Ok(())
    }

    fn test_impl(
        refs: Vec<BytesRef>,
        len: usize,
        comparator: impl BytesRefComparator + Comparator<BytesRef>,
    ) -> Result<(), LuceneError> {
        let mut expected: Vec<BytesRef> = refs.clone();
        expected.sort();
        let delegate_sorter = StringSorterTestImpl::new(refs.clone());
        let mut string_sorter = StringSorter::new(delegate_sorter, comparator);
        string_sorter.sort(0, len as i32)?;

        assert_vecs_equal(&expected, &string_sorter.get_delegate_sorter().refs);
        Ok(())
    }

    fn test_stable(
        refs: Vec<BytesRef>,
        len: usize,
        comparator: impl BytesRefComparator + Comparator<BytesRef>,
    ) -> Result<(), LuceneError> {
        let mut expected: Vec<BytesRef> = refs[..len].to_vec();
        let mut actual = refs[..len].to_vec();
        expected.sort();

        let actual_before_sorted = actual.clone();
        let mut ord: Vec<i32> = (0..len).map(|i| i as i32).collect();
        let ord_len = ord.len();
        let delegate_sorter = StableStringSorterTestImpl {
            tmp: vec![0; ord_len],
            ord: &mut ord,
            refs: &mut actual,
        };
        let string_sorter = StableStringSorter::new(delegate_sorter);
        let mut stable_string_sorter = StringSorter::new(string_sorter, comparator);
        stable_string_sorter.sort(0, len as i32)?;
        // `actual` is not sorted, but `ord` is sorted
        assert_vecs_equal(&actual_before_sorted, &actual);
        for i in 0..len {
            assert_eq!(
                &expected[i], &refs[ord[i] as usize],
                "Mismatch at index {}: expected {:?}, found {:?}",
                i, &expected[i], &refs[ord[i] as usize]
            );

            if i > 0 && expected[i] == expected[i - 1] {
                assert!(
                    ord[i] > ord[i - 1],
                    "Not stable: ord[{}] <= ord[{}]",
                    i,
                    i - 1
                );
            }
        }

        Ok(())
    }

    #[test]
    fn test_empty() -> Result<(), LuceneError> {
        let mut random = random();
        let len = random.random_range(0..5);
        let refs: Vec<BytesRef> = (0..len).map(|_| BytesRef::default()).collect();
        test(refs, 0)
    }

    #[test]
    fn test_one_value() -> Result<(), LuceneError> {
        let mut random = random();
        let bytes = BytesRef::from_string(&TestUtil::random_simple_string(&mut random));
        test(vec![bytes], 1)
    }

    #[test]
    fn test_two_values() -> Result<(), LuceneError> {
        let mut random = random();
        let bytes1 = BytesRef::from_string(&TestUtil::random_simple_string(&mut random));
        let bytes2 = BytesRef::from_string(&TestUtil::random_simple_string(&mut random));
        test(vec![bytes1, bytes2], 2)
    }

    fn test_random_impl(
        common_prefix_len: usize,
        max_len: usize,
        random: &mut StdRng,
    ) -> Result<(), LuceneError> {
        let mut common_prefix = vec![0u8; common_prefix_len];
        random.fill_bytes(&mut common_prefix);
        let len = random.random_range(0..100000);

        let mut bytes: Vec<BytesRef> = Vec::with_capacity(len + random.random_range(0..50));
        for _ in 0..len {
            let mut b = vec![0u8; common_prefix_len + random.random_range(0..max_len)];
            random.fill_bytes(&mut b[common_prefix_len..]);
            b[..common_prefix_len].copy_from_slice(&common_prefix);
            bytes.push(BytesRef::from_bytes(b));
        }

        test(bytes, len)
    }
    #[test]
    fn test_random() -> Result<(), LuceneError> {
        let mut random = random();
        let num_iters = at_least(&mut random, 3) as i32;
        for _ in 0..num_iters {
            test_random_impl(0, 10, &mut random)?;
        }
        Ok(())
    }
    #[test]
    fn test_random_with_lots_of_duplicates() -> Result<(), LuceneError> {
        let mut random = random();
        let num_iters = at_least(&mut random, 3) as i32;
        for _ in 0..num_iters {
            test_random_impl(0, 2, &mut random)?;
        }
        Ok(())
    }
    #[test]
    fn test_random_with_shared_prefix() -> Result<(), LuceneError> {
        let mut random = random();
        let num_iters = at_least(&mut random, 3) as i32;
        for _ in 0..num_iters {
            let shared_prefix_len = TestUtil::next_int(&mut random, 1, 30) as usize;
            test_random_impl(shared_prefix_len, 10, &mut random)?;
        }
        Ok(())
    }
    #[test]
    fn test_random_with_shared_prefix_and_lots_of_duplicates() -> Result<(), LuceneError> {
        let mut random = random();
        let num_iters = at_least(&mut random, 3) as i32;
        for _ in 0..num_iters {
            let shared_prefix_len = TestUtil::next_int(&mut random, 1, 30) as usize;
            test_random_impl(shared_prefix_len, 2, &mut random)?;
        }
        Ok(())
    }

    struct StringSorterTestImpl {
        refs: Vec<BytesRef>,
    }

    impl StringSorterTestImpl {
        fn new(refs: Vec<BytesRef>) -> Self {
            Self { refs }
        }
    }
    impl Sorter for StringSorterTestImpl {
        fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
            self.refs.swap(i as usize, j as usize);
            Ok(())
        }
    }
    impl StringSorterBase for StringSorterTestImpl {
        fn get(
            &mut self,
            _builder: &mut BytesRefBuilder,
            result: &mut BytesRef,
            i: i32,
        ) -> Result<(), LuceneError> {
            let ref_item = &self.refs[i as usize];
            result.offset = ref_item.offset;
            result.length = ref_item.length;
            result.bytes = ref_item.bytes.clone();
            Ok(())
        }
    }

    struct StableStringSorterTestImpl<'a> {
        tmp: Vec<i32>,
        ord: &'a mut Vec<i32>,
        refs: &'a mut [BytesRef],
    }

    impl StringSorterBase for StableStringSorterTestImpl<'_> {
        fn get(
            &mut self,
            _builder: &mut BytesRefBuilder,
            result: &mut BytesRef,
            i: i32,
        ) -> Result<(), LuceneError> {
            let ref_item = &self.refs[self.ord[i as usize] as usize];
            result.offset = ref_item.offset;
            result.length = ref_item.length;
            result.bytes = ref_item.bytes.clone();
            Ok(())
        }
    }

    impl StableStringSorterBase for StableStringSorterTestImpl<'_> {
        fn save(&mut self, i: i32, j: i32) {
            self.tmp[j as usize] = self.ord[i as usize];
        }

        fn restore(&mut self, i: i32, j: i32) {
            self.ord[i as usize..j as usize].copy_from_slice(&self.tmp[i as usize..j as usize]);
        }
    }
    impl Sorter for StableStringSorterTestImpl<'_> {
        fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
            self.ord.swap(i as usize, j as usize);
            Ok(())
        }
    }
    impl MSBRadixSorterBase for StableStringSorterTestImpl<'_> {}
}

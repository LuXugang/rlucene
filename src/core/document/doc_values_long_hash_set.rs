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
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::packed::PackedInts;
use crate::core::util::{CoreHelper, TryIntoInt};
#[cfg(test)]
use std::collections::HashSet;
use std::fmt;

const MISSING: i64 = i64::MIN;
/// Set of longs, optimized for docvalues usage
#[derive(PartialEq, Eq, Hash, Debug)]
pub(crate) struct DocValuesLongHashSet {
    pub(crate) table: Vec<i64>,
    pub(crate) mask: i32,
    pub(crate) has_missing_value: bool,
    pub(crate) size: i32,
    /// minimum value in the set, or Long.MAX_VALUE for an empty set
    pub(crate) min_value: i64,
    /// maximum value in the set, or Long.MIN_VALUE for an empty set
    pub(crate) max_value: i64,
}
impl DocValuesLongHashSet {
    /// Construct a set. Values must be in sorted order.
    pub(crate) fn new(values: &[i64]) -> Result<Self> {
        let mut table_size: i32 = (values.len() as i64 * 3 / 2).try_convert()?;
        let bits = PackedInts::bits_required(table_size as i64)?; // make it a power of 2
        table_size = 1i32 << bits;
        debug_assert!(table_size as usize >= (values.len() * 3 / 2));
        let mut table = vec![MISSING; table_size as usize];
        let mask = table_size - 1;
        let mut has_missing_value = false;
        let mut size = 0;
        let mut previous_value = i64::MIN;
        for &value in values {
            if value == MISSING {
                if !has_missing_value {
                    size += 1;
                    has_missing_value = true;
                }
            } else if Self::add(&mut table, mask, value) {
                size += 1;
            }

            debug_assert!(
                value >= previous_value,
                "values must be provided in sorted order"
            );
            previous_value = value;
        }

        let (min_value, max_value) = if values.is_empty() {
            (i64::MAX, i64::MIN)
        } else {
            (values[0], values[values.len() - 1])
        };

        Ok(Self {
            table,
            mask,
            has_missing_value,
            size,
            min_value,
            max_value,
        })
    }
    fn add(table: &mut [i64], mask: i32, l: i64) -> bool {
        debug_assert!(l != MISSING);
        let hash = (CoreHelper::calculate_hash(&l) & mask as u64) as usize;
        let mut i = hash;

        loop {
            let v = table[i];
            if v == MISSING {
                table[i] = l;
                return true;
            } else if v == l {
                return false;
            }
            i = (i + 1) & mask as usize;
        }
    }
    /// check for membership in the set.
    /// You should use minValue and maxValue to guide/terminate iteration before calling this.
    pub(crate) fn contains(&self, l: i64) -> bool {
        if l == MISSING {
            return self.has_missing_value;
        }

        let hash = CoreHelper::calculate_hash(&l) & self.mask as u64;
        let mut i = hash as usize;

        loop {
            let v = self.table[i];
            if v == MISSING {
                return false;
            } else if v == l {
                return true;
            }
            i = (i + 1) & self.mask as usize;
        }
    }
    /// number of elements in the set
    pub(crate) fn size(&self) -> i32 {
        self.size
    }
    /// returns a stream of all values contained in this set
    #[cfg(test)]
    pub(crate) fn stream(&self) -> HashSet<i64> {
        let mut set = HashSet::with_capacity(self.size as usize);
        if self.has_missing_value {
            set.insert(MISSING);
        }
        for &v in &self.table {
            if v != MISSING {
                set.insert(v);
            }
        }
        set
    }
}
impl Accountable for DocValuesLongHashSet {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}
impl fmt::Display for DocValuesLongHashSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;

        write!(f, "[")?;

        if self.has_missing_value {
            write!(f, "{}", MISSING)?;
            first = false;
        }

        for &v in &self.table {
            if v != MISSING {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "{}", v)?;
                first = false;
            }
        }

        write!(f, "]")
    }
}
#[cfg(test)]
mod tests {
    use crate::core::document::doc_values_long_hash_set::DocValuesLongHashSet;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{at_least, random};
    use rand::Rng;
    use rand::RngExt;
    use std::collections::HashSet;

    fn assert_eq_set<R: Rng + ?Sized>(
        random: &mut R,
        set1: &HashSet<i64>,
        long_hash_set: &DocValuesLongHashSet,
    ) {
        assert_eq!(set1.len() as i32, long_hash_set.size);
        let set2 = long_hash_set.stream();
        assert_eq!(set1, &set2);
        if !set1.is_empty() {
            let mut set3 = set1.clone();
            let removed = *set3.iter().next().unwrap();
            loop {
                let next = random.random();

                if next != removed && set3.insert(next) {
                    assert!(!long_hash_set.contains(next));
                    break;
                }
            }
            assert_ne!(set3, long_hash_set.stream());
        }
        assert!(set1.iter().all(|v| long_hash_set.contains(*v)));
    }
    fn assert_not_eq_set(set1: &HashSet<i64>, long_hash_set: &DocValuesLongHashSet) {
        let set2 = long_hash_set.stream();
        assert_ne!(set1, &set2);
        let mut sorted: Vec<i64> = set1.iter().copied().collect();
        sorted.sort_unstable();
        let set3 = DocValuesLongHashSet::new(&sorted)
            .expect("DocValuesLongHashSet construction must succeed");
        let set3_stream = set3.stream();
        assert_ne!(set2, set3_stream);
        assert!(!set1.iter().all(|v| long_hash_set.contains(*v)));
    }
    #[test]
    fn test_empty() -> Result<()> {
        let mut random = random();
        let set1 = HashSet::new();
        let set2 = DocValuesLongHashSet::new(&[])?;
        assert_eq!(set2.size, 0);
        assert_eq!(set2.min_value, i64::MAX);
        assert_eq!(set2.max_value, i64::MIN);
        assert_eq_set(&mut random, &set1, &set2);
        Ok(())
    }
    #[test]
    fn test_one_value() -> Result<()> {
        let mut random = random();

        let set1 = [42_i64].into_iter().collect();
        let set2 = DocValuesLongHashSet::new(&[42_i64])?;

        assert_eq!(set2.size, 1);
        assert_eq!(set2.min_value, 42);
        assert_eq!(set2.max_value, 42);

        assert_eq_set(&mut random, &set1, &set2);

        let set1 = [i64::MIN].into_iter().collect();
        let set2 = DocValuesLongHashSet::new(&[i64::MIN])?;

        assert_eq!(set2.size, 1);
        assert_eq!(set2.min_value, i64::MIN);
        assert_eq!(set2.max_value, i64::MIN);

        assert_eq_set(&mut random, &set1, &set2);

        Ok(())
    }
    #[test]
    fn test_two_values() -> Result<()> {
        let mut random = random();

        let set1 = [42_i64, i64::MAX].into_iter().collect();
        let set2 = DocValuesLongHashSet::new(&[42_i64, i64::MAX])?;
        assert_eq!(set2.size, 2);
        assert_eq!(set2.min_value, 42);
        assert_eq!(set2.max_value, i64::MAX);
        assert_eq_set(&mut random, &set1, &set2);

        let set1 = [i64::MIN, 42_i64].into_iter().collect();
        let set2 = DocValuesLongHashSet::new(&[i64::MIN, 42_i64])?;
        assert_eq!(set2.size, 2);
        assert_eq!(set2.min_value, i64::MIN);
        assert_eq!(set2.max_value, 42);
        assert_eq_set(&mut random, &set1, &set2);

        Ok(())
    }

    #[test]
    fn test_same_value() -> Result<()> {
        let set2 = DocValuesLongHashSet::new(&[42_i64, 42_i64])?;
        assert_eq!(set2.size, 1);
        assert_eq!(set2.min_value, 42);
        assert_eq!(set2.max_value, 42);
        Ok(())
    }

    #[test]
    fn test_same_missing_placeholder() -> Result<()> {
        let set2 = DocValuesLongHashSet::new(&[i64::MIN, i64::MIN])?;
        assert_eq!(set2.size, 1);
        assert_eq!(set2.min_value, i64::MIN);
        assert_eq!(set2.max_value, i64::MIN);
        Ok(())
    }

    #[test]
    fn test_random() -> Result<()> {
        let mut random = random();
        let iters = at_least(&mut random, 10);

        for _ in 0..iters {
            let v = random.random_range(0..16);
            let len = random.random_range(0..(1 << v));
            let mut values = vec![0_i64; len];

            for i in 0..len {
                if i == 0 || random.random_range(0..10) < 9 {
                    values[i] = random.random();
                } else {
                    let idx = random.random_range(0..i);
                    values[i] = values[idx];
                }
            }

            if len > 0 && random.random_bool(0.5) {
                values[len / 2] = i64::MIN;
            }
            let set1 = values.iter().copied().collect();
            values.sort_unstable();
            let set2 = DocValuesLongHashSet::new(&values)?;
            assert_eq_set(&mut random, &set1, &set2);
        }
        Ok(())
    }
}

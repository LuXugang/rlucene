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
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{Comparator, ToInt};

/// Represents a slice (offset + length) into an existing `Vec<u64>`.
///
/// The `longs` member should never be `None`; use an empty vector
/// (`Vec::new()`) if necessary.
#[derive(Debug, Eq)]
pub struct LongsRef {
    /// The contents of the LongsRef. Should never be `None`.
    pub longs: Vec<i64>,

    /// Offset of the first valid long.
    pub offset: i32,

    /// Length of used longs.
    pub length: i32,
}

impl Default for LongsRef {
    fn default() -> Self {
        Self::new()
    }
}

impl LongsRef {
    /// Create a `LongsRef` with an empty vector.
    pub fn new() -> Self {
        Self {
            longs: Vec::new(),
            offset: 0,
            length: 0,
        }
    }

    /// Create a `LongsRef` pointing to a new vector of the given capacity.
    ///
    /// Offset and length will both be zero.
    pub fn with_capacity(capacity: i32) -> Self {
        Self {
            longs: vec![0; capacity as usize],
            offset: 0,
            length: 0,
        }
    }

    /// This instance will directly reference the given `Vec<u64>` without
    /// making a copy.
    ///
    /// # Arguments
    ///
    /// * `longs` - The vector to reference. Should not be empty.
    /// * `offset` - The offset where valid longs start.
    /// * `length` - The number of valid longs.
    pub fn from_slice(mut longs: Vec<i64>, offset: i32, length: i32) -> Self {
        debug_assert!(Self::is_valid(longs.as_mut_slice(), offset, length).unwrap());
        Self {
            longs,
            offset,
            length,
        }
    }
    /// Creates a new `LongsRef` that points to a copy of the longs from
    /// `other`.
    ///
    /// The returned `LongsRef` will have a length of `other.length` and an
    /// offset of zero.
    ///
    /// # Arguments
    ///
    /// * `other` - The `LongsRef` to copy.
    ///
    /// # Returns
    ///
    /// A new `LongsRef` that is a deep copy of the provided `other`.
    pub fn deep_copy_of(other: &LongsRef) -> Result<LongsRef> {
        if (other.offset + other.length) as usize > other.longs.len() {
            return Err(LuceneError::array_index_out_of_bounds(
                "Offset and length exceed vector bounds",
            ));
        }
        let copied_longs =
            other.longs[other.offset as usize..(other.offset + other.length) as usize].to_vec();

        Ok(LongsRef {
            longs: copied_longs,
            offset: 0,
            length: other.length,
        })
    }

    pub fn is_valid(longs: &[i64], offset: i32, length: i32) -> Result<bool> {
        if longs.is_empty() {
            return Err(LuceneError::illegal_state("longs is empty"));
        }
        if length < 0 {
            return Err(LuceneError::illegal_state(format!(
                "length is negative: {length}"
            )));
        }

        if length as usize > longs.len() {
            return Err(LuceneError::illegal_state(format!(
                "length is out of bounds: {}, longs.len={}",
                length,
                longs.len()
            )));
        }
        if offset < 0 {
            return Err(LuceneError::illegal_state(format!(
                "offset is negative: {offset}"
            )));
        }

        if offset as usize > longs.len() {
            return Err(LuceneError::illegal_state(format!(
                "offset is out of bounds: {}, longs.len={}",
                offset,
                longs.len()
            )));
        }
        if offset + length < 0 {
            return Err(LuceneError::illegal_state(format!(
                "offset + length is negative: offset={offset}, length={length}"
            )));
        }
        if (offset + length) as usize > longs.len() {
            return Err(LuceneError::illegal_state(format!(
                "offset + length out of bounds: offset={}, length={}, longs.len={}",
                offset,
                length,
                longs.len()
            )));
        }
        Ok(true)
    }
    pub fn longs_equals(&self, other: &LongsRef) -> bool {
        debug_assert!(
            (self.offset + self.length) as usize <= self.longs.len()
                && (other.offset + other.length) as usize <= other.longs.len()
        );
        self.longs[self.offset as usize..(self.offset + self.length) as usize]
            == other.longs[other.offset as usize..(other.offset + other.length) as usize]
    }
}

impl Clone for LongsRef {
    fn clone(&self) -> Self {
        Self {
            longs: self.longs.clone(),
            offset: self.offset,
            length: self.length,
        }
    }
}
impl Hash for LongsRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        const PRIME: u64 = 31;
        let mut result: u64 = 0;
        let end = self.offset + self.length;

        for &value in &self.longs[self.offset as usize..end as usize] {
            result = PRIME
                .wrapping_mul(result)
                .wrapping_add((value ^ (value >> 32)) as u64);
        }

        state.write_u64(result);
    }
}
impl PartialEq for LongsRef {
    fn eq(&self, other: &Self) -> bool {
        self.longs_equals(other)
    }
}
impl Display for LongsRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let end = self.offset + self.length;
        for i in self.offset..end {
            if i > self.offset {
                write!(f, " ")?;
            }
            write!(f, "{:x}", self.longs[i as usize])?;
        }
        write!(f, "]")
    }
}
pub struct LongsRefComparator;
impl Comparator<LongsRef> for LongsRefComparator {
    const TYPE: &'static str = "LongsRefComparator";

    /// Compares two `LongsRef` instances.
    ///
    /// # Arguments
    ///
    /// * `a` - The first `LongsRef` to compare.
    /// * `b` - The second `LongsRef` to compare.
    ///
    /// # Returns
    ///
    /// * A negative integer if `a < b`.
    /// * Zero if `a == b`.
    /// * A positive integer if `a > b`.
    fn compare(&self, a: &LongsRef, b: &LongsRef) -> Result<i32> {
        let a_slice = &a.longs[a.offset as usize..(a.offset + a.length) as usize];
        let b_slice = &b.longs[b.offset as usize..(b.offset + b.length) as usize];
        Ok(a_slice.cmp(b_slice).to_int())
    }
}

#[cfg(test)]
mod tests {

    use crate::core::util::error::lucene_error::{LuceneError, Result};
    use crate::core::util::longs_ref::LongsRef;

    #[allow(dead_code)] // for quick search
    struct TestLongsRef;
    #[test]
    fn test_empty() {
        let i = LongsRef::new();
        assert_eq!(i.longs, LongsRef::new().longs);
        assert_eq!(i.offset, 0);
        assert_eq!(i.length, 0);
    }

    #[test]
    fn test_from_longs() {
        let longs = vec![1, 2, 3, 4];
        let i = LongsRef::from_slice(longs.clone(), 0, 4);
        assert_eq!(i.longs, i.longs);
        assert_eq!(i.offset, 0);
        assert_eq!(i.length, 4);

        let i2 = LongsRef::from_slice(longs.clone(), 1, 3);
        let expected_longs = vec![2, 3, 4];
        let expected = LongsRef::from_slice(expected_longs, 0, 3);
        assert!(i2.eq(&expected));

        assert_ne!(i, i2);
    }

    #[test]
    fn test_invalid_deep_copy() -> Result<()> {
        let mut from = LongsRef::from_slice(vec![1, 2], 0, 2);
        from.offset += 1;
        let result = LongsRef::deep_copy_of(&from);
        assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));
        Ok(())
    }
}

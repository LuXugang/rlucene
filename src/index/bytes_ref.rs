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
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{StringHelper, GOOD_FAST_HASH_SEED};
use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::Hash;

/// Represents a `&[u8]` as a slice (offset + length) into an existing byte array.
/// The `bytes` member should never be `None`;
///
/// # Important Note
/// Unless otherwise noted, this struct is used to represent terms encoded as **UTF-8** bytes in the index.
/// To convert them to a Rust `String` (which is UTF-8), use [`utf8_to_string`](#method.utf8_to_string).
/// Using code like `String::from_utf8_lossy(&bytes[offset..offset+length])` is the correct way to handle this.
/// Avoid constructing strings incorrectly, as it may result in wrong results.
///
/// # Sorting
/// This struct implements `Ord`. The underlying byte arrays are sorted lexicographically, treating elements as unsigned.
/// This is identical to Unicode codepoint order.
#[derive(Debug)]
pub struct BytesRef {
    /// The contents of the BytesRef
    pub bytes: Vec<u8>,
    pub offset: i32,
    pub length: i32,
}
impl Default for BytesRef {
    fn default() -> Self {
        Self::new()
    }
}

impl BytesRef {
    pub fn new() -> BytesRef {
        BytesRef {
            bytes: Vec::new(),
            offset: 0,
            length: 0,
        }
    }
    pub fn from_vec(bytes: Vec<u8>, offset: i32, length: i32) -> BytesRef {
        BytesRef {
            bytes,
            offset,
            length,
        }
    }
    pub fn from_bytes(bytes: Vec<u8>) -> BytesRef {
        debug_assert!(bytes.len() <= i32::MAX as usize);
        let length = bytes.len() as i32;
        BytesRef {
            bytes,
            offset: 0,
            length,
        }
    }
    pub fn with_capacity(capacity: i32) -> BytesRef {
        BytesRef {
            bytes: vec![0; capacity as usize],
            offset: 0,
            length: 0,
        }
    }
    pub fn from_string(s: &str) -> BytesRef {
        debug_assert!(s.len() <= i32::MAX as usize);
        BytesRef {
            bytes: s.as_bytes().to_vec(),
            offset: 0,
            length: s.len() as i32,
        }
    }
    /// Expert: compares the bytes against another BytesRef, returning true if the bytes are equal.
    ///
    /// # Arguments
    /// * `other` - Another BytesRef
    ///
    /// # Note
    /// This is an internal method.
    pub fn bytes_equals(&self, other: &BytesRef) -> bool {
        if self.length == other.length {
            for i in 0..self.length {
                if self.bytes[self.offset as usize + i as usize]
                    != other.bytes[other.offset as usize + i as usize]
                {
                    return false;
                }
            }
            return true;
        }
        false
    }
    /// Interprets the stored bytes as UTF-8, returning the resulting string.
    pub fn utf8_to_string(&self) -> Result<String> {
        std::str::from_utf8(&self.bytes[self.offset as usize..(self.offset + self.length) as usize])
            .map(|s| s.to_owned())
            .map_err(LuceneError::Utf8Error)
    }
    /// Creates a new `BytesRef` that points to a copy of the bytes from `other`.
    ///
    /// The returned `BytesRef` will have a length equal to `other.length` and an offset of zero.
    pub fn deep_copy_of(other: &BytesRef) -> BytesRef {
        let bytes =
            ArrayUtil::copy_of_sub_array(&other.bytes, other.offset, other.offset + other.length);
        BytesRef::from_vec(bytes, 0, other.length)
    }
    /// Performs internal consistency checks. Always returns `true` (or throws `IllegalStateError`).
    pub fn is_valid(&self) -> Result<bool> {
        if self.length as usize > self.bytes.len() {
            return Err(LuceneError::illegal_state(format!(
                "length is out of bounds: {},bytes.length= {}",
                self.length,
                self.bytes.len()
            )));
        }
        if self.offset as usize > self.bytes.len() {
            return Err(LuceneError::illegal_state(format!(
                "offset out of bounds: {},bytes.length= {}",
                self.offset,
                self.bytes.len()
            )));
        }
        if (self.offset + self.length) as usize > self.bytes.len() {
            return Err(LuceneError::illegal_state(format!(
                "offset+length out of bounds: offset={},length={},bytes.length= {}",
                self.offset,
                self.length,
                self.bytes.len()
            )));
        }
        Ok(true)
    }
}
impl PartialOrd for BytesRef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for BytesRef {}

impl Ord for BytesRef {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bytes[self.offset as usize..(self.offset + self.length) as usize]
            .cmp(&other.bytes[other.offset as usize..(other.offset + other.length) as usize])
    }
}

impl Clone for BytesRef {
    fn clone(&self) -> Self {
        BytesRef {
            bytes: self.bytes.clone(),
            offset: self.offset,
            length: self.length,
        }
    }
}
impl Hash for BytesRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let hash = StringHelper::murmurhash3_x86_32(self, *GOOD_FAST_HASH_SEED);
        hash.hash(state)
    }
}
impl PartialEq for BytesRef {
    fn eq(&self, other: &Self) -> bool {
        self.bytes_equals(other)
    }
}
impl Display for BytesRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[")?;
        let end = self.offset + self.length;

        for (i, &byte) in self.bytes[self.offset as usize..end as usize]
            .iter()
            .enumerate()
        {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{:02x}", byte)?;
        }

        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use crate::index::BytesRef;
    use crate::test::util::lucene_test_case::random;
    use crate::test::util::test_util::TestUtil;
    use rand::distr::Alphanumeric;
    use rand::Rng;
    use std::ptr;

    #[allow(dead_code)] // for quick search
    struct TestBytesRef {}

    #[test]
    fn test_empty() {
        let b = BytesRef::new();
        assert_eq!(b.bytes.len(), 0);
        assert_eq!(b.length, 0);
        assert_eq!(b.offset, 0);
    }
    #[test]
    fn test_from_bytes() {
        let mut bytes: Vec<u8> = "abcd".as_bytes().to_vec();
        let b = BytesRef::from_bytes(bytes.clone());
        assert_eq!(bytes, b.bytes);
        assert_eq!(b.length, 4);
        assert_eq!(b.offset, 0);

        bytes = "abcd".as_bytes().to_vec();
        let b2 = BytesRef::from_vec(bytes, 1, 3);
        let b2_value = b2.utf8_to_string();
        assert!(b2_value.is_ok());
        assert_eq!("bcd", b2_value.unwrap());

        assert!(!b.eq(&b2));
    }
    #[test]
    fn test_from_chars() {
        let mut random = random();
        let length = random.random_range(1000..100000);
        for _i in 0..100 {
            let s = (&mut random)
                .sample_iter(&Alphanumeric)
                .take(length)
                .map(char::from)
                .collect::<String>();
            let s2 = BytesRef::from_string(&s).utf8_to_string().unwrap();
            assert_eq!(s, s2);
        }
        let s = TestUtil::random_unicode_string(&mut random);
        let s2 = BytesRef::from_string(&s).utf8_to_string().unwrap();
        assert_eq!(s, s2);
    }

    #[test]
    fn test_deep_copy() {
        let from = BytesRef::from_bytes("abcd".as_bytes().to_vec());
        let copy = BytesRef::deep_copy_of(&from);
        let from_ref = &from;
        assert!(from.eq(&copy));
        assert_ne!(ptr::addr_of!(from), ptr::addr_of!(copy));
        assert!(ptr::addr_of!(from) == from_ref);
    }
}

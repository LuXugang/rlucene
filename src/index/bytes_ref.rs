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
use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::Hash;
use std::rc::Rc;

use crate::util::access::AccessVec;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{StringHelper, GOOD_FAST_HASH_SEED};
use crate::with_other;

/// Represents a `&[u8]` as a slice (offset + length) into an existing byte
/// array. The `bytes` member should never be `None`;
///
/// # Important Note
/// To convert them to a Rust `String` (which is UTF-8), use `utf8_to_string`.
/// Using code like `String::from_utf8_lossy(&bytes[offset.offset+length])` is
/// the correct way to handle this. Avoid constructing strings incorrectly, as
/// it may result in wrong results.
///
/// # Sorting
/// This struct implements `Ord`. The underlying byte arrays are sorted
/// lexicographically, treating elements as unsigned. This is identical to
/// Unicode codepoint order.
#[derive(Debug, Default)]
pub struct BytesRef<AV>
where
    AV: AccessVec<u8>,
{
    /// The contents of the BytesRef
    pub bytes: AV,
    pub offset: usize,
    pub length: usize,
}
impl BytesRef<Rc<Vec<u8>>> {
    /// compare: same bytes reference, same offset, same length
    pub fn equals(a: &BytesRef<Rc<Vec<u8>>>, b: &BytesRef<Rc<Vec<u8>>>) -> bool {
        Rc::ptr_eq(&a.bytes, &b.bytes) && a.offset == b.offset && a.length == b.length
    }
}

impl<AV> BytesRef<AV>
where
    AV: AccessVec<u8>,
{
    pub fn new() -> Self {
        BytesRef {
            bytes: AV::new(),
            offset: 0,
            length: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        BytesRef {
            bytes: AV::with_capacity(capacity),
            offset: 0,
            length: 0,
        }
    }
    pub fn from_slice(bytes: AV, offset: usize, length: usize) -> Self {
        BytesRef {
            bytes,
            offset,
            length,
        }
    }
    /// This instance will directly share/ownership bytes w/o making a copy
    pub fn from_bytes(bytes: AV) -> Self {
        let len = bytes.access(|bytes| bytes.len());
        BytesRef {
            bytes,
            offset: 0,
            length: len,
        }
    }
    /// Initialize the `&[u8]` from the UTF-8 bytes for the provided `String`.
    pub fn from_string(s: &str) -> Self {
        let container = AV::from_vec(s.as_bytes().to_vec());
        let len = s.len();
        BytesRef {
            bytes: container,
            offset: 0,
            length: len,
        }
    }

    /// Expert: compares the bytes against another BytesRef, returning true if
    /// the bytes are equal.
    ///
    /// # Arguments
    /// * `other` - Another BytesRef
    pub fn bytes_equals(&self, other: &BytesRef<AV>) -> bool {
        with_other!(self.bytes, other.bytes, |ints_bytes, other_bytes| {
            let self_slice = &ints_bytes[self.offset..(self.offset + self.length)];
            let other_slice = &other_bytes[other.offset..(other.offset + other.length)];
            self_slice == other_slice
        })
    }
    /// Interprets the stored bytes as UTF-8, returning the resulting string.
    pub fn utf8_to_string(&self) -> Result<String> {
        self.bytes.access(|bytes| {
            std::str::from_utf8(&bytes[self.offset..(self.offset + self.length)])
                .map(|s| s.to_owned())
                .map_err(LuceneError::Utf8Error)
        })
    }
    pub fn deep_copy_of(other: &BytesRef<AV>) -> Self {
        BytesRef::from_slice(
            other.bytes.slice_clone(other.offset, other.length),
            0,
            other.length,
        )
    }
    /// Performs internal consistency checks. Always returns `true` (or throws
    /// `IllegalStateError`).
    pub fn is_valid(&self) -> Result<bool> {
        self.bytes.access(|bytes| {
            if self.length > bytes.len() {
                return Err(LuceneError::illegal_state(format!(
                    "length is out of bounds: {},bytes.length= {}",
                    self.length,
                    bytes.len()
                )));
            }
            if self.offset > bytes.len() {
                return Err(LuceneError::illegal_state(format!(
                    "offset out of bounds: {},bytes.length= {}",
                    self.offset,
                    bytes.len()
                )));
            }
            if (self.offset + self.length) > bytes.len() {
                return Err(LuceneError::illegal_state(format!(
                    "offset+length out of bounds: offset={},length={},bytes.length= {}",
                    self.offset,
                    self.length,
                    bytes.len()
                )));
            }
            // Help the compiler infer types.
            Ok::<(), LuceneError>(())
        })?;
        Ok(true)
    }
}
impl<AV> PartialOrd for BytesRef<AV>
where
    AV: AccessVec<u8>,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<AV> Eq for BytesRef<AV> where AV: AccessVec<u8> {}

impl<AV> Ord for BytesRef<AV>
where
    AV: AccessVec<u8>,
{
    fn cmp(&self, other: &Self) -> Ordering {
        with_other!(self.bytes, other.bytes, |bytes, other_bytes| {
            bytes[self.offset..(self.offset + self.length)]
                .cmp(&other_bytes[other.offset..(other.offset + other.length)])
        })
    }
}

impl<AV> Clone for BytesRef<AV>
where
    AV: AccessVec<u8>,
{
    fn clone(&self) -> Self {
        BytesRef {
            bytes: self.bytes.clone(),
            offset: self.offset,
            length: self.length,
        }
    }
}
impl<AV> Hash for BytesRef<AV>
where
    AV: AccessVec<u8>,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let hash = StringHelper::murmurhash3_x86_32(self, *GOOD_FAST_HASH_SEED);
        hash.hash(state)
    }
}
impl<AV> PartialEq for BytesRef<AV>
where
    AV: AccessVec<u8>,
{
    fn eq(&self, other: &Self) -> bool {
        self.bytes_equals(other)
    }
}
impl<AV> Display for BytesRef<AV>
where
    AV: AccessVec<u8>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.bytes.access(|bytes| {
            write!(f, "[")?;
            let end = self.offset + self.length;

            for (i, &byte) in bytes[self.offset..end].iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{:02x}", byte)?;
            }
            write!(f, "]")?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use rand::distr::Alphanumeric;
    use rand::Rng;

    use crate::index::BytesRef;
    use crate::test::util::lucene_test_case::random;
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::Result;

    #[allow(dead_code)] // for quick search
    struct TestBytesRef {}

    #[test]
    fn test_empty() {
        let b: BytesRef<Vec<u8>> = BytesRef::new();
        assert_eq!(b.bytes.len(), 0);
        assert_eq!(b.length, 0);
        assert_eq!(b.offset, 0);
    }
    #[test]
    fn test_from_bytes() -> Result<()> {
        let mut bytes: Vec<u8> = "abcd".as_bytes().to_vec();
        let b = BytesRef::from_bytes(bytes.clone());
        assert_eq!(bytes, b.bytes);
        assert_eq!(b.length, 4);
        assert_eq!(b.offset, 0);

        bytes = "abcd".as_bytes().to_vec();
        let b2 = BytesRef::from_slice(bytes, 1, 3);
        let b2_value = b2.utf8_to_string()?;
        assert_eq!("bcd", b2_value);

        assert!(!b.eq(&b2));
        Ok(())
    }
    #[test]
    fn test_from_chars() -> Result<()> {
        let mut random = random();
        let length = random.random_range(1000..100000);
        for _i in 0..100 {
            let s = (&mut random)
                .sample_iter(&Alphanumeric)
                .take(length)
                .map(char::from)
                .collect::<String>();
            let s2: String = BytesRef::<Vec<u8>>::from_string(&s).utf8_to_string()?;
            assert_eq!(s, s2);
        }
        let s = TestUtil::random_unicode_string(&mut random);
        let s2 = BytesRef::<Vec<u8>>::from_string(&s).utf8_to_string()?;
        assert_eq!(s, s2);
        Ok(())
    }

    #[test]
    fn test_deep_copy() -> Result<()> {
        let from = BytesRef::from_bytes("abcd".as_bytes().to_vec());
        let copy = BytesRef::deep_copy_of(&from);
        let from_ref = &from;
        assert!(from.eq(&copy));
        assert_ne!(ptr::addr_of!(from), ptr::addr_of!(copy));
        assert!(ptr::addr_of!(from) == from_ref);
        Ok(())
    }
}

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
use crate::core::util::access::SharedAccessVec;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{GOOD_FAST_HASH_SEED, HashCode, StringHelper};
use crate::with_other;
use std::cmp::Ordering;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::sync::Arc;

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
  AV: SharedAccessVec<u8>,
{
  /// The contents of the BytesRef
  pub bytes: AV,
  pub offset: usize,
  pub length: usize,
}
impl BytesRef<Arc<Vec<u8>>> {
  /// compare: same bytes reference, same offset, same length
  pub fn equals(a: &BytesRef<Arc<Vec<u8>>>, b: &BytesRef<Arc<Vec<u8>>>) -> bool {
    let v = Arc::ptr_eq(&a.bytes, &b.bytes);
    // Simulate Java-style reference equality: if the bytes reference is the same,
    // then offset and length must also be equal.
    debug_assert!({
      if v {
        a.offset == b.offset && a.length == b.length
      } else {
        !v
      }
    });
    v
  }
}

impl<AV> BytesRef<AV>
where
  AV: SharedAccessVec<u8>,
{
  pub fn new() -> Self {
    BytesRef {
      bytes: AV::new(),
      offset: 0,
      length: 0,
    }
  }

  pub fn with_capacity(capacity: usize) -> Result<Self> {
    Ok(BytesRef {
      bytes: AV::with_capacity(capacity)?,
      offset: 0,
      length: 0,
    })
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
  pub fn take_bytes(&mut self) -> AV {
    std::mem::take(&mut self.bytes)
  }
}
impl<AV> PartialOrd for BytesRef<AV>
where
  AV: SharedAccessVec<u8>,
{
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl<AV> Eq for BytesRef<AV> where AV: SharedAccessVec<u8> {}

impl<AV> Ord for BytesRef<AV>
where
  AV: SharedAccessVec<u8>,
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
  AV: SharedAccessVec<u8>,
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
  AV: SharedAccessVec<u8>,
{
  fn hash<H>(&self, state: &mut H)
  where
    H: std::hash::Hasher,
  {
    let hash = StringHelper::murmurhash3_x86_32(self, *GOOD_FAST_HASH_SEED);
    hash.hash(state)
  }
}
impl<AV> PartialEq for BytesRef<AV>
where
  AV: SharedAccessVec<u8>,
{
  fn eq(&self, other: &Self) -> bool {
    self.bytes_equals(other)
  }
}
impl<AV> Display for BytesRef<AV>
where
  AV: SharedAccessVec<u8>,
{
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    self.bytes.access(|bytes| {
      write!(f, "[")?;
      let end = self.offset + self.length;

      for (i, &byte) in bytes[self.offset..end].iter().enumerate() {
        if i > 0 {
          write!(f, " ")?;
        }
        write!(f, "{byte:02x}")?;
      }
      write!(f, "]")?;
      Ok(())
    })
  }
}
impl<AV> HashCode for BytesRef<AV>
where
  AV: SharedAccessVec<u8>,
{
  fn hash_code(&self) -> i32 {
    StringHelper::murmurhash3_x86_32(self, *GOOD_FAST_HASH_SEED)
  }
}
impl From<String> for BytesRef<Vec<u8>> {
  fn from(value: String) -> Self {
    BytesRef::from_string(value.as_ref())
  }
}
impl From<&str> for BytesRef<Vec<u8>> {
  fn from(value: &str) -> Self {
    BytesRef::from_string(value)
  }
}

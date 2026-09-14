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
use crate::core::index::BytesRef;
use crate::core::util::access::ByteSource;
use crate::core::util::comparator::Comparator;
use crate::core::util::error::lucene_error::Result;

/// Specialized [`BytesRef`] comparator that [`StringSorter`](crate::core::index::index_sorter::StringSorter) has optimizations
/// for.
///
/// # Note
/// This is an internal API.
pub trait BytesRefComparator<AV = Vec<u8>>: Comparator<BytesRef<AV>> {
  /// Returns the unsigned byte to use for comparison at index `i`, or `-1` if
  /// all bytes that are useful for comparisons are exhausted. This may
  /// only be called with a value of `i` between `0` (inclusive) and
  /// `compared_bytes_count` (exclusive).
  fn byte_at(&self, _bytes_ref: &BytesRef<AV>, _i: usize) -> Result<i32>;
  fn compare_with_offset(&self, o1: &BytesRef<AV>, o2: &BytesRef<AV>, k: usize) -> Result<i32> {
    for i in k..self.compared_bytes_count() {
      let b1 = self.byte_at(o1, i)?;
      let b2 = self.byte_at(o2, i)?;
      if b1 != b2 {
        return Ok(b1 - b2);
      } else if b1 == -1 {
        break;
      }
    }
    Ok(0)
  }
  fn compared_bytes_count(&self) -> usize;
}

pub struct Natural {
  compared_bytes_count: usize,
}
impl Default for Natural {
  fn default() -> Self {
    Natural {
      compared_bytes_count: i32::MAX as usize,
    }
  }
}

impl<AV: ByteSource> Comparator<BytesRef<AV>> for Natural {
  const TYPE: &'static str = BYTES_REF_COMPARATOR_TYPE;

  fn compare(&self, a: &BytesRef<AV>, b: &BytesRef<AV>) -> Result<i32> {
    self.compare_with_offset(a, b, 0)
  }
}

impl<AV: ByteSource> BytesRefComparator<AV> for Natural {
  fn byte_at(&self, bytes_ref: &BytesRef<AV>, i: usize) -> Result<i32> {
    if bytes_ref.length <= i {
      Ok(-1)
    } else {
      Ok(bytes_ref.bytes.as_slice()[i + bytes_ref.offset] as i32)
    }
  }

  fn compare_with_offset(&self, o1: &BytesRef<AV>, o2: &BytesRef<AV>, k: usize) -> Result<i32> {
    let start1 = o1.offset + k;
    let start2 = o2.offset + k;

    let slice1 = &o1.bytes.as_slice()[start1..(o1.offset + o1.length)];
    let slice2 = &o2.bytes.as_slice()[start2..(o2.offset + o2.length)];

    for (byte_a, byte_b) in slice1.iter().zip(slice2.iter()) {
      if byte_a != byte_b {
        return Ok(*byte_a as i32 - *byte_b as i32);
      }
    }
    Ok((slice1.len() as i32) - (slice2.len() as i32))
  }

  fn compared_bytes_count(&self) -> usize {
    self.compared_bytes_count
  }
}

pub const BYTES_REF_COMPARATOR_TYPE: &str = "BytesRefComparator";

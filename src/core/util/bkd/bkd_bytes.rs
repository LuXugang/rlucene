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

use crate::core::util::ToInt;
use crate::core::util::array_util::{
  ByteArrayComparator, U32byteArrayComparator, U64byteArrayComparator,
};
use crate::core::util::bkd::bkd_util::{BKDUtil, ByteArrayPredicate};

/// `N` is 4 or 8 for the fixed-width paths, and 0 for other widths.
#[derive(Clone, Copy)]
pub(crate) struct BKDBytes<const N: usize> {
  num_bytes: usize,
}

impl<const N: usize> BKDBytes<N> {
  pub(crate) fn new(num_bytes: usize) -> Self {
    Self { num_bytes }
  }
}

impl<const N: usize> ByteArrayComparator for BKDBytes<N> {
  fn compare(&self, a: &[u8], a_i: usize, b: &[u8], b_i: usize) -> i32 {
    if N == 8 {
      U64byteArrayComparator.compare(a, a_i, b, b_i)
    } else if N == 4 {
      U32byteArrayComparator.compare(a, a_i, b, b_i)
    } else {
      a[a_i..a_i + self.num_bytes]
        .cmp(&b[b_i..b_i + self.num_bytes])
        .to_int()
    }
  }
}

pub(crate) struct BKDPrefix<const N: usize>(pub(crate) BKDBytes<N>);

impl<const N: usize> ByteArrayComparator for BKDPrefix<N> {
  fn compare(&self, a: &[u8], a_i: usize, b: &[u8], b_i: usize) -> i32 {
    if N == 8 {
      BKDUtil::common_prefix_length8(a, a_i, b, b_i)
    } else if N == 4 {
      BKDUtil::common_prefix_length4(a, a_i, b, b_i)
    } else {
      BKDUtil::common_prefix_length_n(a, a_i, b, b_i, self.0.num_bytes)
    }
  }
}

pub(crate) struct BKDEquals<const N: usize>(pub(crate) BKDBytes<N>);

impl<const N: usize> ByteArrayPredicate for BKDEquals<N> {
  fn test(&self, a: &[u8], a_i: usize, b: &[u8], b_i: usize) -> bool {
    if N == 8 {
      BKDUtil::equals8(a, a_i, b, b_i)
    } else if N == 4 {
      BKDUtil::equals4(a, a_i, b, b_i)
    } else {
      a[a_i..a_i + self.0.num_bytes] == b[b_i..b_i + self.0.num_bytes]
    }
  }
}

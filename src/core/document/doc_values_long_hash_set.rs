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
use crate::core::util::ram_usage_estimator::size_of_vec;
use crate::core::util::{CoreHelper, TryIntoInt};
#[cfg(test)]
use std::collections::HashSet;
use std::fmt;

const MISSING: i64 = i64::MIN;
/// Set of `i64` values optimized for doc-values usage.
#[derive(PartialEq, Eq, Hash, Debug)]
pub(crate) struct DocValuesLongHashSet {
  pub(crate) table: Vec<i64>,
  pub(crate) mask: i32,
  pub(crate) has_missing_value: bool,
  pub(crate) size: i32,
  /// Minimum value in the set, or `i64::MAX` for an empty set.
  pub(crate) min_value: i64,
  /// Maximum value in the set, or `i64::MIN` for an empty set.
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
  pub fn stream(&self) -> HashSet<i64> {
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
    Ok(size_of_vec(&self.table))
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

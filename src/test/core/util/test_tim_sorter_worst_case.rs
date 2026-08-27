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
use std::collections::LinkedList;

use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::packed::mutable_packed64_enum::MutablePacked64Enum;
use crate::core::util::packed::{Mutable, PackedInts, Reader};
use crate::core::util::{Sorter, TimSorter, TimSorterBase};
use crate::test_framework::core::util::lucene_test_case::{is_night_mode, random};
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestTimSorterWorstCase;

#[cfg(feature = "nightly")]
#[test]
#[ignore = "nightly"]
fn test_worst_case_stack_size() -> Result<()> {
  // We need large arrays to be able to reproduce this bug
  // but not so big we blow up available heap.
  let mut random = random();
  let length = if is_night_mode() {
    TestUtil::next_usize(&mut random, 140_000_000, 400_000_000)
  } else {
    TestUtil::next_usize(&mut random, 140_000_000, 200_000_000)
  };
  let arr = generate_worst_case_array(length)?;
  TimSorter::new(0, WorstCaseSorter::new(arr)).sort(0, length)
}

/// Create an array for the given list of runs.
fn create_array(length: usize, runs: LinkedList<usize>) -> Result<MutablePacked64Enum> {
  let mut array = PackedInts::get_mutable(length as i32, 1, 0.0)?;
  let mut end_run = -1_i32;
  for len in runs {
    end_run += len as i32;
    array.set(end_run, 1)?;
  }
  array.set(length as i32 - 1, 0)?;
  Ok(array)
}

/// Create an array that triggers a worst-case sequence of run lengths.
fn generate_worst_case_array(length: usize) -> Result<MutablePacked64Enum> {
  let min_run = TimSorter::<WorstCaseSorter>::min_run(length);
  let runs = runs_worst_case(length, min_run);
  create_array(length, runs)
}

//
// Code below is borrowed from
// https://github.com/abstools/java-timsort-bug/blob/master/TestTimSort.java
//

/// Fills `runs` with a sequence of run lengths of the form
/// Y_n x_{n,1} x_{n,2} ... x_{n,l_n}
/// Y_{n-1} x_{n-1,1} x_{n-1,2} ... x_{n-1,l_{n-1}}
/// ...
/// Y_1 x_{1,1} x_{1,2} ... x_{1,l_1}
/// The Y_i's are chosen to satisfy the invariant throughout execution, but the x_{i,j}'s are
/// merged (by `TimSorter::ensure_invariants`) into an X_i that violates the invariant.
fn runs_worst_case(length: usize, min_run: usize) -> LinkedList<usize> {
  let mut runs = LinkedList::new();

  let mut running_total = 0;
  let mut y = min_run + 4;
  let mut x = min_run;

  while running_total + y + x <= length {
    running_total += x + y;
    generate_wrong_elem(x, min_run, &mut runs);
    runs.push_front(y);

    // X_{i+1} = Y_i + x_{i,1} + 1, since runs[1] = x_{i,1}
    x = y + runs.iter().nth(1).copied().unwrap() + 1;

    // Y_{i+1} = X_{i+1} + Y_i + 1
    y += x + 1;
  }

  if running_total + x <= length {
    running_total += x;
    generate_wrong_elem(x, min_run, &mut runs);
  }

  runs.push_front(length - running_total);
  runs
}

/// Adds a sequence x_1, ..., x_n of run lengths to `runs` such that:
/// 1. X = x_1 + ... + x_n
/// 2. x_j >= minRun for all j
/// 3. x_1 + ... + x_{j-2} < x_j < x_1 + ... + x_{j-1} for all j
///
/// These conditions guarantee that TimSort merges all x_j's one by one (resulting in X) using only
/// merges on the second-to-last element.
///
/// # Arguments
///
/// * `x` - The sum of the sequence that should be added to runs.
fn generate_wrong_elem(mut x: usize, min_run: usize, runs: &mut LinkedList<usize>) {
  while x > 2 * min_run {
    // Default strategy
    let mut new_total = x / 2 + 1;

    // Specialized strategies
    if (3 * min_run + 3..=4 * min_run + 1).contains(&x) {
      // Add x_1=MIN+1, x_2=MIN, x_3=X-newTotal to runs.
      new_total = 2 * min_run + 1;
    } else if (5 * min_run + 5..=6 * min_run + 5).contains(&x) {
      // Add x_1=MIN+1, x_2=MIN, x_3=MIN+2, x_4=X-newTotal to runs.
      new_total = 3 * min_run + 3;
    } else if (8 * min_run + 9..=10 * min_run + 9).contains(&x) {
      // Add x_1=MIN+1, x_2=MIN, x_3=MIN+2, x_4=2MIN+2, x_5=X-newTotal to runs.
      new_total = 5 * min_run + 5;
    } else if (13 * min_run + 15..=16 * min_run + 17).contains(&x) {
      // Add x_1=MIN+1, x_2=MIN, x_3=MIN+2, x_4=2MIN+2, x_5=3MIN+4,
      // x_6=X-newTotal to runs.
      new_total = 8 * min_run + 9;
    }
    runs.push_front(x - new_total);
    x = new_total;
  }
  runs.push_front(x);
}

struct WorstCaseSorter {
  arr: MutablePacked64Enum,
  pivot_index: usize,
}

impl WorstCaseSorter {
  fn new(arr: MutablePacked64Enum) -> Self {
    Self {
      arr,
      pivot_index: 0,
    }
  }
}

impl Sorter for WorstCaseSorter {
  fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    let tmp = self.arr.get(i);
    self.arr.set(i as i32, self.arr.get(j))?;
    self.arr.set(j as i32, tmp)?;
    Ok(())
  }

  fn compare(&mut self, i: usize, j: usize) -> Result<i32> {
    Ok(match self.arr.get(i).cmp(&self.arr.get(j)) {
      Ordering::Less => -1,
      Ordering::Equal => 0,
      Ordering::Greater => 1,
    })
  }

  fn set_pivot(&mut self, i: usize) -> Result<()> {
    self.pivot_index = i;
    Ok(())
  }

  fn compare_pivot(&mut self, j: usize) -> Result<i32> {
    self.compare(self.pivot_index, j)
  }
}

impl TimSorterBase for WorstCaseSorter {
  fn copy(&mut self, src: usize, dest: usize) -> Result<()> {
    self.arr.set(dest as i32, self.arr.get(src))
  }

  fn save(&mut self, _i: usize, _len: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn restore(&mut self, _i: usize, _j: usize) {
    panic!("restore is unsupported")
  }

  fn compare_saved(&self, _i: usize, _j: usize) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }
}

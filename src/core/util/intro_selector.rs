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
use rand::RngExt;
use rand::rngs::ThreadRng;

use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::intro_sorter::SINGLE_MEDIAN_THRESHOLD;
use crate::core::util::selector::Selector;

/// Adaptive selection algorithm based on the introspective quick select
/// algorithm. The quick select algorithm uses an interpolation variant of
/// Tukey's ninther median-of-medians for pivot, and Bentley-McIlroy 3-way
/// partitioning. For the introspective protection, it shuffles the sub-range if
/// the max recursive depth is exceeded.
///
/// This selection algorithm is fast on most data shapes, especially on nearly
/// sorted data, or when `k` is close to the boundaries. It runs in linear time
/// on average.
///
/// # Internal
/// This method is intended for internal use in the library.
pub struct IntroSelector<T> {
  random: Option<ThreadRng>,
  sub_selector: T,
}
impl<T> IntroSelector<T> {
  pub fn new(sub_selector: T) -> IntroSelector<T> {
    IntroSelector {
      random: None,
      sub_selector,
    }
  }
}

impl<T> IntroSelector<T>
where
  T: IntroSelectorBase,
{
  pub fn select(
    &mut self,
    mut from: usize,
    mut to: usize,
    k: usize,
    mut max_depth: i32,
  ) -> Result<()> {
    // This code is adapted from `IntroSorter::sort` to loop on a
    // single partition.

    // For efficiency, we must enter the loop with at least 4 entries to be
    // able to skip some boundary tests during the 3-way
    // partitioning.
    let mut size;

    // Ensure the loop enters with at least 4 entries to skip boundary
    // checks.
    while {
      size = to - from;
      size > 3
    } {
      max_depth -= 1;
      if max_depth == -1 {
        // Max recursion depth exceeded: shuffle (only once) and
        // continue.
        self.shuffle(from, to)?;
      }

      // Pivot selection based on medians.
      let last = to - 1;
      let mid = (from + last) >> 1;
      let pivot;

      if size <= SINGLE_MEDIAN_THRESHOLD {
        // Select the pivot with a single median around the middle
        // element. Do not take the median between
        // [from, mid, last] because it hurts performance
        // if the order is descending in conjunction with the 3-way
        // partitioning.
        let range = size >> 2;
        pivot = self.median(mid - range, mid, mid + range)?;
      } else {
        // Select the pivot with a variant of the Tukey's ninther median
        // of medians. If k is close to the boundaries,
        // select either the lowest or highest median (this variant
        // is inspired from the interpolation search).
        let range = size >> 3;
        let double_range = range << 1;
        let median_first = self.median(from, from + range, from + double_range)?;
        let median_middle = self.median(mid - range, mid, mid + range)?;
        let median_last = self.median(last - double_range, last - range, last)?;
        if k - from < range {
          // k is close to 'from': select the lowest median.
          pivot = self.min(median_first, median_middle, median_last)?;
        } else if to - k <= range {
          pivot = self.max(median_first, median_middle, median_last)?;
        } else {
          pivot = self.median(median_first, median_middle, median_last)?;
        }
      }
      // Bentley-McIlroy 3-way partitioning
      self.sub_selector.set_pivot(pivot)?;
      self.sub_selector.swap(from, pivot)?;

      let mut i = from;
      let mut j: i32 = to as i32;
      let mut p = from + 1;
      let mut q: i32 = last as i32;

      loop {
        let mut left_cmp;
        let mut right_cmp;

        while {
          left_cmp = self.sub_selector.compare_pivot(i + 1)?;
          i += 1;
          left_cmp > 0
        } {}

        while {
          right_cmp = self.sub_selector.compare_pivot(j.try_convert()? - 1)?;
          j -= 1;
          right_cmp < 0
        } {}
        let v: i32 = i.try_convert()?;
        if v >= j {
          if v == j && right_cmp == 0 {
            self.sub_selector.swap(i, p)?;
          }
          break;
        }

        self.sub_selector.swap(i, j.try_convert()?)?;
        if right_cmp == 0 {
          self.sub_selector.swap(i, p)?;
          p += 1;
        }

        if left_cmp == 0 {
          self.sub_selector.swap(j.try_convert()?, q.try_convert()?)?;
          q -= 1;
        }
      }
      i = (j + 1).try_convert()?;
      for l in from..p {
        self.sub_selector.swap(l, j.try_convert()?)?;
        j -= 1;
      }
      for l in last..q.try_convert()? {
        self.sub_selector.swap(l, i)?;
        i += 1;
      }
      let v: i32 = k.try_convert()?;
      if v <= j {
        to = (j + 1).try_convert()?;
      } else if k >= i {
        from = i;
      } else {
        return Ok(());
      }
    }
    // Sort the final tiny range (3 entries or less) with a very specialized
    // sort.
    match size {
      2 if IntroSelectorBase::compare(&mut self.sub_selector, from, from + 1)? > 0 => {
        self.sub_selector.swap(from, from + 1)?;
      },
      3 => {
        self.sort3(from)?;
      },
      _ => {},
    }
    Ok(())
  }

  /// Returns the index of the min element among three elements at provided
  /// indices.
  pub fn min(&mut self, i: usize, j: usize, k: usize) -> Result<usize> {
    if IntroSelectorBase::compare(&mut self.sub_selector, i, j)? <= 0 {
      if IntroSelectorBase::compare(&mut self.sub_selector, i, k)? <= 0 {
        Ok(i)
      } else {
        Ok(k)
      }
    } else if IntroSelectorBase::compare(&mut self.sub_selector, j, k)? <= 0 {
      Ok(j)
    } else {
      Ok(k)
    }
  }

  /// Returns the index of the max element among three elements at provided
  /// indices.
  pub fn max(&mut self, i: usize, j: usize, k: usize) -> Result<usize> {
    if IntroSelectorBase::compare(&mut self.sub_selector, i, j)? <= 0 {
      if IntroSelectorBase::compare(&mut self.sub_selector, j, k)? < 0 {
        Ok(k)
      } else {
        Ok(j)
      }
    } else if IntroSelectorBase::compare(&mut self.sub_selector, i, k)? < 0 {
      Ok(k)
    } else {
      Ok(i)
    }
  }

  pub fn median(&mut self, i: usize, j: usize, k: usize) -> Result<usize> {
    if IntroSelectorBase::compare(&mut self.sub_selector, i, j)? < 0 {
      if IntroSelectorBase::compare(&mut self.sub_selector, j, k)? <= 0 {
        return Ok(j);
      }
      return if IntroSelectorBase::compare(&mut self.sub_selector, i, k)? < 0 {
        Ok(k)
      } else {
        Ok(i)
      };
    }
    if IntroSelectorBase::compare(&mut self.sub_selector, j, k)? >= 0 {
      return Ok(j);
    }
    if IntroSelectorBase::compare(&mut self.sub_selector, i, k)? < 0 {
      Ok(i)
    } else {
      Ok(k)
    }
  }
  /// Sorts 3 entries starting at from (inclusive). This specialized method is
  /// more efficient than calling `insertionSort(int, int)`.
  pub fn sort3(&mut self, from: usize) -> Result<()> {
    let mid = from + 1;
    let last = from + 2;

    if IntroSelectorBase::compare(&mut self.sub_selector, from, mid)? <= 0 {
      if IntroSelectorBase::compare(&mut self.sub_selector, mid, last)? > 0 {
        self.sub_selector.swap(mid, last)?;
        if IntroSelectorBase::compare(&mut self.sub_selector, from, mid)? > 0 {
          self.sub_selector.swap(from, mid)?;
        }
      }
    } else if IntroSelectorBase::compare(&mut self.sub_selector, mid, last)? >= 0 {
      self.sub_selector.swap(from, last)?;
    } else {
      self.sub_selector.swap(from, mid)?;
      if IntroSelectorBase::compare(&mut self.sub_selector, mid, last)? > 0 {
        self.sub_selector.swap(mid, last)?;
      }
    }
    Ok(())
  }
  /// Shuffles the entries between from (inclusive) and to (exclusive) with
  /// Durstenfeld's algorithm.
  pub fn shuffle(&mut self, from: usize, to: usize) -> Result<()> {
    let random = self.random.get_or_insert_with(rand::rng);

    for i in (from..to).rev() {
      let j = random.random_range(from..=i);
      self.sub_selector.swap(i, j)?;
    }
    Ok(())
  }
}

impl<T> Selector for IntroSelector<T>
where
  T: IntroSelectorBase,
{
  fn select(&mut self, from: usize, to: usize, k: usize) -> Result<()> {
    self.check_args(from, to, k)?;
    let max_depth = 2 * (f64::log2((to - from) as f64) as i32);
    self.select(from, to, k, max_depth)?;
    Ok(())
  }
}

pub trait IntroSelectorBase: IntroSelectorBaseDefault + Selector {
  /// Compare entries found in slots `i` and `j`.
  fn compare(&mut self, i: usize, j: usize) -> Result<i32> {
    IntroSelectorBaseDefault::compare(self, i, j)
  }
}
pub trait IntroSelectorBaseDefault {
  /// Save the value at slot `i` so that it can later be used as a pivot.
  fn set_pivot(&mut self, i: usize) -> Result<()>;
  /// Compare the pivot with the slot at `j`, similarly to `compare(i, j)`.
  fn compare_pivot(&mut self, j: usize) -> Result<i32>;
  fn compare(&mut self, i: usize, j: usize) -> Result<i32> {
    self.set_pivot(i)?;
    self.compare_pivot(j)
  }
}

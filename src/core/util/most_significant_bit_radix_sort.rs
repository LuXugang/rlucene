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
use crate::core::index::BytesRefBuilder;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::intro_sorter::IntroSorter;
use crate::core::util::{Sorter, TryIntoInt, check_range};

/// After this many levels of recursion, we fall back to introsort.
/// This protects against poor performance when there are long common prefixes,
/// likely due to cache locality issues.
pub(crate) const LEVEL_THRESHOLD: usize = 8;
/// Size of histograms: 256 + 1 to indicate that the string is finished.
pub(crate) const HISTOGRAM_SIZE: usize = 257;
/// Buckets below this size will be sorted with the fallback sorter.
pub(crate) const LENGTH_THRESHOLD: usize = 100;
pub struct MSBRadixSorter<T> {
  /// One histogram per recursion level.
  histograms: Vec<Vec<usize>>,
  /// End offsets for histograms.
  end_offsets: Vec<usize>,
  /// Array to store common prefixes.
  common_prefix: Vec<i32>,
  /// Maximum length of strings to sort.
  max_length: usize,
  delegate: T,
}
impl<T> MSBRadixSorter<T> {
  /// Creates a new instance.
  ///
  /// # Parameters
  /// - `max_length`: The maximum length of keys. Pass `i32::MAX` if unknown.
  pub fn new(max_length: usize, delegate: T) -> Self {
    let histograms: Vec<Vec<usize>> = (0..LEVEL_THRESHOLD).map(|_| Vec::new()).collect();
    Self {
      histograms,
      end_offsets: vec![0; HISTOGRAM_SIZE],
      max_length,
      common_prefix: vec![0; 24.min(max_length)],
      delegate,
    }
  }
}

impl<T> MSBRadixSorter<T>
where
  T: MSBRadixSorterBase,
{
  pub fn sort_impl(&mut self, from: usize, to: usize, k: usize, l: usize) -> Result<()> {
    if self.should_fallback(from, to, l) {
      self.get_fallback_sorter(k).sort(from, to)
    } else {
      self.radix_sort(from, to, k, l)
    }
  }
  fn should_fallback(&self, from: usize, to: usize, l: usize) -> bool {
    self.delegate.should_fallback(from, to, l)
  }
  /// Computes the initial common prefix length for the given range.
  ///
  /// This method has been split to avoid platform-specific issues.
  fn compute_initial_common_prefix_length(&mut self, from: usize, k: usize) -> Result<usize> {
    let common_prefix = &mut self.common_prefix;
    let mut common_prefix_length = std::cmp::min(common_prefix.len(), self.max_length - k);

    for (j, slot) in common_prefix
      .iter_mut()
      .enumerate()
      .take(common_prefix_length)
    {
      let b = self.delegate.byte_at(from, k + j)?;
      *slot = b;
      if b == -1 {
        common_prefix_length = j + 1;
        break;
      }
    }
    Ok(common_prefix_length)
  }
  fn compute_common_prefix_length_and_build_histogram_part2(
    &mut self,
    from: usize,
    to: usize,
    k: usize,
    l: usize,
    common_prefix_length: usize,
    i: usize,
  ) -> Result<usize> {
    if i < to {
      debug_assert!(common_prefix_length == 0);
      self.build_histogram(
        (self.common_prefix[0] + 1).try_convert()?,
        i - from,
        i,
        to,
        k,
        l,
      )?;
    } else {
      debug_assert!(common_prefix_length > 0);
      self.histograms[l][(self.common_prefix[0] + 1).try_convert()?] = to - from;
    }

    Ok(common_prefix_length)
  }
  /// Build a histogram of the k-th characters of values occurring between
  /// offsets `from` and `to`, using the `get_bucket` method.
  fn build_histogram(
    &mut self,
    prefix_common_bucket: usize,
    prefix_common_len: usize,
    from: usize,
    to: usize,
    k: usize,
    l: usize,
  ) -> Result<()> {
    self.delegate.build_histogram(
      prefix_common_bucket,
      prefix_common_len,
      from,
      to,
      k,
      &mut self.histograms[l],
    )
  }
  fn compute_common_prefix_length_and_build_histogram_part1(
    &mut self,
    from: usize,
    to: usize,
    k: usize,
    l: usize,
    mut common_prefix_length: usize,
  ) -> Result<usize> {
    let mut i = from + 1;

    'outer: for idx in from + 1..to {
      let mut j = 0;
      while j < common_prefix_length {
        let b = self.delegate.byte_at(idx, k + j)?;
        if b != self.common_prefix[j] {
          common_prefix_length = j;
          if common_prefix_length == 0 {
            break 'outer;
          }
          break;
        }
        j += 1;
      }
      i = idx + 1;
    }

    self.compute_common_prefix_length_and_build_histogram_part2(
      from,
      to,
      k,
      l,
      common_prefix_length,
      i,
    )
  }
  pub fn compute_common_prefix_length_and_build_histogram(
    &mut self,
    from: usize,
    to: usize,
    k: usize,
    l: usize,
  ) -> Result<usize> {
    let common_prefix_length = self.compute_initial_common_prefix_length(from, k)?;
    self.compute_common_prefix_length_and_build_histogram_part1(
      from,
      to,
      k,
      l,
      common_prefix_length,
    )
  }
  fn sum_histogram(histogram: &mut [usize], end_offsets: &mut [usize]) {
    let mut accum = 0;
    for (hist, end_offset) in histogram.iter_mut().zip(end_offsets.iter_mut()) {
      let count = *hist;
      *hist = accum;
      accum += count;
      *end_offset = accum;
    }
  }
  /// Reorder based on start/end offsets for each bucket. When this method
  /// returns, `start_offsets` and `end_offsets` are equal.
  ///
  /// # Parameters
  /// - `from`: The starting index (inclusive).
  /// - `to`: The ending index (exclusive).
  /// - `start_offsets`: Start offsets per bucket.
  /// - `end_offsets`: End offsets per bucket.
  /// - `k`: The current position offset.
  fn reorder(&mut self, from: usize, to: usize, l: usize, k: usize) -> Result<()> {
    self
      .delegate
      .reorder(from, to, &mut self.histograms[l], &mut self.end_offsets, k)
  }
  /// Performs radix sort on the specified range and recursion level.
  ///
  /// # Parameters
  /// - `from`: Start index (inclusive).
  /// - `to`: End index (exclusive).
  /// - `k`: The character number to compare.
  /// - `l`: The level of recursion.
  fn radix_sort(&mut self, from: usize, to: usize, k: usize, l: usize) -> Result<()> {
    // Access or initialize the histogram for this level
    if self.histograms[l].is_empty() {
      self.histograms[l] = vec![0; HISTOGRAM_SIZE];
    } else {
      self.histograms[l].fill(0);
    }

    // Compute the common prefix length and build the histogram
    let common_prefix_length =
      self.compute_common_prefix_length_and_build_histogram(from, to, k, l)?;

    if common_prefix_length > 0 {
      // if there are no more chars to compare or if all entries fell into
      // the first bucket (which means strings are shorter
      // than k) then we are done otherwise recurse
      if k + common_prefix_length < self.max_length && self.histograms[l][0] < (to - from) {
        self.radix_sort(from, to, k + common_prefix_length, l)?;
      }
      return Ok(());
    }

    // Assert histogram correctness (can be implemented as a debug check)
    debug_assert!(Self::assert_histogram(
      common_prefix_length,
      &self.histograms[l]
    ));

    // Prepare start and end offsets
    Self::sum_histogram(&mut self.histograms[l], &mut self.end_offsets);

    // Reorder the range
    self.reorder(from, to, l, k)?;

    // Update end offsets

    // Recursively sort buckets if more levels are allowed
    if k + 1 < self.max_length {
      let mut prev = self.histograms[l][0];
      for i in 1..HISTOGRAM_SIZE {
        let h = self.histograms[l][i];
        let bucket_len = h - prev;
        if bucket_len > 1 {
          self.sort_impl(from + prev, from + h, k + 1, l + 1)?;
        }
        prev = h;
      }
    }
    Ok(())
  }

  fn get_fallback_sorter(&mut self, k: usize) -> impl Sorter + use<'_, T> {
    self.delegate.get_fallback_sorter(k, self.max_length)
  }

  /// Always returns `true` if the assertions pass.
  fn assert_histogram(common_prefix_length: usize, histogram: &[usize]) -> bool {
    let number_of_unique_bytes = histogram.iter().filter(|&&freq| freq > 0).count();

    if number_of_unique_bytes == 1 {
      debug_assert!(common_prefix_length >= 1);
    } else {
      debug_assert!(
        common_prefix_length == 0,
        "Expected common_prefix_length to be 0, but found {common_prefix_length}"
      );
    }
    true
  }
  #[cfg(test)]
  pub fn get_delegate(&self) -> &T {
    &self.delegate
  }
}

impl<T> Sorter for MSBRadixSorter<T>
where
  T: MSBRadixSorterBase,
{
  fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    self.delegate.swap(i, j)
  }

  fn set_pivot(&mut self, i: usize) -> Result<()> {
    self.delegate.set_pivot(i)
  }

  fn compare_pivot(&mut self, i: usize) -> Result<i32> {
    self.delegate.compare_pivot(i)
  }

  fn sort(&mut self, from: usize, to: usize) -> Result<()> {
    check_range(from, to)?;
    self.sort_impl(from, to, 0, 0)
  }
}

pub struct MSBRadixIntroSorterImpl<'a, T> {
  pivot: BytesRefBuilder<Vec<u8>>,
  max_length: usize,
  k: usize,
  delegate: &'a mut T,
}

impl<T> Sorter for MSBRadixIntroSorterImpl<'_, T>
where
  T: MSBRadixSorterBase,
{
  fn compare(&mut self, i: usize, j: usize) -> Result<i32> {
    for o in self.k..self.max_length {
      let b1 = self.delegate.byte_at(i, o)?;
      let b2 = self.delegate.byte_at(j, o)?;

      if b1 != b2 {
        return Ok(b1 - b2);
      } else if b1 == -1 {
        break;
      }
    }
    Ok(0)
  }

  fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    self.delegate.swap(i, j)
  }

  fn set_pivot(&mut self, i: usize) -> Result<()> {
    self.pivot.set_length(0);

    for o in self.k..self.max_length {
      let b = self.delegate.byte_at(i, o)?;
      if b == -1 {
        break;
      }
      self.pivot.append_byte(b as u8)?;
    }
    Ok(())
  }

  fn compare_pivot(&mut self, j: usize) -> Result<i32> {
    for o in 0..self.pivot.length() {
      let b1 = self.pivot.byte_at(o) as i32;
      let b2 = self.delegate.byte_at(j, self.k + o)?;
      if b1 != b2 {
        return Ok(b1 - b2);
      }
    }

    if self.k + self.pivot.length() == self.max_length {
      Ok(0)
    } else {
      Ok(-1 - self.delegate.byte_at(j, self.k + self.pivot.length())?)
    }
  }

  fn sort(&mut self, from: usize, to: usize) -> Result<()> {
    IntroSorter::sort_range(self, from, to)?;
    Ok(())
  }
}

impl<T> IntroSorter for MSBRadixIntroSorterImpl<'_, T> where T: MSBRadixSorterBase {}

pub trait MSBRadixSorterBase: Sorter {
  /// Returns the k-th byte of the entry at the given index `i`, or `-1` if
  /// its length is less than or equal to `k`.
  ///
  /// # Parameters
  /// - `i`: The index of the entry, which must be between `0` (inclusive) and
  ///   `max_length` (exclusive).
  /// - `k`: The position of the byte to retrieve within the entry.
  ///
  /// # Returns
  /// The k-th byte of the entry at index `i` as an `i32`, or `-1` if the
  /// entry's length is less than or equal to `k`.
  ///
  /// # Note
  /// In Rust, this method might return a signed integer (`i32`) to
  /// accommodate the `-1` case, which differs from Java's default integer
  /// handling.
  fn byte_at(&mut self, _i: usize, _k: usize) -> Result<i32> {
    Err(LuceneError::not_implemented(""))
  }

  fn get_fallback_sorter(&mut self, k: usize, length: usize) -> impl Sorter
  where
    Self: Sized,
  {
    MSBRadixIntroSorterImpl {
      pivot: BytesRefBuilder::new(),
      max_length: length,
      k,
      delegate: self,
    }
  }

  /// Reorder based on start/end offsets for each bucket. When this method
  /// returns, `start_offsets` and `end_offsets` are equal.
  ///
  /// # Parameters
  /// - `from`: The starting index (inclusive).
  /// - `to`: The ending index (exclusive).
  /// - `start_offsets`: Start offsets per bucket.
  /// - `end_offsets`: End offsets per bucket.
  /// - `k`: The current position offset.
  fn reorder(
    &mut self,
    from: usize,
    _to: usize,
    start_offsets: &mut [usize],
    end_offsets: &mut [usize],
    k: usize,
  ) -> Result<()> {
    // Reorder in place, similar to the Dutch national flag problem
    for i in 0..HISTOGRAM_SIZE {
      let limit = end_offsets[i];
      while start_offsets[i] < limit {
        let h1 = start_offsets[i];
        let b = self.get_bucket(from + h1, k)?;
        let h2 = start_offsets[b as usize];
        start_offsets[b as usize] += 1;
        self.swap(from + h1, from + h2)?;
      }
    }
    Ok(())
  }

  fn get_bucket(&mut self, i: usize, k: usize) -> Result<i32> {
    Ok(self.byte_at(i, k)? + 1)
  }

  fn build_histogram(
    &mut self,
    prefix_common_bucket: usize,
    prefix_common_len: usize,
    from: usize,
    to: usize,
    k: usize,
    histogram: &mut [usize],
  ) -> Result<()> {
    histogram[prefix_common_bucket] = prefix_common_len;

    for i in from..to {
      let b = self.get_bucket(i, k)? as usize;
      histogram[b] += 1;
    }
    Ok(())
  }

  fn should_fallback(&self, from: usize, to: usize, l: usize) -> bool {
    (to - from) <= LENGTH_THRESHOLD || l >= LEVEL_THRESHOLD
  }
}

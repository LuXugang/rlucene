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

use std::cmp::{max, min};

use crate::core::util::error::lucene_error::Result;
use crate::core::util::{Sorter, sorter};

const MIN_RUN: usize = 32;
const THRESHOLD: usize = 64;
const STACK_SIZE: usize = 49; // depends on MINRUN
const MIN_GALLOP: usize = 7;

/// [`Sorter`] implementation based on the [TimSort](http://svn.python.org/projects/python/trunk/Objects/listsort.txt) algorithm. It
/// sorts small arrays with a binary sort.
///
/// This algorithm is stable and is especially good at sorting partially-sorted
/// arrays.
///
/// # Note
/// There are a few differences with the original implementation:
/// - The extra amount of memory to perform merges is configurable. This allows
///   small merges to be very fast, while large merges will be performed
///   in-place (slightly slower). You can ensure that the fast merge routine
///   will always be used by having `max_temp_slots` equal to half the length of
///   the slice of data to sort.
/// - Only the fast merge routine can gallop (the one that doesn't run
///   in-place), and it only gallops on the longest slice.
///
/// # Note
/// This is an internal API.
pub struct TimSorter<T>
where
  T: TimSorterBase,
{
  max_temp_slots: usize,
  min_run: usize,
  to: usize,
  stack_size: usize,
  run_ends: Vec<usize>,
  delegate: T,
}
impl<T: TimSorterBase> TimSorter<T> {
  pub fn new(max_temp_slots: usize, delegate: T) -> TimSorter<T> {
    TimSorter {
      max_temp_slots,
      min_run: 0,
      to: 0,
      stack_size: 0,
      run_ends: vec![0; STACK_SIZE + 1],
      delegate,
    }
  }
  fn min_run(&self, length: usize) -> usize {
    debug_assert!(length >= MIN_RUN);
    let mut n = length;
    let mut r = 0;
    while n >= 64 {
      r |= n & 1;
      n >>= 1;
    }
    let min_run = n + r;
    debug_assert!((MIN_RUN..=THRESHOLD).contains(&min_run));
    min_run
  }
  fn run_len(&self, i: usize) -> usize {
    let off = self.stack_size - i;
    self.run_ends[off] - self.run_ends[off - 1]
  }
  fn run_base(&self, i: usize) -> usize {
    self.run_ends[self.stack_size - i - 1]
  }
  fn run_end(&self, i: usize) -> usize {
    self.run_ends[self.stack_size - i]
  }
  fn set_run_end(&mut self, i: usize, run_end: usize) {
    self.run_ends[self.stack_size - i] = run_end;
  }
  fn push_run_len(&mut self, len: usize) {
    self.run_ends[self.stack_size + 1] = self.run_ends[self.stack_size] + len;
    self.stack_size += 1;
  }
  // Compute the length of the next run, make the run sorted and return its
  // length.
  fn next_run(&mut self) -> Result<usize> {
    let run_base = self.run_end(0);
    debug_assert!(run_base < self.to);

    if run_base == self.to - 1 {
      return Ok(1);
    }
    let mut o = run_base + 2;
    if self.compare(run_base, run_base + 1)? > 0 {
      while o < self.to && self.compare(o - 1, o)? > 0 {
        o += 1;
      }
      self.reverse(run_base, o)?;
    } else {
      while o < self.to && self.compare(o - 1, o)? <= 0 {
        o += 1;
      }
    }
    let run_hi = max(o, min(self.to, run_base + self.min_run));
    self.binary_sort_with_start(run_base, run_hi, o)?;
    Ok(run_hi - run_base)
  }
  pub fn ensure_invariants(&mut self) -> Result<()> {
    while self.stack_size > 1 {
      let run_len0 = self.run_len(0);
      let run_len1 = self.run_len(1);

      if self.stack_size > 2 {
        let run_len2 = self.run_len(2);

        if run_len2 <= run_len1 + run_len0 {
          // merge the smaller of 0 and 2 with 1
          if run_len2 < run_len0 {
            self.merge_at(1)?;
          } else {
            self.merge_at(0)?;
          }
          continue;
        }
      }

      if run_len1 <= run_len0 {
        self.merge_at(0)?;
        continue;
      }

      break;
    }
    Ok(())
  }
  pub fn exhaust_stack(&mut self) -> Result<()> {
    while self.stack_size > 1 {
      self.merge_at(0)?;
    }
    Ok(())
  }

  pub fn reset(&mut self, from: usize, to: usize) {
    self.stack_size = 0;
    self.run_ends.fill(0);
    self.run_ends[0] = from;
    self.to = to;
    let length = to - from;
    self.min_run = if length <= THRESHOLD {
      length
    } else {
      self.min_run(length)
    };
  }

  pub fn merge_at(&mut self, n: usize) -> Result<()> {
    debug_assert!(self.stack_size >= 2);
    self.merge(self.run_base(n + 1), self.run_base(n), self.run_end(n))?;

    for j in (1..=n + 1).rev() {
      self.set_run_end(j, self.run_end(j - 1));
    }

    self.stack_size -= 1;
    Ok(())
  }

  fn merge(&mut self, mut lo: usize, mid: usize, mut hi: usize) -> Result<()> {
    if self.compare(mid - 1, mid)? <= 0 {
      return Ok(());
    }

    lo = self.upper2(lo, mid, mid)?;
    hi = self.lower2(mid, hi, mid - 1)?;
    if hi - mid <= mid - lo && hi - mid <= self.max_temp_slots {
      self.merge_hi(lo, mid, hi)?;
    } else if mid - lo <= self.max_temp_slots {
      self.merge_lo(lo, mid, hi)?;
    } else {
      self.merge_in_place(lo, mid, hi)?;
    }
    Ok(())
  }

  fn merge_lo(&mut self, lo: usize, mid: usize, hi: usize) -> Result<()> {
    debug_assert!(self.delegate.compare(lo, mid)? > 0);

    let len1 = mid - lo;
    self.delegate.save(lo, len1)?;
    self.delegate.copy(mid, lo);

    let mut i = 0;
    let mut j = mid + 1;
    let mut dest = lo + 1;

    'outer: loop {
      let mut count = 0;
      while count < MIN_GALLOP {
        if i >= len1 || j >= hi {
          break 'outer;
        } else if self.delegate.compare_saved(i, j)? <= 0 {
          self.delegate.restore(i, dest);
          i += 1;
          dest += 1;
          count = 0;
        } else {
          self.delegate.copy(j, dest);
          j += 1;
          dest += 1;
          count += 1;
        }
      }

      // Galloping phase
      let next = self.lower_saved3(j, hi, i)?;
      while j < next {
        self.delegate.copy(j, dest);
        j += 1;
        dest += 1;
      }
      self.delegate.restore(i, dest);
      i += 1;
      dest += 1;
    }

    while i < len1 {
      self.delegate.restore(i, dest);
      i += 1;
      dest += 1;
    }

    debug_assert_eq!(j, dest);
    Ok(())
  }

  pub fn merge_hi(&mut self, lo: usize, mid: usize, hi: usize) -> Result<()> {
    debug_assert!(self.compare(mid - 1, hi - 1)? > 0);

    let len2 = hi - mid;
    self.delegate.save(mid, len2)?;
    self.delegate.copy(mid - 1, hi - 1);

    let mut i: i32 = mid as i32 - 2;
    let mut j: i32 = len2 as i32 - 1;
    let mut dest: i32 = hi as i32 - 2;

    'outer: loop {
      let mut count = 0;
      while count < MIN_GALLOP {
        if i < lo as i32 || j < 0 {
          break 'outer;
        } else if self.delegate.compare_saved(j as usize, i as usize)? >= 0 {
          self.delegate.restore(j as usize, dest as usize);
          j -= 1;
          dest -= 1;
          count = 0;
        } else {
          self.delegate.copy(i as usize, dest as usize);
          i -= 1;
          dest -= 1;
          count += 1;
        }
      }

      // Galloping phase
      let next = self.upper_saved3(lo, (i + 1) as usize, j as usize)?;
      while i >= next as i32 {
        self.delegate.copy(i as usize, dest as usize);
        i -= 1;
        dest -= 1;
      }
      self.delegate.restore(j as usize, dest as usize);
      j -= 1;
      dest -= 1;
    }

    while j >= 0 {
      self.delegate.restore(j as usize, dest as usize);
      j -= 1;
      dest -= 1;
    }

    debug_assert!(i == dest);
    Ok(())
  }

  pub fn lower_saved(&self, mut from: usize, to: usize, val: usize) -> Result<usize> {
    let mut len: i32 = if to >= from {
      (to - from) as i32
    } else {
      return Ok(from);
    };

    while len > 0 {
      let half = len >> 1;
      let mid = from + half as usize;
      if self.delegate.compare_saved(val, mid)? > 0 {
        from = mid + 1;
        len -= half + 1;
      } else {
        len = half;
      }
    }
    Ok(from)
  }

  pub fn upper_saved(&self, mut from: usize, to: usize, val: usize) -> Result<usize> {
    let mut len: i32 = if to >= from {
      (to - from) as i32
    } else {
      return Ok(from);
    };

    while len > 0 {
      let half = len >> 1;
      let mid = from + half as usize;
      if self.delegate.compare_saved(val, mid)? < 0 {
        len = half;
      } else {
        from = mid + 1;
        len -= half + 1;
      }
    }
    Ok(from)
  }

  pub fn lower_saved3(&self, from: usize, to: usize, val: usize) -> Result<usize> {
    let mut f = from;
    let mut t = f + 1;

    while t < to {
      if self.delegate.compare_saved(val, t)? <= 0 {
        return self.lower_saved(f, t, val);
      }
      let delta = t - f;
      f = t;
      t += delta * 2;
    }
    self.lower_saved(f, to, val)
  }

  pub fn upper_saved3(&self, from: usize, to: usize, val: usize) -> Result<usize> {
    let mut f: i32 = to as i32 - 1;
    let mut t = to;

    while f > from as i32 {
      let v = f as usize;
      if self.delegate.compare_saved(val, v)? >= 0 {
        return self.upper_saved(v, t, val);
      }
      let delta = t - v;
      t = v;
      f -= delta as i32 * 2
    }
    self.upper_saved(from, t, val)
  }
}
impl<T> Sorter for TimSorter<T>
where
  T: TimSorterBase,
{
  fn compare(&mut self, i: usize, j: usize) -> Result<i32> {
    self.delegate.compare(i, j)
  }

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
    sorter::check_range(from, to)?;
    if to - from <= 1 {
      return Ok(());
    }

    self.reset(from, to);

    loop {
      self.ensure_invariants()?;
      let run_length = self.next_run()?;
      self.push_run_len(run_length);

      if self.run_end(0) >= to {
        break;
      }
    }
    self.exhaust_stack()?;

    debug_assert_eq!(self.run_end(0), to);
    Ok(())
  }

  fn do_rotate(&mut self, mut lo: usize, mut mid: usize, hi: usize) -> Result<()> {
    let len1 = mid - lo;
    let len2 = hi - mid;

    if len1 == len2 {
      while mid < hi {
        self.swap(lo, mid)?;
        lo += 1;
        mid += 1;
      }
    } else if len2 < len1 && len2 <= self.max_temp_slots {
      self.delegate.save(mid, len2)?;
      let mut i: i32 = (lo + len1) as i32 - 1;
      let mut j: i32 = hi as i32 - 1;
      while i >= lo as i32 {
        self.delegate.copy(i as usize, j as usize);
        i -= 1;
        j -= 1;
      }
      i = 0;
      j = lo as i32;
      while i < len2 as i32 {
        self.delegate.restore(i as usize, j as usize);
        i += 1;
        j += 1;
      }
    } else if len1 <= self.max_temp_slots {
      self.delegate.save(lo, len1)?;
      let mut i = mid;
      let mut j = lo;
      while i < hi {
        self.delegate.copy(i, j);
        i += 1;
        j += 1;
      }
      i = 0;
      j = lo + len2;
      while j < hi {
        self.delegate.restore(i, j);
        i += 1;
        j += 1;
      }
    } else {
      self.reverse(lo, mid)?;
      self.reverse(mid, hi)?;
      self.reverse(lo, hi)?;
    }
    Ok(())
  }
}

pub trait TimSorterBase: Sorter {
  ///Copy data from slot `src` to slot `dest`
  fn copy(&mut self, src: usize, dest: usize);

  /// Save all elements between slots i and `i+len` into the temporary
  /// storage.
  fn save(&mut self, i: usize, len: usize) -> Result<()>;
  /// Restore element `j` from the temporary storage into slot `i`.
  fn restore(&mut self, i: usize, j: usize);

  /// Compare element `i` from the temporary storage with element `j` from the
  /// slice to sort, similarly to `compare`.
  fn compare_saved(&self, i: usize, j: usize) -> Result<i32>;
}

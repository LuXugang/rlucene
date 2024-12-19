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
use crate::util::error::runtime_error::RuntimeError;
use crate::util::{sorter, Sorter};
use std::cmp::{max, min};

const MINRUN: i32 = 32;
const THRESHOLD: i32 = 64;
const STACKSIZE: i32 = 49; // depends on MINRUN
const MIN_GALLOP: i32 = 7;

/// [`Sorter`] implementation based on the [TimSort](http://svn.python.org/projects/python/trunk/Objects/listsort.txt) algorithm. It
/// sorts small arrays with a binary sort.
///
/// This algorithm is stable and is especially good at sorting partially-sorted arrays.
///
/// # Note
/// There are a few differences with the original implementation:
/// - The extra amount of memory to perform merges is configurable. This allows small merges to be very fast,
///   while large merges will be performed in-place (slightly slower). You can ensure that the fast merge routine
///   will always be used by having `max_temp_slots` equal to half the length of the slice of data to sort.
/// - Only the fast merge routine can gallop (the one that doesn't run in-place), and it only gallops on the longest slice.
///
/// # Note
/// This is an internal API.
pub struct TimSorter<T>
where
    T: Sorter + TimSorterBase,
{
    max_temp_slots: i32,
    min_run: i32,
    to: i32,
    stack_size: i32,
    run_ends: Vec<i32>,
    sub_sorter: T,
}
impl<T: Sorter + TimSorterBase> TimSorter<T> {
    pub fn new(max_temp_slots: i32, sub_sorter: T) -> TimSorter<T> {
        TimSorter {
            max_temp_slots,
            min_run: 0,
            to: 0,
            stack_size: 0,
            run_ends: vec![0; STACKSIZE as usize + 1],
            sub_sorter,
        }
    }
    fn min_run(&self, length: i32) -> i32 {
        debug_assert!(length >= MINRUN);
        let mut n = length;
        let mut r = 0;
        while n >= 64 {
            r |= n & 1;
            n >>= 1;
        }
        let min_run = n + r;
        debug_assert!((MINRUN..=THRESHOLD).contains(&min_run));
        min_run
    }
    fn run_len(&self, i: i32) -> i32 {
        let off = self.stack_size - i;
        self.run_ends[off as usize] - self.run_ends[(off - 1) as usize]
    }
    fn run_base(&self, i: i32) -> i32 {
        self.run_ends[(self.stack_size - i - 1) as usize]
    }
    fn run_end(&self, i: i32) -> i32 {
        self.run_ends[(self.stack_size - i) as usize]
    }
    fn set_run_end(&mut self, i: i32, run_end: i32) {
        self.run_ends[(self.stack_size - i) as usize] = run_end;
    }
    fn push_run_len(&mut self, len: i32) {
        self.run_ends[(self.stack_size + 1) as usize] =
            self.run_ends[(self.stack_size) as usize] + len;
        self.stack_size += 1;
    }
    // Compute the length of the next run, make the run sorted and return its length.
    fn next_run(&mut self) -> i32 {
        let run_base = self.run_end(0);
        debug_assert!(run_base < self.to);

        if run_base == self.to - 1 {
            return 1;
        }
        let mut o = run_base + 2;
        if self.compare(run_base, run_base + 1) > 0 {
            while o < self.to && self.compare(o - 1, o) > 0 {
                o += 1;
            }
            self.reverse(run_base, o);
        } else {
            while o < self.to && self.compare(o - 1, o) <= 0 {
                o += 1;
            }
        }
        let run_hi = max(o, min(self.to, run_base + self.min_run));
        self.binary_sort_with_start(run_base, run_hi, o);
        run_hi - run_base
    }
    pub fn ensure_invariants(&mut self) {
        while self.stack_size > 1 {
            let run_len0 = self.run_len(0);
            let run_len1 = self.run_len(1);

            if self.stack_size > 2 {
                let run_len2 = self.run_len(2);

                if run_len2 <= run_len1 + run_len0 {
                    // merge the smaller of 0 and 2 with 1
                    if run_len2 < run_len0 {
                        self.merge_at(1);
                    } else {
                        self.merge_at(0);
                    }
                    continue;
                }
            }

            if run_len1 <= run_len0 {
                self.merge_at(0);
                continue;
            }

            break;
        }
    }
    pub fn exhaust_stack(&mut self) {
        while self.stack_size > 1 {
            self.merge_at(0);
        }
    }

    pub fn reset(&mut self, from: i32, to: i32) {
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

    pub fn merge_at(&mut self, n: i32) {
        debug_assert!(self.stack_size >= 2);
        self.merge(self.run_base(n + 1), self.run_base(n), self.run_end(n));

        for j in (1..=n + 1).rev() {
            self.set_run_end(j, self.run_end(j - 1));
        }

        self.stack_size -= 1;
    }

    fn merge(&mut self, mut lo: i32, mid: i32, mut hi: i32) {
        if self.compare(mid - 1, mid) <= 0 {
            return;
        }

        lo = self.upper2(lo, mid, mid);
        hi = self.lower2(mid, hi, mid - 1);
        if hi - mid <= mid - lo && hi - mid <= self.max_temp_slots {
            self.merge_hi(lo, mid, hi);
        } else if mid - lo <= self.max_temp_slots {
            self.merge_lo(lo, mid, hi);
        } else {
            self.merge_in_place(lo, mid, hi);
        }
    }

    fn merge_lo(&mut self, lo: i32, mid: i32, hi: i32) {
        debug_assert!(self.sub_sorter.compare(lo, mid) > 0);

        let len1 = mid - lo;
        self.sub_sorter.save(lo, len1);
        self.sub_sorter.copy(mid, lo);

        let mut i = 0;
        let mut j = mid + 1;
        let mut dest = lo + 1;

        'outer: loop {
            let mut count = 0;
            while count < MIN_GALLOP {
                if i >= len1 || j >= hi {
                    break 'outer;
                } else if self.sub_sorter.compare_saved(i, j) <= 0 {
                    self.sub_sorter.restore(i, dest);
                    i += 1;
                    dest += 1;
                    count = 0;
                } else {
                    self.sub_sorter.copy(j, dest);
                    j += 1;
                    dest += 1;
                    count += 1;
                }
            }

            // Galloping phase
            let next = self.lower_saved3(j, hi, i);
            while j < next {
                self.sub_sorter.copy(j, dest);
                j += 1;
                dest += 1;
            }
            self.sub_sorter.restore(i, dest);
            i += 1;
            dest += 1;
        }

        while i < len1 {
            self.sub_sorter.restore(i, dest);
            i += 1;
            dest += 1;
        }

        assert_eq!(j, dest);
    }

    pub fn merge_hi(&mut self, lo: i32, mid: i32, hi: i32) {
        debug_assert!(self.compare(mid - 1, hi - 1) > 0);

        let len2 = hi - mid;
        self.sub_sorter.save(mid, len2);
        self.sub_sorter.copy(mid - 1, hi - 1);

        let mut i = mid - 2;
        let mut j: i32 = len2 - 1;
        let mut dest = hi - 2;

        'outer: loop {
            let mut count = 0;
            while count < MIN_GALLOP {
                if i < lo || j < 0 {
                    break 'outer;
                } else if self.sub_sorter.compare_saved(j, i) >= 0 {
                    self.sub_sorter.restore(j, dest);
                    j -= 1;
                    dest -= 1;
                    count = 0;
                } else {
                    self.sub_sorter.copy(i, dest);
                    i -= 1;
                    dest -= 1;
                    count += 1;
                }
            }

            // Galloping phase
            let next = self.upper_saved3(lo, i + 1, j);
            while i >= next {
                self.sub_sorter.copy(i, dest);
                i -= 1;
                dest -= 1;
            }
            self.sub_sorter.restore(j, dest);
            j -= 1;
            dest -= 1;
        }

        while j >= 0 {
            self.sub_sorter.restore(j, dest);
            j -= 1;
            dest -= 1;
        }

        debug_assert!(i == dest);
    }

    pub fn lower_saved(&self, mut from: i32, to: i32, val: i32) -> i32 {
        let mut len = to - from;

        while len > 0 {
            let half = len >> 1;
            let mid = from + half;
            if self.sub_sorter.compare_saved(val, mid) > 0 {
                from = mid + 1;
                len -= half + 1;
            } else {
                len = half;
            }
        }
        from
    }

    pub fn upper_saved(&self, mut from: i32, to: i32, val: i32) -> i32 {
        let mut len = to - from;

        while len > 0 {
            let half = len >> 1;
            let mid = from + half;
            if self.sub_sorter.compare_saved(val, mid) < 0 {
                len = half;
            } else {
                from = mid + 1;
                len -= half + 1;
            }
        }
        from
    }

    pub fn lower_saved3(&self, from: i32, to: i32, val: i32) -> i32 {
        let mut f = from;
        let mut t = f + 1;

        while t < to {
            if self.sub_sorter.compare_saved(val, t) <= 0 {
                return self.lower_saved(f, t, val);
            }
            let delta = t - f;
            f = t;
            t += delta * 2;
        }
        self.lower_saved(f, to, val)
    }

    pub fn upper_saved3(&self, from: i32, to: i32, val: i32) -> i32 {
        let mut f = to - 1;
        let mut t = to;

        while f > from {
            if self.sub_sorter.compare_saved(val, f) >= 0 {
                return self.upper_saved(f, t, val);
            }
            let delta = t - f;
            t = f;
            f -= delta * 2
        }
        self.upper_saved(from, t, val)
    }
}
impl<T> Sorter for TimSorter<T>
where
    T: Sorter + TimSorterBase,
{
    fn compare(&self, i: i32, j: i32) -> i32 {
        self.sub_sorter.compare(i, j)
    }

    fn swap(&mut self, i: i32, j: i32) {
        self.sub_sorter.swap(i, j);
    }

    fn set_pivot(&mut self, i: i32) {
        self.sub_sorter.set_pivot(i);
    }

    fn compare_pivot(&self, i: i32) -> i32 {
        self.sub_sorter.compare_pivot(i)
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<(), RuntimeError> {
        sorter::check_range(from, to)?;
        if to - from <= 1 {
            return Ok(());
        }

        self.reset(from, to);

        loop {
            self.ensure_invariants();
            let run_length = self.next_run();
            self.push_run_len(run_length);

            if self.run_end(0) >= to {
                break;
            }
        }
        self.exhaust_stack();

        assert_eq!(self.run_end(0), to);
        Ok(())
    }

    fn do_rotate(&mut self, mut lo: i32, mut mid: i32, hi: i32) {
        let len1 = mid - lo;
        let len2 = hi - mid;

        if len1 == len2 {
            while mid < hi {
                self.swap(lo, mid);
                lo += 1;
                mid += 1;
            }
        } else if len2 < len1 && len2 <= self.max_temp_slots {
            self.sub_sorter.save(mid, len2);
            let mut i = lo + len1 - 1;
            let mut j = hi - 1;
            while i >= lo {
                self.sub_sorter.copy(i, j);
                i -= 1;
                j -= 1;
            }
            i = 0;
            j = lo;
            while i < len2 {
                self.sub_sorter.restore(i, j);
                i += 1;
                j += 1;
            }
        } else if len1 <= self.max_temp_slots {
            self.sub_sorter.save(lo, len1);
            let mut i = mid;
            let mut j = lo;
            while i < hi {
                self.sub_sorter.copy(i, j);
                i += 1;
                j += 1;
            }
            i = 0;
            j = lo + len2;
            while j < hi {
                self.sub_sorter.restore(i, j);
                i += 1;
                j += 1;
            }
        } else {
            self.reverse(lo, mid);
            self.reverse(mid, hi);
            self.reverse(lo, hi);
        }
    }
}

pub trait TimSorterBase {
    ///Copy data from slot `src` to slot `dest`
    fn copy(&mut self, src: i32, dest: i32);

    /// Save all elements between slots i and `i+len` into the temporary
    /// storage.
    fn save(&mut self, i: i32, len: i32);
    /// Restore element `j` from the temporary storage into slot `i`.
    fn restore(&mut self, i: i32, j: i32);

    /// Compare element `i` from the temporary storage with element `j` from the
    /// slice to sort, similarly to #compare(i32, i32).
    fn compare_saved(&self, i: i32, j: i32) -> i32;
}

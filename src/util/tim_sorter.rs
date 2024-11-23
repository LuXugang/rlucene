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
use crate::util::tim_sorter_base::TimSorterBase;
use crate::util::{sorter, Sorter};
use std::cmp::{max, min};

/**
 * Sorter implementation based on the
 * href="http://svn.python.org/projects/python/trunk/Objects/listsort.txt">TimSort</a> algorithm. It
 * sorts small arrays with a binary sort.
 *
 * This algorithm is stable. It's especially good at sorting partially-sorted arrays.
 *
 * NOTE:There are a few differences with the original implementation:
 *
 * `maxTempSlots` The extra amount of memory to perform merges is configurable. This
 *       allows small merges to be very fast while large merges will be performed in-place (slightly
 *       slower). You can make sure that the fast merge routine will always be used by having
 *       `maxTempSlots` equal to half of the length of the slice of data to sort.
 *       Only the fast merge routine can gallop (the one that doesn't run in-place) and it only
 *       gallops on the longest slice.
 *
 */
const MINRUN: usize = 32;
const THRESHOLD: usize = 64;
const STACKSIZE: usize = 49; // depends on MINRUN
const MIN_GALLOP: usize = 7;

pub struct TimSorter<T>
where
    T: Sorter + TimSorterBase,
{
    max_temp_slots: usize,
    min_run: usize,
    to: usize,
    stack_size: usize,
    run_ends: Vec<usize>,
    sub_sorter: T,
}
impl<T: Sorter + TimSorterBase> TimSorter<T> {
    pub fn new(max_temp_slots: usize, sub_sorter: T) -> TimSorter<T> {
        TimSorter {
            max_temp_slots,
            min_run: 0,
            to: 0,
            stack_size: 0,
            run_ends: vec![0; STACKSIZE + 1],
            sub_sorter,
        }
    }
    fn min_run(&self, length: usize) -> usize {
        assert!(length >= MINRUN);
        let mut n = length;
        let mut r = 0;
        while n >= 64 {
            r |= n & 1;
            n >>= 1; //
        }
        let min_run = n + r;
        assert!((MINRUN..=THRESHOLD).contains(&min_run));
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
    /** Compute the length of the next run, make the run sorted and return its length. */
    fn next_run(&mut self) -> usize {
        let run_base = self.run_end(0);
        assert!(run_base < self.to);

        if run_base == self.to - 1 {
            return 1;
        }
        let mut o = run_base + 2;
        if self.sub_sorter.compare(run_base, run_base + 1) > 0 {
            while o < self.to && self.sub_sorter.compare(o - 1, o) > 0 {
                o += 1;
            }
            self.sub_sorter.reverse(run_base, o);
        } else {
            while o < self.to && self.sub_sorter.compare(o - 1, o) <= 0 {
                o += 1;
            }
        }
        let run_hi = max(o, min(self.to, run_base + self.min_run));
        self.sub_sorter.binary_sort_with_start(run_base, run_hi, o);
        run_hi - run_base
    }
    pub fn ensure_invariants(&mut self) {
        while self.stack_size > 1 {
            let run_len0 = self.run_len(0);
            let run_len1 = self.run_len(1);

            if self.stack_size > 2 {
                let run_len2 = self.run_len(2);

                if run_len2 <= run_len1 + run_len0 {
                    // 合并 0 和 2 中较小的一个与 1
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
    pub fn exhaust_stack(&mut self)
    where
        T: TimSorterBase + Sorter,
    {
        while self.stack_size > 1 {
            self.merge_at(0);
        }
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

    pub fn merge_at(&mut self, n: usize) {
        assert!(self.stack_size >= 2);
        self.merge(
            self.run_base(n + 1),
            self.run_base(n),
            self.run_end(n),
        );

        for j in (1..=n + 1).rev() {
            self.set_run_end(j, self.run_end(j - 1));
        }

        self.stack_size -= 1;
    }

    pub fn merge(&mut self, mut lo: usize, mid: usize, mut hi: usize) {
        if self.sub_sorter.compare(mid - 1, mid) <= 0 {
            return;
        }

        lo = self.sub_sorter.upper2(lo, mid, mid);
        hi = self.sub_sorter.lower2(mid, hi, mid - 1);

        if hi - mid <= mid - lo && hi - mid <= self.max_temp_slots {
            self.merge_hi(lo, mid, hi);
        } else if mid - lo <= self.max_temp_slots {
            self.merge_lo(lo, mid, hi);
        } else {
            self.sub_sorter.merge_in_place(lo, mid, hi);
        }
    }

    pub fn sort(&mut self, from: usize, to: usize) -> Result<(), String> {
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

        assert!(self.run_end(0) == to);
        Ok(())
    }
    pub fn do_rotate(&mut self, lo: usize, mid: usize, hi: usize) {
        let len1 = mid - lo;
        let len2 = hi - mid;

        if len1 == len2 {
            let mut lo_idx = lo;
            let mut mid_idx = mid;
            while mid_idx < hi {
                self.sub_sorter.swap(lo_idx, mid_idx);
                lo_idx += 1;
                mid_idx += 1;
            }
        } else if len2 < len1 && len2 <= self.max_temp_slots {
            self.sub_sorter.save(mid, len2);
            for (i, j) in (lo..lo + len1).rev().zip((hi - len2..hi).rev()) {
                self.sub_sorter.copy(i, j);
            }
            for (i, j) in (0..len2).zip(lo..lo + len2) {
                self.sub_sorter.restore(i, j);
            }
        } else if len1 <= self.max_temp_slots {
            // len1 较小且临时空间足够
            self.sub_sorter.save(lo, len1);
            for (i, j) in (mid..hi).zip(lo..lo + len2) {
                self.sub_sorter.copy(i, j);
            }
            for (i, j) in (0..len1).zip(lo + len2..hi) {
                self.sub_sorter.restore(i, j);
            }
        } else {
            // 使用反转实现旋转
            self.sub_sorter.reverse(lo, mid);
            self.sub_sorter.reverse(mid, hi);
            self.sub_sorter.reverse(lo, hi);
        }
    }
    pub fn merge_lo(&mut self, lo: usize, mid: usize, hi: usize) {
        assert!(self.sub_sorter.compare(lo, mid) > 0);

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

    pub fn merge_hi(&mut self, lo: usize, mid: usize, hi: usize)
    where
        T: Sorter + TimSorterBase,
    {
        assert!(
            self.sub_sorter.compare(mid - 1, hi - 1) > 0,
            "mergeHi precondition failed"
        );

        let len2 = hi - mid;
        self.sub_sorter.save(mid, len2);
        self.sub_sorter.copy(mid - 1, hi - 1);

        let mut i = mid - 2;
        let mut j: i32 = (len2 - 1) as i32;
        let mut dest = hi - 2;

        'outer: loop {
            let mut count = 0;
            while count < MIN_GALLOP {
                if i < lo || j < 0 {
                    break 'outer;
                } else if self.sub_sorter.compare_saved(j as usize, i) >= 0 {
                    self.sub_sorter.restore(j as usize, dest);
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
            let next = self.upper_saved3(lo, i + 1, j as usize);
            while i >= next {
                self.sub_sorter.copy(i, dest);
                i -= 1;
                dest -= 1;
            }
            self.sub_sorter.restore(j as usize, dest);
            j -= 1;
            dest -= 1;
        }

        while j >= 0 {
            self.sub_sorter.restore(j as usize, dest);
            j -= 1;
            dest -= 1;
        }

        assert!(i == dest);
    }

    pub fn lower_saved(&self, from: usize, to: usize, val: usize) -> usize
    where
        T: Sorter + TimSorterBase,
    {
        let mut len = to - from;
        let mut start = from;

        while len > 0 {
            let half = len / 2;
            let mid = start + half;
            if self.sub_sorter.compare_saved(val, mid) > 0 {
                start = mid + 1;
                len -= half + 1;
            } else {
                len = half;
            }
        }
        start
    }

    pub fn upper_saved(&self, from: usize, to: usize, val: usize) -> usize
    where
        T: Sorter + TimSorterBase,
    {
        let mut len = to - from;
        let mut start = from;

        while len > 0 {
            let half = len / 2;
            let mid = start + half;
            if self.sub_sorter.compare_saved(val, mid) < 0 {
                len = half;
            } else {
                start = mid + 1;
                len -= half + 1;
            }
        }
        start
    }

    pub fn lower_saved3(&self, from: usize, to: usize, val: usize) -> usize
    where
        T: Sorter + TimSorterBase,
    {
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

    pub fn upper_saved3(&self, from: usize, to: usize, val: usize) -> usize
    where
        T: Sorter + TimSorterBase,
    {
        let mut f = to - 1;
        let mut t = to;

        while f > from {
            if self.sub_sorter.compare_saved(val, f) >= 0 {
                return self.upper_saved(f, t, val);
            }
            let delta = t - f;
            t = f;
            f = f.saturating_sub(delta * 2);
        }
        self.upper_saved(from, t, val)
    }
}

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
use std::sync::Arc;

use chrono::{DateTime, TimeZone, Timelike};
use chrono_tz::Tz;
use parking_lot::Mutex;
use rand::RngExt;
use rand::prelude::StdRng;

use crate::core::index::log_merge_policy::{LogMergePolicy, LogMergePolicyBase};
use crate::core::index::merge_policy::{
  DEFAULT_MAX_CFS_SEGMENT_SIZE, MergeContext, MergePolicyBase,
};
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::test_util::TestUtil;

/// Merge policy for testing, it is like an alcoholic. It drinks (merges) at night, and randomly
/// decides what to drink. During the daytime it sleeps.
///
/// If tests pass with this, then they are likely to pass with any bizarro merge policy users might
/// write.
///
/// It is a fine bottle of champagne (Ordered by Martijn).
#[derive(Clone)]
pub struct AlcoholicMergePolicy {
  random: Arc<Mutex<StdRng>>,
  calendar: DateTime<Tz>,
}

impl AlcoholicMergePolicy {
  pub fn new(time_zone: Tz, mut random: StdRng) -> LogMergePolicy<Self> {
    let time_in_millis = TestUtil::next_long(&mut random, 0, i64::MAX);
    let calendar = time_zone
      .timestamp_millis_opt(time_in_millis)
      .single()
      .unwrap_or_else(|| {
        // Java's GregorianCalendar accepts every non-negative i64 millisecond value, while
        // chrono's DateTime has a narrower year range. The Gregorian calendar repeats every
        // 400 years, so normalize otherwise-unrepresentable instants into a future 400-year
        // cycle. IANA zones use their recurring future transition rules in this range.
        const MILLIS_PER_400_YEARS: i64 = 146_097 * 24 * 60 * 60 * 1000;
        time_zone
          .timestamp_millis_opt(
            MILLIS_PER_400_YEARS + time_in_millis.rem_euclid(MILLIS_PER_400_YEARS),
          )
          .single()
          .unwrap()
      });
    let max_merge_size = TestUtil::next_int(&mut random, 1024 * 1024, i32::MAX) as i64;

    LogMergePolicy {
      merge_factor: LogMergePolicy::<Self>::DEFAULT_MERGE_FACTOR,
      min_merge_size: 0,
      max_merge_size,
      max_merge_size_for_forced_merge: i64::MAX,
      max_merge_docs: LogMergePolicy::<Self>::DEFAULT_MAX_MERGE_DOCS,
      calibrate_size_by_deletes: true,
      target_search_concurrency: 1,
      base: MergePolicyBase::new(
        LogMergePolicy::<Self>::DEFAULT_NO_CFS_RATIO,
        DEFAULT_MAX_CFS_SEGMENT_SIZE,
      ),
      sub: Self {
        random: Arc::new(Mutex::new(random)),
        calendar,
      },
    }
  }
}

impl LogMergePolicyBase for AlcoholicMergePolicy {
  // @BlackMagic(level=Voodoo);
  #[allow(clippy::manual_range_contains)]
  fn size<D, MC>(
    &self,
    info: &SegmentCommitInfo<D>,
    _merge_context: &MC,
    _calibrate_size_by_deletes: bool,
  ) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    let mut random = self.random.lock();
    let hour_of_day = self.calendar.hour();
    if hour_of_day < 6
      || hour_of_day > 20
      ||
      // It's 5 o'clock somewhere.
      random.random_range(0..23) == 5
    {
      let values = Drink::VALUES;
      // Pick a random drink during the day.
      return Ok(
        values[random.random_range(0..values.len())]
          .drunk_factor()
          .wrapping_mul(info.size_in_bytes()?),
      );
    }

    info.size_in_bytes()
  }
}

#[derive(Clone, Copy)]
enum Drink {
  Beer,
  Wine,
  Champagne,
  WhiteRussian,
  SingleMalt,
}

impl Drink {
  const VALUES: [Self; 5] = [
    Self::Beer,
    Self::Wine,
    Self::Champagne,
    Self::WhiteRussian,
    Self::SingleMalt,
  ];

  fn drunk_factor(self) -> i64 {
    match self {
      Self::Beer => 15,
      Self::Wine => 17,
      Self::Champagne => 21,
      Self::WhiteRussian => 22,
      Self::SingleMalt => 30,
    }
  }
}

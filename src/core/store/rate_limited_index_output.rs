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

use std::fmt::{Display, Formatter};

use crate::core::store::data_output::DataOutput;
use crate::core::store::index_output::IndexOutput;
use crate::core::store::rate_limiter::RateLimiter;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;

/// A [`RateLimiter`] rate limiting [`IndexOutput`].
///
/// @lucene.internal
pub struct RateLimitedIndexOutput<O, R>
where
  O: IndexOutput,
  R: RateLimiter,
{
  out: O,
  rate_limiter: R,
  /// How many bytes we've written since we last called [`RateLimiter::pause`].
  bytes_since_last_pause: i64,
  /// Cached here to not always have to call [`RateLimiter::get_min_pause_check_bytes`].
  current_min_pause_check_bytes: i64,
}

impl<O, R> RateLimitedIndexOutput<O, R>
where
  O: IndexOutput,
  R: RateLimiter,
{
  pub fn new(rate_limiter: R, out: O) -> Self {
    let current_min_pause_check_bytes = rate_limiter.get_min_pause_check_bytes();
    Self {
      out,
      rate_limiter,
      bytes_since_last_pause: 0,
      current_min_pause_check_bytes,
    }
  }

  fn check_rate(&mut self) -> Result<()> {
    if self.bytes_since_last_pause > self.current_min_pause_check_bytes {
      self.rate_limiter.pause(self.bytes_since_last_pause)?;
      self.bytes_since_last_pause = 0;
      self.current_min_pause_check_bytes = self.rate_limiter.get_min_pause_check_bytes();
    }
    Ok(())
  }
}

impl<O, R> DataOutput for RateLimitedIndexOutput<O, R>
where
  O: IndexOutput,
  R: RateLimiter,
{
  fn write_byte(&mut self, b: u8) -> Result<()> {
    self.bytes_since_last_pause += 1;
    self.check_rate()?;
    self.out.write_byte(b)
  }

  fn write_bytes_with_len(&mut self, b: &[u8], len: usize) -> Result<()> {
    self.write_bytes_range(b, 0, len)
  }

  fn write_bytes_range(&mut self, b: &[u8], offset: usize, length: usize) -> Result<()> {
    self.bytes_since_last_pause += length as i64;
    self.check_rate()?;
    // The bytes array slice is written without pauses.
    // This can cause instant write rate to breach rate limit if there have
    // been no writes for enough time to keep the average write rate within limit.
    // See https://issues.apache.org/jira/browse/LUCENE-10448
    self.out.write_bytes_range(b, offset, length)
  }

  fn write_int(&mut self, i: i32) -> Result<()> {
    self.bytes_since_last_pause += std::mem::size_of::<i32>() as i64;
    self.check_rate()?;
    self.out.write_int(i)
  }

  fn write_short(&mut self, i: i16) -> Result<()> {
    self.bytes_since_last_pause += std::mem::size_of::<i16>() as i64;
    self.check_rate()?;
    self.out.write_short(i)
  }

  fn write_long(&mut self, i: i64) -> Result<()> {
    self.bytes_since_last_pause += std::mem::size_of::<i64>() as i64;
    self.check_rate()?;
    self.out.write_long(i)
  }
}

impl<O, R> Display for RateLimitedIndexOutput<O, R>
where
  O: IndexOutput,
  R: RateLimiter,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "RateLimitedIndexOutput({})", self.out)
  }
}

impl<O, R> Closeable for RateLimitedIndexOutput<O, R>
where
  O: IndexOutput,
  R: RateLimiter,
{
  fn close(&mut self) -> Result<()> {
    self.out.close()
  }
}

impl<O, R> IndexOutput for RateLimitedIndexOutput<O, R>
where
  O: IndexOutput,
  R: RateLimiter,
{
  fn get_file_pointer(&self) -> Result<usize> {
    self.out.get_file_pointer()
  }

  fn get_checksum(&mut self) -> Result<u64> {
    self.out.get_checksum()
  }

  fn get_name(&self) -> &str {
    self.out.get_name()
  }

  fn align_file_pointer(&mut self, alignment_bytes: usize) -> Result<usize> {
    self.out.align_file_pointer(alignment_bytes)
  }
}

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
use std::thread;
use std::time::{Duration, Instant};

use crate::core::store::{DataOutput, IndexOutput};
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;

/// Intentionally slow IndexOutput for testing.
pub struct ThrottledIndexOutput<O> {
  bytes_per_second: i32,
  #[allow(dead_code)]
  flush_delay_millis: i64,
  close_delay_millis: i64,
  #[allow(dead_code)]
  seek_delay_millis: i64,
  pending_bytes: i64,
  min_bytes_written: i64,
  time_elapsed: i64,
  out: O,
  resource_description: String,
  name: String,
}

impl<O> ThrottledIndexOutput<O>
where
  O: IndexOutput,
{
  pub const DEFAULT_MIN_WRITTEN_BYTES: i32 = 1024;

  #[allow(unused)]
  pub fn new_from_delegate<D>(&self, out: D) -> ThrottledIndexOutput<D>
  where
    D: IndexOutput,
  {
    ThrottledIndexOutput::with_all_delays(
      self.bytes_per_second,
      self.flush_delay_millis,
      self.close_delay_millis,
      self.seek_delay_millis,
      self.min_bytes_written,
      out,
    )
  }

  pub fn new(bytes_per_second: i32, delay_in_millis: i64, out: O) -> Self {
    Self::with_all_delays(
      bytes_per_second,
      delay_in_millis,
      delay_in_millis,
      delay_in_millis,
      Self::DEFAULT_MIN_WRITTEN_BYTES as i64,
      out,
    )
  }

  #[allow(unused)]
  pub fn with_delays(bytes_per_second: i32, delays: i64, min_bytes_written: i32, out: O) -> Self {
    Self::with_all_delays(
      bytes_per_second,
      delays,
      delays,
      delays,
      min_bytes_written as i64,
      out,
    )
  }

  #[allow(unused)]
  pub fn m_bits_to_bytes(mbits: i32) -> i32 {
    mbits.wrapping_mul(125_000_000)
  }

  pub fn with_all_delays(
    bytes_per_second: i32,
    flush_delay_millis: i64,
    close_delay_millis: i64,
    seek_delay_millis: i64,
    min_bytes_written: i64,
    out: O,
  ) -> Self {
    assert!(bytes_per_second > 0);
    let resource_description = format!("ThrottledIndexOutput({out})");
    let name = out.get_name().to_string();
    Self {
      bytes_per_second,
      flush_delay_millis,
      close_delay_millis,
      seek_delay_millis,
      pending_bytes: 0,
      min_bytes_written,
      time_elapsed: 0,
      out,
      resource_description,
      name,
    }
  }

  fn get_delay(&mut self, closing: bool) -> i64 {
    if self.pending_bytes > 0 && (closing || self.pending_bytes > self.min_bytes_written) {
      let actual_bps = (self.time_elapsed / self.pending_bytes) * 1_000_000_000;
      if actual_bps > self.bytes_per_second as i64 {
        let expected = self.pending_bytes * 1000 / self.bytes_per_second as i64;
        let delay = expected - Duration::from_nanos(self.time_elapsed as u64).as_millis() as i64;
        self.pending_bytes = 0;
        self.time_elapsed = 0;
        return delay;
      }
    }
    0
  }

  fn sleep(ms: i64) {
    if ms <= 0 {
      return;
    }
    thread::sleep(Duration::from_millis(ms as u64));
  }
}

impl<O> DataOutput for ThrottledIndexOutput<O>
where
  O: IndexOutput,
{
  fn write_byte(&mut self, b: u8) -> Result<()> {
    let bytes = [b];
    self.write_bytes_range(&bytes, 0, 1)
  }

  fn write_bytes_range(&mut self, b: &[u8], offset: usize, length: usize) -> Result<()> {
    let before = Instant::now();
    self.out.write_bytes_range(b, offset, length)?;
    self.time_elapsed += before.elapsed().as_nanos().min(i64::MAX as u128) as i64;
    self.pending_bytes += length as i64;
    Self::sleep(self.get_delay(false));
    Ok(())
  }
}

impl<O> Display for ThrottledIndexOutput<O> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.resource_description)
  }
}

impl<O> Closeable for ThrottledIndexOutput<O>
where
  O: IndexOutput,
{
  fn close(&mut self) -> Result<()> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> {
      Self::sleep(self.close_delay_millis + self.get_delay(true));
      Ok(())
    }));
    let close_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.out.close()));
    crate::core::util::io_utils::IOUtils::finally_caught_result(result, close_result)
  }
}

impl<O> IndexOutput for ThrottledIndexOutput<O>
where
  O: IndexOutput,
{
  fn get_file_pointer(&self) -> Result<usize> {
    self.out.get_file_pointer()
  }

  fn get_checksum(&mut self) -> Result<u64> {
    self.out.get_checksum()
  }

  fn get_name(&self) -> &str {
    &self.name
  }
}

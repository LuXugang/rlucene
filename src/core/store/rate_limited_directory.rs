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

use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

use crate::core::index::index_reader::Identity;
use crate::core::store::data_input::DataInput;
use crate::core::store::data_output::DataOutput;
use crate::core::store::directory::Directory;
use crate::core::store::index_output::IndexOutput;
use crate::core::store::rate_limited_index_output::RateLimitedIndexOutput;
use crate::core::store::rate_limiter::RateLimiter;
use crate::core::store::{Context, IOContext};
use crate::core::util::HasIdentity;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::Result;

pub enum RateLimitedIndexOutputEnum<O, R> {
  A(RateLimitedIndexOutput<O, R>),
  B(O),
}

impl<O, R> DataOutput for RateLimitedIndexOutputEnum<O, R>
where
  O: IndexOutput,
  R: RateLimiter,
{
  fn write_byte(&mut self, b: u8) -> Result<()> {
    match self {
      Self::A(output) => output.write_byte(b),
      Self::B(output) => output.write_byte(b),
    }
  }

  fn write_bytes_with_len(&mut self, b: &[u8], len: usize) -> Result<()> {
    match self {
      Self::A(output) => output.write_bytes_with_len(b, len),
      Self::B(output) => output.write_bytes_with_len(b, len),
    }
  }

  fn write_bytes_range(&mut self, b: &[u8], offset: usize, length: usize) -> Result<()> {
    match self {
      Self::A(output) => output.write_bytes_range(b, offset, length),
      Self::B(output) => output.write_bytes_range(b, offset, length),
    }
  }

  fn write_int(&mut self, i: i32) -> Result<()> {
    match self {
      Self::A(output) => output.write_int(i),
      Self::B(output) => output.write_int(i),
    }
  }

  fn write_short(&mut self, i: i16) -> Result<()> {
    match self {
      Self::A(output) => output.write_short(i),
      Self::B(output) => output.write_short(i),
    }
  }

  fn write_vint(&mut self, i: i32) -> Result<()> {
    match self {
      Self::A(output) => output.write_vint(i),
      Self::B(output) => output.write_vint(i),
    }
  }

  fn write_zint(&mut self, i: i32) -> Result<()> {
    match self {
      Self::A(output) => output.write_zint(i),
      Self::B(output) => output.write_zint(i),
    }
  }

  fn write_long(&mut self, i: i64) -> Result<()> {
    match self {
      Self::A(output) => output.write_long(i),
      Self::B(output) => output.write_long(i),
    }
  }

  fn write_vlong(&mut self, i: i64) -> Result<()> {
    match self {
      Self::A(output) => output.write_vlong(i),
      Self::B(output) => output.write_vlong(i),
    }
  }

  fn write_signed_vlong(&mut self, i: i64) -> Result<()> {
    match self {
      Self::A(output) => output.write_signed_vlong(i),
      Self::B(output) => output.write_signed_vlong(i),
    }
  }

  fn write_zlong(&mut self, i: i64) -> Result<()> {
    match self {
      Self::A(output) => output.write_zlong(i),
      Self::B(output) => output.write_zlong(i),
    }
  }

  fn write_string(&mut self, s: &str) -> Result<()> {
    match self {
      Self::A(output) => output.write_string(s),
      Self::B(output) => output.write_string(s),
    }
  }

  fn copy_bytes<I>(&mut self, input: &mut I, num_bytes: usize) -> Result<()>
  where
    I: DataInput + ?Sized,
  {
    match self {
      Self::A(output) => output.copy_bytes(input, num_bytes),
      Self::B(output) => output.copy_bytes(input, num_bytes),
    }
  }

  fn write_map_of_strings(&mut self, map: &HashMap<String, String>) -> Result<()> {
    match self {
      Self::A(output) => output.write_map_of_strings(map),
      Self::B(output) => output.write_map_of_strings(map),
    }
  }

  fn write_set_of_strings(&mut self, set: &HashSet<String>) -> Result<()> {
    match self {
      Self::A(output) => output.write_set_of_strings(set),
      Self::B(output) => output.write_set_of_strings(set),
    }
  }
}

impl<O, R> Display for RateLimitedIndexOutputEnum<O, R>
where
  O: Display,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::A(output) => output.fmt(f),
      Self::B(output) => output.fmt(f),
    }
  }
}

impl<O, R> Closeable for RateLimitedIndexOutputEnum<O, R>
where
  O: Closeable,
{
  fn close(&mut self) -> Result<()> {
    match self {
      Self::A(output) => output.close(),
      Self::B(output) => output.close(),
    }
  }
}

impl<O, R> IndexOutput for RateLimitedIndexOutputEnum<O, R>
where
  O: IndexOutput,
  R: RateLimiter,
{
  fn get_file_pointer(&self) -> Result<usize> {
    match self {
      Self::A(output) => output.get_file_pointer(),
      Self::B(output) => output.get_file_pointer(),
    }
  }

  fn get_checksum(&mut self) -> Result<u64> {
    match self {
      Self::A(output) => output.get_checksum(),
      Self::B(output) => output.get_checksum(),
    }
  }

  fn get_name(&self) -> &str {
    match self {
      Self::A(output) => output.get_name(),
      Self::B(output) => output.get_name(),
    }
  }

  fn align_file_pointer(&mut self, alignment_bytes: usize) -> Result<usize> {
    match self {
      Self::A(output) => output.align_file_pointer(alignment_bytes),
      Self::B(output) => output.align_file_pointer(alignment_bytes),
    }
  }
}

/// A delegating [`Directory`] that rate-limits created outputs.
pub struct RateLimitedDirectory<D, R> {
  in_: D,
  rate_limiter: R,
  id: Identity,
}

impl<D, R> RateLimitedDirectory<D, R> {
  pub fn new(in_: D, rate_limiter: R) -> Self {
    Self {
      in_,
      rate_limiter,
      id: Identity::new(),
    }
  }
}

impl<D, R> Display for RateLimitedDirectory<D, R>
where
  D: Display,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "RateLimitedDirectory({})", self.in_)
  }
}

impl<D, R> CloseableRef for RateLimitedDirectory<D, R>
where
  D: CloseableRef,
{
  fn close(&self) -> Result<()> {
    self.in_.close()
  }
}

impl<D, R> HasIdentity for RateLimitedDirectory<D, R> {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl<D, R> Directory for RateLimitedDirectory<D, R>
where
  D: Directory,
  R: RateLimiter + Clone,
{
  fn list_all(&self) -> Result<Vec<String>> {
    self.in_.list_all()
  }

  fn delete_file(&self, name: &str) -> Result<()> {
    self.in_.delete_file(name)
  }

  fn file_length(&self, name: &str) -> Result<usize> {
    self.in_.file_length(name)
  }

  type IndexOutput = RateLimitedIndexOutputEnum<D::IndexOutput, R>;

  fn create_output(&self, name: &str, context: &IOContext) -> Result<Self::IndexOutput> {
    self.ensure_open()?;
    debug_assert!(
      matches!(context.get_context(), &Context::Merge),
      "got context={:?}",
      context.get_context()
    );
    Ok(RateLimitedIndexOutputEnum::A(RateLimitedIndexOutput::new(
      self.rate_limiter.clone(),
      self.in_.create_output(name, context)?,
    )))
  }

  fn create_temp_output(
    &self,
    prefix: &str,
    suffix: &str,
    context: &IOContext,
  ) -> Result<Self::IndexOutput> {
    self.ensure_open()?;
    Ok(RateLimitedIndexOutputEnum::B(
      self.in_.create_temp_output(prefix, suffix, context)?,
    ))
  }

  fn sync(&self, names: &[String]) -> Result<()> {
    self.in_.sync(names)
  }

  fn sync_metadata(&self) -> Result<()> {
    self.in_.sync_metadata()
  }

  fn rename(&self, source: &str, dest: &str) -> Result<()> {
    self.in_.rename(source, dest)
  }

  type IndexInput = D::IndexInput;

  fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInput> {
    self.in_.open_input(name, context)
  }

  type Lock = D::Lock;

  fn obtain_lock(&self, name: &str) -> Result<Self::Lock> {
    self.in_.obtain_lock(name)
  }

  fn get_pending_deletions(&self) -> Result<HashSet<String>> {
    self.in_.get_pending_deletions()
  }

  #[cfg(debug_assertions)]
  fn is_fs_directory(&self) -> bool {
    self.in_.is_fs_directory()
  }

  fn ensure_open(&self) -> Result<()> {
    self.in_.ensure_open()
  }
}

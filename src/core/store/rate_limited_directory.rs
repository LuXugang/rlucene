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

use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use crate::core::index::index_reader::Identity;
use crate::core::store::directory::Directory;
use crate::core::store::rate_limited_index_output::RateLimitedIndexOutput;
use crate::core::store::rate_limiter::RateLimiter;
use crate::core::store::{Context, IOContext, IndexOutputEnum2};
use crate::core::util::HasIdentity;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;

/// A delegating [`Directory`] that rate-limits created outputs.
pub struct RateLimitedDirectory<D, R>
where
  D: Directory,
  R: RateLimiter + Clone,
{
  in_: D,
  rate_limiter: R,
  id: Identity,
}

impl<D, R> RateLimitedDirectory<D, R>
where
  D: Directory,
  R: RateLimiter + Clone,
{
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
  D: Directory,
  R: RateLimiter + Clone,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "RateLimitedDirectory({})", self.in_)
  }
}

impl<D, R> CloseableRef for RateLimitedDirectory<D, R>
where
  D: Directory,
  R: RateLimiter + Clone,
{
  fn close(&self) -> Result<()> {
    self.in_.close()
  }
}

impl<D, R> HasIdentity for RateLimitedDirectory<D, R>
where
  D: Directory,
  R: RateLimiter + Clone,
{
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

  type IndexOutput = IndexOutputEnum2<RateLimitedIndexOutput<D::IndexOutput, R>, D::IndexOutput>;

  fn create_output(&self, name: &str, context: &IOContext) -> Result<Self::IndexOutput> {
    self.ensure_open()?;
    debug_assert!(
      matches!(context.get_context(), &Context::Merge),
      "got context={:?}",
      context.get_context()
    );
    Ok(IndexOutputEnum2::A(RateLimitedIndexOutput::new(
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
    Ok(IndexOutputEnum2::B(
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

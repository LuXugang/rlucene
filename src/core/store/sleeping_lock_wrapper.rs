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
use crate::core::index::index_reader::Identity;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::HasIdentity;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::thread;
use std::time::Duration;

/// Directory that wraps another, and that sleeps and retries if obtaining the lock fails.
///
/// This is not a good idea.
pub struct SleepingLockWrapper<D> {
  in_: D,
  lock_wait_timeout: i64,
  poll_interval: i64,
  id: Identity,
}

impl<D> SleepingLockWrapper<D> {
  /// Pass this lockWaitTimeout to try forever to obtain the lock.
  pub const LOCK_OBTAIN_WAIT_FOREVER: i64 = -1;

  /// How long `obtain_lock` waits, in milliseconds, in between attempts to acquire the lock.
  pub const DEFAULT_POLL_INTERVAL: i64 = 1000;

  /// Create a new SleepingLockFactory.
  pub fn new(delegate: D, lock_wait_timeout: i64) -> Result<Self> {
    Self::with_poll_interval(delegate, lock_wait_timeout, Self::DEFAULT_POLL_INTERVAL)
  }

  /// Create a new SleepingLockFactory.
  pub fn with_poll_interval(
    delegate: D,
    lock_wait_timeout: i64,
    poll_interval: i64,
  ) -> Result<Self> {
    if lock_wait_timeout < 0 && lock_wait_timeout != Self::LOCK_OBTAIN_WAIT_FOREVER {
      return Err(LuceneError::illegal_argument(format!(
        "lockWaitTimeout should be LOCK_OBTAIN_WAIT_FOREVER or a non-negative number (got {lock_wait_timeout})"
      )));
    }
    if poll_interval < 0 {
      return Err(LuceneError::illegal_argument(format!(
        "pollInterval must be a non-negative number (got {poll_interval})"
      )));
    }
    Ok(Self {
      in_: delegate,
      lock_wait_timeout,
      poll_interval,
      id: Identity::new(),
    })
  }

  /// Return the wrapped `Directory`.
  pub fn get_delegate(&self) -> &D {
    &self.in_
  }
}

impl<D> Display for SleepingLockWrapper<D>
where
  D: Display,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "SleepingLockWrapper({})", self.in_)
  }
}

impl<D> CloseableRef for SleepingLockWrapper<D>
where
  D: CloseableRef,
{
  fn close(&self) -> Result<()> {
    self.in_.close()
  }
}

impl<D> HasIdentity for SleepingLockWrapper<D> {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl<D> Directory for SleepingLockWrapper<D>
where
  D: Directory,
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

  type IndexOutput = D::IndexOutput;

  fn create_output(&self, name: &str, context: &IOContext) -> Result<Self::IndexOutput> {
    self.in_.create_output(name, context)
  }

  fn create_temp_output(
    &self,
    prefix: &str,
    suffix: &str,
    context: &IOContext,
  ) -> Result<Self::IndexOutput> {
    self.in_.create_temp_output(prefix, suffix, context)
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
    let mut failure_reason = None;
    let max_sleep_count = if self.poll_interval == 0 {
      self.lock_wait_timeout
    } else {
      self.lock_wait_timeout / self.poll_interval
    };
    let mut sleep_count = 0;

    loop {
      match self.in_.obtain_lock(name) {
        Ok(lock) => return Ok(lock),
        Err(err @ LuceneError::LockObtainFailed(_)) => {
          if failure_reason.is_none() {
            failure_reason = Some(err);
          }
        },
        Err(err) => return Err(err),
      }

      thread::sleep(Duration::from_millis(self.poll_interval as u64));

      let should_continue =
        sleep_count < max_sleep_count || self.lock_wait_timeout == Self::LOCK_OBTAIN_WAIT_FOREVER;
      sleep_count += 1;
      if !should_continue {
        break;
      }
    }

    let failure_reason = failure_reason
      .unwrap_or_else(|| LuceneError::lock_obtain_failed(format!("Lock obtain timed out: {self}")));
    let reason = format!("Lock obtain timed out: {self}: {failure_reason}");
    let mut error = LuceneError::lock_obtain_failed(reason);
    error.add_suppressed(failure_reason);
    Err(error)
  }

  fn copy_from<T>(&self, from: &T, src: &str, dest: &str, context: &IOContext) -> Result<()>
  where
    T: Directory + ?Sized,
  {
    self.in_.copy_from(from, src, dest, context)
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

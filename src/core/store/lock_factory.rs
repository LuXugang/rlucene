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
use std::fmt::Display;
use std::path::Path;

use crate::impl_from_for_enum;

use std::fmt::Formatter;

use crate::core::store::lock::{Lock, LockEnum};
use crate::core::store::native_fs_lock_factory::NativeFSLockFactory;
use crate::core::store::no_lock_factory::NoLockFactory;
use crate::core::store::simple_fs_lock_factory::SimpleFSLockFactory;
use crate::core::store::single_instance_lock_factory::SingleInstanceLockFactory;
use crate::core::util::error::lucene_error::Result;

/// Base trait for locking implementations. `Directory` uses instances of this
/// trait to implement locking.
///
/// # Default Implementation
/// Lucene uses [`NativeFSLockFactory`](crate::core::store::NativeFSLockFactory) by
/// default for `FSDirectory`-based index directories.
///
/// # Note
/// Special care needs to be taken if you change the locking implementation:
/// First, ensure that no writer is actively writing to the index, as doing so
/// could corrupt the index. Be sure to change the `LockFactory` on all Lucene
/// instances and clean up any leftover lock files before starting with the new
/// configuration. Different implementations cannot work together.
pub trait LockFactory: Display {
  type Lock: Lock;
  /// Returns a new got `Lock` instance identified by `lock_name`.
  ///
  /// # Arguments
  /// * `lock_name` - The name of the lock to be created.
  ///
  /// # Errors
  /// - Returns a `LockObtainFailedError` (optional specific error) if
  ///   the lock could not be obtained because it is currently held elsewhere.
  /// - Returns an `std::io::Error` if any I/O error occurs attempting to gain
  ///   the lock.
  fn obtain_lock(&self, dir: &Path, lock_name: &str) -> Result<Self::Lock>;
}

pub type DynLockFactory = dyn LockFactory<Lock = LockEnum> + Send + Sync;
pub type CustomLockFactory = Box<DynLockFactory>;

pub enum LockFactoryEnum {
  Single(SingleInstanceLockFactory),
  Simple(SimpleFSLockFactory),
  Native(NativeFSLockFactory),
  Custom(CustomLockFactory),
  NoLock(NoLockFactory),
}

impl LockFactoryEnum {
  pub fn custom<F>(lock_factory: F) -> Self
  where
    F: LockFactory<Lock = LockEnum> + Send + Sync + 'static,
  {
    Self::Custom(Box::new(lock_factory))
  }
}

impl Display for LockFactoryEnum {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Single(inner) => inner.fmt(f),
      Self::Simple(inner) => inner.fmt(f),
      Self::Native(inner) => inner.fmt(f),
      Self::Custom(inner) => inner.fmt(f),
      Self::NoLock(inner) => inner.fmt(f),
    }
  }
}

impl LockFactory for LockFactoryEnum {
  type Lock = LockEnum;

  fn obtain_lock(&self, dir: &Path, lock_name: &str) -> Result<Self::Lock> {
    match self {
      Self::Single(inner) => inner.obtain_lock(dir, lock_name).map(LockEnum::Single),
      Self::Simple(inner) => inner.obtain_lock(dir, lock_name).map(LockEnum::Simple),
      Self::Native(inner) => inner.obtain_lock(dir, lock_name).map(LockEnum::Native),
      Self::Custom(inner) => inner.obtain_lock(dir, lock_name),
      Self::NoLock(inner) => inner.obtain_lock(dir, lock_name).map(LockEnum::NoLock),
    }
  }
}

impl_from_for_enum!(
    LockFactoryEnum,
    SingleInstanceLockFactory => Single,
    SimpleFSLockFactory => Simple,
    NativeFSLockFactory => Native,
    NoLockFactory => NoLock,
);

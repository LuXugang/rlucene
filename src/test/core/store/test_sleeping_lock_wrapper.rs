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
use crate::core::store::directory::DirectoryEnum2;
use crate::core::store::nio_fs_directory::NIOFSDirectory;
use crate::core::store::single_instance_lock_factory::SingleInstanceLockFactory;
use crate::core::store::sleeping_lock_wrapper::SleepingLockWrapper;
use crate::core::store::{FSDirectory, NativeFSLockFactory};
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::store::base_lock_factory_test_case::BaseLockFactoryTestCase;
use rand::RngExt;
use std::path::PathBuf;

/** Simple tests for SleepingLockWrapper */
#[allow(dead_code)] // for quick search
struct TestSleepingLockWrapper;

impl BaseLockFactoryTestCase for TestSleepingLockWrapper {
  type Directory = DirectoryEnum2<
    SleepingLockWrapper<FSDirectory<SingleInstanceLockFactory, NIOFSDirectory>>,
    SleepingLockWrapper<FSDirectory<NativeFSLockFactory, NIOFSDirectory>>,
  >;

  fn get_directory<R>(&self, random: &mut R, path: PathBuf) -> Result<Self::Directory>
  where
    R: rand::Rng + ?Sized,
  {
    // TODO IMPORTANT 应该使用带参数的newFSDirectory
    let lock_wait_timeout = random.random_range(20..=100);
    let poll_interval = random.random_range(2..=10);
    let which = random.random_range(0..3);
    match which {
      0 => Ok(DirectoryEnum2::A(SleepingLockWrapper::with_poll_interval(
        NIOFSDirectory::with_lock_factory(path, SingleInstanceLockFactory::new())?,
        lock_wait_timeout,
        poll_interval,
      )?)),
      1 => Ok(DirectoryEnum2::B(SleepingLockWrapper::with_poll_interval(
        NIOFSDirectory::new(path)?,
        lock_wait_timeout,
        poll_interval,
      )?)),
      _ => Ok(DirectoryEnum2::B(SleepingLockWrapper::with_poll_interval(
        NIOFSDirectory::new(path)?,
        lock_wait_timeout,
        poll_interval,
      )?)),
    }
  }
}

mod base_lock_factory_test_case_tests {
  use super::TestSleepingLockWrapper;
  use crate::core::util::error::lucene_error::Result;
  use crate::test_framework::core::store::base_lock_factory_test_case::BaseLockFactoryTestCase;
  use crate::test_framework::core::util::lucene_test_case::random;

  #[test]
  fn test_basics() -> Result<()> {
    let case = TestSleepingLockWrapper;
    let mut random = random();
    case.test_basics(&mut random)
  }

  #[test]
  fn test_double_close() -> Result<()> {
    let case = TestSleepingLockWrapper;
    let mut random = random();
    case.test_double_close(&mut random)
  }

  #[test]
  fn test_valid_after_acquire() -> Result<()> {
    let case = TestSleepingLockWrapper;
    let mut random = random();
    case.test_valid_after_acquire(&mut random)
  }

  #[test]
  fn test_invalid_after_close() -> Result<()> {
    let case = TestSleepingLockWrapper;
    let mut random = random();
    case.test_invalid_after_close(&mut random)
  }

  #[test]
  fn test_obtain_concurrently() -> Result<()> {
    let case = TestSleepingLockWrapper;
    let mut random = random();
    case.test_obtain_concurrently(&mut random)
  }

  #[test]
  fn test_stress_locks() -> Result<()> {
    let case = TestSleepingLockWrapper;
    let mut random = random();
    case.test_stress_locks(&mut random)
  }
}

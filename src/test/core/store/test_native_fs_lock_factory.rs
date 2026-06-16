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
use crate::core::store::nio_fs_directory::NIOFSDirectory;
use crate::core::store::{FSDirectory, NativeFSLockFactory};
use crate::core::util::error::lucene_error::Result;
use crate::test::core::store::base_lock_factory_test_case::BaseLockFactoryTestCase;
use std::path::PathBuf;

/** Simple tests for NativeFSLockFactory */
#[allow(dead_code)] // for quick search
struct TestNativeFSLockFactory;

impl BaseLockFactoryTestCase for TestNativeFSLockFactory {
  type Directory = FSDirectory<NativeFSLockFactory, NIOFSDirectory>;

  fn get_directory<R>(&self, _random: &mut R, path: PathBuf) -> Result<Self::Directory>
  where
    R: rand::Rng + ?Sized,
  {
    // TODO IMPORTANT 应该使用带参数的newFSDirectory
    NIOFSDirectory::new(path)
  }
}

mod native_fs_lock_factory_tests {
  use super::TestNativeFSLockFactory;
  use crate::core::store::directory::Directory;
  use crate::core::store::lock::Lock;
  use crate::core::util::close::{Closeable, CloseableRef};
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::store::base_lock_factory_test_case::BaseLockFactoryTestCase;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{create_temp_dir, random};
  use std::fs::{self, File};

  /** Verify NativeFSLockFactory works correctly if the lock file exists */
  #[test]
  fn test_lock_file_exists() -> Result<()> {
    let case = TestNativeFSLockFactory;
    let mut random = random();
    let temp_dir = create_temp_dir()?;
    let lock_file = temp_dir.path().join("test.lock");
    File::create(lock_file)?;

    let dir = case.get_directory(&mut random, temp_dir.path().to_path_buf())?;
    let l = dir.obtain_lock("test.lock")?;
    l.close()?;
    Ok(())
  }

  /** release the lock and test ensureValid fails */
  #[test]
  fn test_invalidate_lock() -> Result<()> {
    let case = TestNativeFSLockFactory;
    let mut random = random();
    let temp_dir = create_temp_dir()?;
    let dir = case.get_directory(&mut random, temp_dir.path().to_path_buf())?;
    let lock = dir.obtain_lock("test.lock")?;
    lock.ensure_valid()?;

    lock.release_lock_for_test()?;
    assert!(lock.ensure_valid().is_err());

    lock.close()?;
    let mut dir = dir;
    dir.close()?;
    Ok(())
  }

  /** close the channel and test ensureValid fails */
  #[test]
  fn test_invalidate_channel() -> Result<()> {
    let case = TestNativeFSLockFactory;
    let mut random = random();
    let temp_dir = create_temp_dir()?;
    let dir = case.get_directory(&mut random, temp_dir.path().to_path_buf())?;
    let lock = dir.obtain_lock("test.lock")?;
    lock.ensure_valid()?;

    lock.close()?;
    assert!(lock.ensure_valid().is_err());

    let mut dir = dir;
    dir.close()?;
    Ok(())
  }

  /** delete the lockfile and test ensureValid fails */
  #[test]
  fn test_delete_lock_file() -> Result<()> {
    let case = TestNativeFSLockFactory;
    let mut random = random();
    let temp_dir = create_temp_dir()?;
    let dir = case.get_directory(&mut random, temp_dir.path().to_path_buf())?;
    let lock = dir.obtain_lock("test.lock")?;
    lock.ensure_valid()?;

    dir.delete_file("test.lock")?;

    assert!(lock.ensure_valid().is_err());

    lock.close()?;
    let mut dir = dir;
    dir.close()?;
    Ok(())
  }
  /// This test relies on Unix directory write permissions; Windows readonly directories can still create files.
  #[cfg(unix)]
  #[test]
  fn test_bad_permissions() -> Result<()> {
    let case = TestNativeFSLockFactory;
    let mut random = random();
    // create a directory that will fail while creating test.lock
    let tmp_dir = create_temp_dir()?;
    let index_dir = tmp_dir.path().join("indexDir");
    let dir = case.get_directory(&mut random, index_dir)?;
    let mut permissions = fs::metadata(&dir.directory)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&dir.directory, permissions)?;

    let result = dir.obtain_lock("test.lock");

    assert!(result.is_err());

    let mut dir = dir;
    dir.close()?;
    Ok(())
  }
}

mod base_lock_factory_test_case_tests {
  use super::TestNativeFSLockFactory;
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::store::base_lock_factory_test_case::BaseLockFactoryTestCase;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;

  #[test]
  fn test_basics() -> Result<()> {
    let case = TestNativeFSLockFactory;
    let mut random = random();
    case.test_basics(&mut random)
  }

  #[test]
  fn test_double_close() -> Result<()> {
    let case = TestNativeFSLockFactory;
    let mut random = random();
    case.test_double_close(&mut random)
  }

  #[test]
  fn test_valid_after_acquire() -> Result<()> {
    let case = TestNativeFSLockFactory;
    let mut random = random();
    case.test_valid_after_acquire(&mut random)
  }

  #[test]
  fn test_invalid_after_close() -> Result<()> {
    let case = TestNativeFSLockFactory;
    let mut random = random();
    case.test_invalid_after_close(&mut random)
  }

  #[test]
  fn test_obtain_concurrently() -> Result<()> {
    let case = TestNativeFSLockFactory;
    let mut random = random();
    case.test_obtain_concurrently(&mut random)
  }

  #[test]
  fn test_stress_locks() -> Result<()> {
    let case = TestNativeFSLockFactory;
    let mut random = random();
    case.test_stress_locks(&mut random)
  }
}

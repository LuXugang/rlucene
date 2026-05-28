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
use crate::core::store::{FSDirectory, SimpleFSLockFactory};
use crate::core::util::error::lucene_error::Result;
use crate::test::core::store::base_lock_factory_test_case::BaseLockFactoryTestCase;
use std::path::PathBuf;

/// Simple tests for SimpleFSLockFactory
#[allow(dead_code)] // for quick search
struct TestSimpleFSLockFactory;

impl BaseLockFactoryTestCase for TestSimpleFSLockFactory {
  type Directory = FSDirectory<SimpleFSLockFactory, NIOFSDirectory>;

  fn get_directory(&self, path: PathBuf) -> Result<Self::Directory> {
    // TODO IMPORTANT 应该使用带参数的newFSDirectory
    FSDirectory::with_lock_factory(path, SimpleFSLockFactory::new(), NIOFSDirectory::new())
  }
}

mod simple_fs_lock_factory_tests {
  use crate::core::store::directory::Directory;
  use crate::core::store::lock::Lock;
  use crate::core::util::close::{Closeable, CloseableRef};
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::store::base_lock_factory_test_case::BaseLockFactoryTestCase;
  use crate::test::core::store::test_simple_fs_lock_factory::run_case;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::create_temp_dir;

  /// delete the lockfile and test ensureValid fails
  #[test]
  fn test_delete_lock_file() -> Result<()> {
    run_case(|case, _random| {
      let temp_dir = create_temp_dir()?;
      let dir = case.get_directory(temp_dir.path().to_path_buf())?;
      let lock = dir.obtain_lock("test.lock")?;
      lock.ensure_valid()?;

      dir.delete_file("test.lock")?;

      assert!(lock.ensure_valid().is_err());
      let _ = lock.close();

      let mut dir = dir;
      dir.close()?;
      Ok(())
    })
  }
}

mod base_lock_factory_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::store::base_lock_factory_test_case::BaseLockFactoryTestCase;
  use crate::test::core::store::test_simple_fs_lock_factory::run_case;

  #[test]
  fn test_basics() -> Result<()> {
    run_case(|case, _random| case.test_basics())
  }

  #[test]
  fn test_double_close() -> Result<()> {
    run_case(|case, _random| case.test_double_close())
  }

  #[test]
  fn test_valid_after_acquire() -> Result<()> {
    run_case(|case, _random| case.test_valid_after_acquire())
  }

  #[test]
  fn test_invalid_after_close() -> Result<()> {
    run_case(|case, _random| case.test_invalid_after_close())
  }

  #[test]
  fn test_obtain_concurrently() -> Result<()> {
    run_case(|case, random| case.test_obtain_concurrently(random))
  }

  #[test]
  fn test_stress_locks() -> Result<()> {
    run_case(|case, random| case.test_stress_locks(random))
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestSimpleFSLockFactory, &mut rand::prelude::StdRng) -> Result<()>,
{
  let mut random = crate::test::core::util::lucene_test_case::lucene_test_case_util::random();
  let case = TestSimpleFSLockFactory;
  f(&case, &mut random)
}

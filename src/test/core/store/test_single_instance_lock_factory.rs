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
use crate::core::store::FSDirectory;
use crate::core::store::nio_fs_directory::NIOFSDirectory;
use crate::core::store::single_instance_lock_factory::SingleInstanceLockFactory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::store::base_lock_factory_test_case::BaseLockFactoryTestCase;
use std::path::PathBuf;

/// Simple tests for SingleInstanceLockFactory
#[allow(dead_code)] // for quick search
struct TestSingleInstanceLockFactory;

impl BaseLockFactoryTestCase for TestSingleInstanceLockFactory {
  type Directory = FSDirectory<SingleInstanceLockFactory, NIOFSDirectory>;

  fn get_directory(&self, path: PathBuf) -> Result<Self::Directory> {
    // TODO IMPORTANT 应该使用带参数的newFSDirectory
    FSDirectory::with_lock_factory(
      path,
      SingleInstanceLockFactory::new(),
      NIOFSDirectory::new(),
    )
  }
}

mod single_instance_lock_factory_tests {
  use crate::core::util::error::lucene_error::Result;

  // Verify: basic locking on single instance lock factory (can't create two IndexWriters)
  #[test]
  fn test_default_lock_factory() -> Result<()> {
    // TODO IMPORTANT ByteBuffersDirectory未实现
    Ok(())
  }
}

mod base_lock_factory_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::store::base_lock_factory_test_case::BaseLockFactoryTestCase;
  use crate::test::core::store::test_single_instance_lock_factory::run_case;

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

  // TODO IMPORTANT
  fn test_stress_locks() -> Result<()> {
    run_case(|case, random| case.test_stress_locks(random))
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestSingleInstanceLockFactory, &mut rand::prelude::StdRng) -> Result<()>,
{
  let mut random = crate::test::core::util::lucene_test_case::lucene_test_case_util::random();
  let case = TestSingleInstanceLockFactory;
  f(&case, &mut random)
}

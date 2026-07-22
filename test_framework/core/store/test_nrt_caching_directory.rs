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

use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::store::nrt_caching_directory::{
  NRTCachingDirectory, NRTCachingDirectoryBase, NRTCachingDirectoryDefaults,
};

#[allow(dead_code)] // for quick search
struct TestNRTCachingDirectory;

pub(crate) struct AssertCacheWriteNRTCachingDirectory {
  expected: bool,
}

impl AssertCacheWriteNRTCachingDirectory {
  pub(crate) fn new(expected: bool) -> Self {
    Self { expected }
  }
}

impl NRTCachingDirectoryBase for AssertCacheWriteNRTCachingDirectory {
  fn do_cache_write<D>(
    &self,
    directory: &NRTCachingDirectory<D>,
    name: &str,
    context: &IOContext,
  ) -> bool
  where
    D: Directory,
  {
    let cache = NRTCachingDirectoryDefaults::do_cache_write(directory, name, context);
    assert_eq!(self.expected, cache);
    cache
  }
}

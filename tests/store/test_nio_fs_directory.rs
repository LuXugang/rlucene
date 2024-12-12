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
use std::path::PathBuf;
use tempfile::TempDir;
use rlucene::store::directory::Directory;
use rlucene::store::fs_directory::FSDirectory;
use rlucene::store::nio_fs_directory::NIOFSDirectory;
use crate::common::my_random;
use crate::store::base_directory_test_case::BaseDirectoryTestCase;
use crate::util::test_error::TestError;

#[allow(dead_code)] // for quick search
struct TestNIOFSDirectory;

impl BaseDirectoryTestCase for TestNIOFSDirectory{
    fn get_directory(&self, path: PathBuf) ->Result<impl Directory,TestError> {
        let sub_directory = NIOFSDirectory::new();
        Ok(FSDirectory::new(path, sub_directory)?)
    }
}

#[test]
fn test_copy_from() -> Result<(), TestError> {
    let mut random = my_random("test_copy_from".to_string());
    let test = TestNIOFSDirectory;
    test.test_copy_from(&mut random)
}


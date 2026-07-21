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
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
use crate::test_framework::core::util::lucene_test_case::create_temp_dir;
use std::fs::{File, create_dir_all};
use std::io::Write;

#[allow(dead_code)] // for quick search
struct TestIOUtils;

#[test]
fn test_delete_file_ignoring_exceptions() -> Result<()> {
  let dir = create_temp_dir()?;
  let file1 = dir.path().join("file1");
  File::create(&file1)?;
  IOUtils::delete_paths_ignoring_exceptions([&file1]);
  assert!(!file1.exists());
  // actually deletes
  Ok(())
}

#[test]
fn test_dont_delete_file_ignoring_exceptions() -> Result<()> {
  let dir = create_temp_dir()?;
  let file1 = dir.path().join("file1");
  IOUtils::delete_paths_ignoring_exceptions([&file1]);
  // no exception
  Ok(())
}

#[test]
fn test_delete_two_files_ignoring_exceptions() -> Result<()> {
  let dir = create_temp_dir()?;
  let file1 = dir.path().join("file1");
  let file2 = dir.path().join("file2");
  // only create file2
  File::create(&file2)?;
  IOUtils::delete_paths_ignoring_exceptions([&file1, &file2]);
  assert!(!file2.exists());
  // no exception
  // actually deletes file2
  Ok(())
}

#[test]
fn test_delete_file_if_exists() -> Result<()> {
  let dir = create_temp_dir()?;
  let file1 = dir.path().join("file1");
  File::create(&file1)?;
  IOUtils::delete_files_if_exist([&file1])?;
  assert!(!file1.exists());
  // actually deletes
  Ok(())
}

#[test]
fn test_dont_delete_doesnt_exist() -> Result<()> {
  let dir = create_temp_dir()?;
  let file1 = dir.path().join("file1");
  IOUtils::delete_files_if_exist([&file1])?;
  // no exception
  Ok(())
}

#[test]
fn test_delete_two_files_if_exist() -> Result<()> {
  let dir = create_temp_dir()?;
  let file1 = dir.path().join("file1");
  let file2 = dir.path().join("file2");
  // only create file2
  File::create(&file2)?;
  IOUtils::delete_files_if_exist([&file1, &file2])?;
  assert!(!file2.exists());
  // no exception
  // actually deletes file2
  Ok(())
}

#[test]
fn test_fsync_directory() -> Result<()> {
  let dir = create_temp_dir()?;
  let dev_dir = dir.path().join("dev");
  create_dir_all(&dev_dir)?;
  IOUtils::fsync(&dev_dir, true)?;
  // no exception
  Ok(())
}

#[test]
fn test_fsync_access_denied_opening_directory() -> Result<()> {
  // TODO: FilterFileSystemProvider and wrapped Path support have not been migrated.
  Ok(())
}

#[test]
fn test_fsync_non_existent_directory() -> Result<()> {
  let dir = create_temp_dir()?;
  let non_existent_dir = dir.path().join("non-existent");
  assert!(matches!(
    IOUtils::fsync(&non_existent_dir, true),
    Err(LuceneError::NoSuchFile(_))
  ));
  Ok(())
}

#[test]
fn test_fsync_file() -> Result<()> {
  let dir = create_temp_dir()?;
  let dev_dir = dir.path().join("dev");
  create_dir_all(&dev_dir)?;
  let file_path = dev_dir.join("somefile");
  let mut output = File::create(&file_path)?;
  output.write_all(b"0\n")?;
  output.flush()?;
  drop(output);
  IOUtils::fsync(&file_path, false)?;
  // no exception
  Ok(())
}

#[test]
fn test_apply_to_all() -> Result<()> {
  let mut closed = Vec::new();
  let error = IOUtils::apply_to_all(&[1, 2], |value| {
    closed.push(*value);
    Err(LuceneError::illegal_state(value.to_string()))
  })
  .unwrap_err();
  assert_eq!("1", error.to_string());
  let suppressed = error
    .get_suppressed()?
    .expect("the second failure must be suppressed");
  assert_eq!("2", suppressed.to_string());
  assert_eq!(vec![1, 2], closed);
  Ok(())
}

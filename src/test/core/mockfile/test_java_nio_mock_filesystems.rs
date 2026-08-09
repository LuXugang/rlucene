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
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)]
struct TestDisableFsyncFS;
#[allow(dead_code)]
struct TestExtrasFS;
#[allow(dead_code)]
struct TestHandleLimitFS;
#[allow(dead_code)]
struct TestHandleTrackingFS;
#[allow(dead_code)]
struct TestLeakFS;
#[allow(dead_code)]
struct TestShuffleFS;
#[allow(dead_code)]
struct TestVerboseFS;
#[allow(dead_code)]
struct TestVirusCheckingFS;
#[allow(dead_code)]
struct TestWindowsFS;

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_fsync_works() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_extra_file() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_extra_directory() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_no_extras() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_too_many_open_files() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_on_close_throws_exception() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_on_open_throws_exception() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_leak_input_stream() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_leak_output_stream() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_leak_file_channel() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_leak_async_file_channel() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_leak_byte_channel() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_shuffle_works() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_actually_shuffles() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_consistent_order() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_file_name_only() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_create_directory() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_delete() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_delete_if_exists() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_copy() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_move() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_new_output_stream() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_file_channel() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_async_file_channel() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_byte_channel() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_verbose_fs_no_such_file_exception() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_delete_sometimes_fails() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_delete_open_file() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_delete_if_exists_open_file() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_rename_open_file() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_open_delete_concurrently() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust does not use Java NIO FileSystemProvider wrappers"]
fn test_file_name() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

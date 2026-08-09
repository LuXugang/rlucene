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

#[allow(dead_code)] // for quick search
struct TestFilterIterator;

#[test]
#[ignore = "Java-only: Rust Iterator::filter has no Java hasNext/next/remove contract"]
fn test_empty() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust Iterator::filter has no Java hasNext/next/remove contract"]
fn test_a1() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust Iterator::filter has no Java hasNext/next/remove contract"]
fn test_a2() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust Iterator::filter has no Java hasNext/next/remove contract"]
fn test_b1() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust Iterator::filter has no Java hasNext/next/remove contract"]
fn test_b2() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust Iterator::filter has no Java hasNext/next/remove contract"]
fn test_all1() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust Iterator::filter has no Java hasNext/next/remove contract"]
fn test_all2() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust Iterator::filter has no Java hasNext/next/remove contract"]
fn test_unmodifiable() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

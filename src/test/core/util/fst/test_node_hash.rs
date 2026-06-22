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

use crate::test::core::util::lucene_test_case::{at_least, random};
use rand::RngExt;
use std::sync::Arc;

use crate::core::util::error::lucene_error::Result;

use crate::core::util::fst_impl::node_hash::PagedGrowableHash;

#[allow(dead_code)] // for quick search
struct TestNodeHash;
#[test]
fn test_copy_fallback_node_bytes() -> Result<()> {
  let mut random = random();
  // Create primary and fallback hash tables
  let mut primary_hash_table: PagedGrowableHash<Arc<i64>> = PagedGrowableHash::new()?;
  let mut fallback_hash_table = PagedGrowableHash::new()?;

  let node_length = at_least(&mut random, 500);
  let fallback_hash_slot = 1;
  let fallback_bytes: Vec<u8> = (0..node_length).map(|_| random.random()).collect();

  fallback_hash_table.copy_node_bytes(fallback_hash_slot, &fallback_bytes, node_length)?;

  // Check that fallback bytes stored correctly
  let stored_bytes = fallback_hash_table.get_bytes(fallback_hash_slot, node_length)?;
  for i in 0..node_length as usize {
    assert_eq!(fallback_bytes[i], stored_bytes[i], "byte @ index={}", i);
  }

  let primary_hash_slot = 2;
  primary_hash_table.copy_fallback_node_bytes(
    primary_hash_slot,
    &mut fallback_hash_table,
    fallback_hash_slot,
    node_length,
  )?;

  // Check that primary copied bytes match original
  let copied_bytes = primary_hash_table.get_bytes(primary_hash_slot, node_length)?;
  for i in 0..node_length as usize {
    assert_eq!(fallback_bytes[i], copied_bytes[i], "byte @ index={}", i);
  }
  Ok(())
}

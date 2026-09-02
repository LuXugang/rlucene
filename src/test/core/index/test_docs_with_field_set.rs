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
use crate::test_framework::core::util::lucene_test_case::random;
use rand::RngExt;

use crate::core::index::docs_with_field_set::DocsWithFieldSet;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestDocsWithFieldSet {}
#[test]
fn test_dense() -> Result<()> {
  let mut set = DocsWithFieldSet::new();
  set.finish();
  let mut it = set.iterator()?;
  assert_eq!(NO_MORE_DOCS, it.next_doc()?);

  // Each Java checkpoint needs its own set because Rust iterators require finish().
  let mut set = DocsWithFieldSet::new();
  set.add(0)?;
  set.finish();
  let mut it = set.iterator()?;
  assert_eq!(0, it.next_doc()?);
  assert_eq!(NO_MORE_DOCS, it.next_doc()?);

  let mut set = DocsWithFieldSet::new();
  set.add(0)?;
  let ram_bytes_used = set.ram_bytes_used()?;
  for i in 1..1000 {
    set.add(i)?;
  }
  assert_eq!(ram_bytes_used, set.ram_bytes_used()?);
  set.finish();
  let mut it = set.iterator()?;
  for i in 0..1000 {
    assert_eq!(i, it.next_doc()?);
  }
  assert_eq!(NO_MORE_DOCS, it.next_doc()?);
  Ok(())
}

#[test]
fn test_sparse() -> Result<()> {
  let mut random = random();
  let mut set = DocsWithFieldSet::new();
  let doc = random.random_range(0..10000);
  set.add(doc)?;
  set.finish();
  let mut it = set.iterator()?;
  assert_eq!(doc, it.next_doc()?);
  assert_eq!(NO_MORE_DOCS, it.next_doc()?);

  let doc2 = doc + TestUtil::next_int(&mut random, 1, 100);
  // Rebuild the prefix for the second Java checkpoint with a newly finished set.
  let mut set = DocsWithFieldSet::new();
  set.add(doc)?;
  set.add(doc2)?;
  set.finish();
  let mut it = set.iterator()?;
  assert_eq!(doc, it.next_doc()?);
  assert_eq!(doc2, it.next_doc()?);
  assert_eq!(NO_MORE_DOCS, it.next_doc()?);
  Ok(())
}

#[test]
fn test_dense_then_sparse() -> Result<()> {
  let mut random = random();
  let dense_count = random.random_range(0..10000);
  let next_doc = dense_count + random.random_range(0..10000);
  let mut set = DocsWithFieldSet::new();
  for i in 0..dense_count {
    set.add(i)?;
  }
  set.add(next_doc)?;
  set.finish();
  let mut it = set.iterator()?;
  for i in 0..dense_count {
    assert_eq!(i, it.next_doc()?);
  }
  assert_eq!(next_doc, it.next_doc()?);
  assert_eq!(NO_MORE_DOCS, it.next_doc()?);
  Ok(())
}

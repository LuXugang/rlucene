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
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestDocsWithFieldSet {}
#[test]
fn test_dense() -> Result<()> {
  let mut random = random();
  let mut set = DocsWithFieldSet::new();
  let mut it;

  match random.random_range(0..3) {
    0 => {
      set.finish();
      let mut it = set.iterator()?;
      assert_eq!(it.next_doc()?, NO_MORE_DOCS);
      Ok(())
    },
    1 => {
      set.add(0)?;
      set.finish();
      it = set.iterator()?;
      assert_eq!(0, it.next_doc()?);
      assert_eq!(it.next_doc()?, NO_MORE_DOCS);
      Ok(())
    },
    _ => {
      set.add(0)?;

      // TODO: 可以在这里获取内存使用情况
      // let ram_bytes_used = set.ram_bytes_used();

      for i in 1..1000 {
        set.add(i)?;
      }
      set.finish();

      // TODO: 之后可以加断言
      // assert_eq!(ram_bytes_used, set.ram_bytes_used());

      it = set.iterator()?;
      for i in 0..1000 {
        assert_eq!(i, it.next_doc()?);
      }
      assert_eq!(NO_MORE_DOCS, it.next_doc()?);
      Ok(())
    },
  }
}

#[test]
fn test_sparse() -> Result<()> {
  let mut random = random();
  let mut set = DocsWithFieldSet::new();
  let doc = random.random_range(0..10000);
  let _ = set.add(doc);
  if random.random_bool(0.5) {
    set.finish();
    {
      let mut it = set.iterator()?;
      assert_eq!(doc, it.next_doc()?);
      assert_eq!(it.next_doc()?, NO_MORE_DOCS);
    }
  } else {
    let doc2 = doc + TestUtil::next_int(&mut random, 1, 100);
    set.add(doc2)?;
    set.finish();
    let mut it = set.iterator()?;
    assert_eq!(doc, it.next_doc()?);
    assert_eq!(doc2, it.next_doc()?);
    assert_eq!(it.next_doc()?, NO_MORE_DOCS);
  }
  Ok(())
}

#[test]
fn test_dense_then_sparse() -> Result<()> {
  let mut random = random();
  let dense_count = random.random_range(1..10000);
  let next_doc = dense_count + random.random_range(1..10000);
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

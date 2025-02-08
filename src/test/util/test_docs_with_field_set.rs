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
use crate::index::docs_with_field_set::DocsWithFieldSet;
use crate::search::doc_id_set::DocIdSet;
use crate::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::test::util::lucene_test_case::random;
use crate::test::util::test_error::TestError;
use crate::test::util::test_util::TestUtil;
use rand::Rng;

#[allow(dead_code)] // for quick search
struct TestDocsWithFieldSet {}
#[test]
fn test_dense() -> Result<(), TestError> {
    let mut set = DocsWithFieldSet::new();
    let mut it = set.iterator().unwrap();
    assert_eq!(it.next_doc()?, NO_MORE_DOCS);

    let _ = set.add(0);
    it = set.iterator().unwrap();
    assert_eq!(0, it.next_doc()?);
    assert_eq!(it.next_doc()?, NO_MORE_DOCS);

    //TODO
    // let ram_bytes_used = set.ram_bytes_used();
    for i in 0..1000 {
        let _ = set.add(i);
    }
    //TODO:
    // assert_eq!(ram_bytes_used, set.ram_bytes_used());
    it = set.iterator().unwrap();
    for i in 0..1000 {
        assert_eq!(i, it.next_doc()?);
    }
    assert_eq!(NO_MORE_DOCS, it.next_doc()?);
    Ok(())
}

#[test]
fn test_sparse() -> Result<(), TestError> {
    let mut random = random();
    let mut set = DocsWithFieldSet::new();
    let doc = random.gen_range(0..10000);
    let _ = set.add(doc);
    let mut it = set.iterator().unwrap();
    assert_eq!(doc, it.next_doc()?);
    assert_eq!(it.next_doc()?, NO_MORE_DOCS);
    let doc2 = doc + TestUtil::next_int(&mut random, 1, 100);
    let _ = set.add(doc2);
    it = set.iterator().unwrap();
    assert_eq!(doc, it.next_doc()?);
    assert_eq!(doc2, it.next_doc()?);
    assert_eq!(it.next_doc()?, NO_MORE_DOCS);
    Ok(())
}

#[test]
fn test_dense_then_sparse() -> Result<(), TestError> {
    let mut random = random();
    let dense_count = random.gen_range(1..10000);
    let next_doc = dense_count + random.gen_range(1..10000);
    let mut set = DocsWithFieldSet::new();
    for i in 0..dense_count {
        let _ = set.add(i);
    }
    let _ = set.add(next_doc);
    let mut it = set.iterator().unwrap();
    for i in 0..dense_count {
        assert_eq!(i, it.next_doc()?);
    }
    assert_eq!(next_doc, it.next_doc()?);
    assert_eq!(NO_MORE_DOCS, it.next_doc()?);
    Ok(())
}

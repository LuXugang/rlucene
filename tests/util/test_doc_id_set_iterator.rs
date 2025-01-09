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
use crate::util::test_error::TestError;
use rlucene::search::doc_id_set_iterator::{DocIdSetIterator, Range, NO_MORE_DOCS};

#[allow(dead_code)] // for quick search
struct TestDocIdSetIterator {}
#[test]
fn test_range_basic() -> Result<(), TestError> {
    let result = Range::new(5, 8);
    assert!(result.is_ok());
    let mut disi = result?;
    assert_eq!(-1, disi.doc_id());
    assert_eq!(5, disi.next_doc()?);
    assert_eq!(6, disi.next_doc()?);
    assert_eq!(7, disi.next_doc()?);
    assert_eq!(NO_MORE_DOCS, disi.next_doc()?);
    Ok(())
}

#[test]
fn test_invalid_range() {
    let disi_result = Range::new(5, 4);
    assert!(disi_result.is_err());
}

#[test]
fn test_invalid_min() {
    let disi_result = Range::new(-1, 4);
    assert!(disi_result.is_err());
}

#[test]
fn test_empty() {
    let disi_result = Range::new(7, 7);
    assert!(disi_result.is_err());
}

#[test]
fn test_advance() -> Result<(), TestError> {
    let disi_result = Range::new(5, 20);
    assert!(disi_result.is_ok());
    let mut disi = disi_result?;
    assert_eq!(-1, disi.doc_id());
    assert_eq!(5, disi.next_doc()?);
    assert_eq!(17, disi.advance(17)?);
    assert_eq!(18, disi.next_doc()?);
    assert_eq!(19, disi.next_doc()?);
    assert_eq!(NO_MORE_DOCS, disi.next_doc()?);
    Ok(())
}

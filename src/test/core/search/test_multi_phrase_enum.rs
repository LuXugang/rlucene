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
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::index::directory_reader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::postings_enum::{POSITIONS, PostingsEnum};
use crate::core::index::term::Term;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::multi_phrase_query::UnionPostingsEnum;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  get_only_leaf_reader, new_directory_shared, new_index_writer_config, new_log_merge_policy,
  new_text_field, random,
};
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
struct TestMultiPhraseEnum;

/// Tests union on one document
#[test]
fn test_one_document() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let mut field_to_type = HashMap::new();

  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "foo bar",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(doc)?;

  let ir = directory_reader::open_from_writer(&writer)?;
  writer.close()?;

  let r = get_only_leaf_reader(ir)?;

  let p1 = r
    .postings_with_flag(&Term::from_text("field", "foo"), POSITIONS as i32)?
    .unwrap();
  let p2 = r
    .postings_with_flag(&Term::from_text("field", "bar"), POSITIONS as i32)?
    .unwrap();
  let mut union = UnionPostingsEnum::new(vec![p1, p2]);

  assert_eq!(-1, union.doc_id());

  assert_eq!(0, union.next_doc()?);
  assert_eq!(2, union.freq()?);
  assert_eq!(0, union.next_position()?);
  assert_eq!(1, union.next_position()?);

  assert_eq!(NO_MORE_DOCS, union.next_doc()?);

  Ok(())
}

/// Tests union on a few documents
#[test]
fn test_some_documents() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let mut field_to_type = HashMap::new();

  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "foo",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(doc)?;

  writer.add_document(Document::new())?;

  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "foo bar",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(doc)?;

  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "bar",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(doc)?;

  writer.force_merge(1)?;
  let ir = directory_reader::open_from_writer(&writer)?;
  writer.close()?;

  let r = get_only_leaf_reader(ir)?;

  let p1 = r
    .postings_with_flag(&Term::from_text("field", "foo"), POSITIONS as i32)?
    .unwrap();
  let p2 = r
    .postings_with_flag(&Term::from_text("field", "bar"), POSITIONS as i32)?
    .unwrap();
  let mut union = UnionPostingsEnum::new(vec![p1, p2]);

  assert_eq!(-1, union.doc_id());

  assert_eq!(0, union.next_doc()?);
  assert_eq!(1, union.freq()?);
  assert_eq!(0, union.next_position()?);

  assert_eq!(2, union.next_doc()?);
  assert_eq!(2, union.freq()?);
  assert_eq!(0, union.next_position()?);
  assert_eq!(1, union.next_position()?);

  assert_eq!(3, union.next_doc()?);
  assert_eq!(1, union.freq()?);
  assert_eq!(0, union.next_position()?);

  assert_eq!(NO_MORE_DOCS, union.next_doc()?);

  Ok(())
}

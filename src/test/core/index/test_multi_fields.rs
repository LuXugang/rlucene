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
use crate::core::document::field::Store::No;
use crate::core::document::field_type::FieldType;
use crate::core::index::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_bits::get_live_docs;
use crate::core::index::multi_terms::get_term_postings_enum_with_flag;
use crate::core::index::postings_enum::{FREQS, NONE};
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::test_index_writer::KeepFullyDeletedSegmentsMergePolicy;
use crate::test::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, new_string_field, random,
  random_multiplier,
};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
use std::collections::{HashMap, HashSet};

#[allow(dead_code)] // for quick search
struct TestMultiFields;

#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 2);
  for _ in 0..num {
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
    iwc.set_merge_policy(KeepFullyDeletedSegmentsMergePolicy::default());
    let w = IndexWriter::new(dir, iwc)?;

    let mut docs: HashMap<BytesRef<Vec<u8>>, Vec<i32>> = HashMap::new();
    let mut deleted: HashSet<i32> = HashSet::new();
    let mut terms: Vec<BytesRef<Vec<u8>>> = Vec::new();

    let num_docs = TestUtil::next_int(&mut random, 1, 100 * random_multiplier());
    let only_unique_terms = random.random_bool(0.5);
    let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

    for i in 0..num_docs {
      let (term, value) = if !only_unique_terms && random.random_bool(0.5) && !terms.is_empty() {
        let term = terms[random.random_range(0..terms.len())].clone();
        let value = term.utf8_to_string()?;
        docs.get_mut(&term).unwrap().push(i);
        (term, value)
      } else {
        let value = TestUtil::random_unicode_string_with_len(&mut random, 10);
        let term = BytesRef::from_string(&value);
        docs.entry(term.clone()).or_default().push(i);
        terms.push(term.clone());
        (term, value)
      };

      let mut doc = Document::new();
      doc.add(new_string_field(
        &mut random,
        "field",
        value,
        No,
        &mut field_to_type,
      )?);
      doc.add(new_string_field(
        &mut random,
        "id",
        i.to_string(),
        No,
        &mut field_to_type,
      )?);
      w.add_document(doc)?;

      if random.random_range(0..4) == 1 {
        w.commit()?;
      }
      if i > 0 && random.random_range(0..20) == 1 {
        let del_id = random.random_range(0..i);
        deleted.insert(del_id);
        w.delete_documents_with_terms(vec![Term::from_text("id", del_id.to_string())])?;
      }

      debug_assert!(docs.contains_key(&term));
    }

    let reader = directory_reader::open_from_writer(&w)?;
    w.close()?;

    if let Some(live_docs) = get_live_docs(&reader)? {
      for del_doc in &deleted {
        assert!(!live_docs.get(*del_doc as usize)?);
      }
    }

    for _ in 0..100 {
      let term = &terms[random.random_range(0..terms.len())];
      let mut postings_enum =
        TestUtil::docs_with_reader(&mut random, &reader, "field", term, None, NONE as i32)?
          .unwrap();

      for doc_id in docs.get(term).unwrap() {
        assert_eq!(*doc_id, postings_enum.next_doc()?);
      }
      assert_eq!(NO_MORE_DOCS, postings_enum.next_doc()?);
    }

    reader.close()?;
  }
  Ok(())
}

#[test]
fn test_separate_enums() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  let iw = IndexWriter::new(dir.clone(), iwc)?;

  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "f",
    "j",
    No,
    &mut field_to_type,
  )?);

  iw.add_document(doc.clone())?;
  iw.commit()?;
  iw.add_document(doc)?;

  let reader = directory_reader::open_from_writer(&iw)?;
  iw.close()?;

  let mut d1 = TestUtil::docs_with_reader(
    &mut random,
    &reader,
    "f",
    &BytesRef::from_string("j"),
    None,
    NONE as i32,
  )?
  .unwrap();

  let mut d2 = TestUtil::docs_with_reader(
    &mut random,
    &reader,
    "f",
    &BytesRef::from_string("j"),
    None,
    NONE as i32,
  )?
  .unwrap();

  assert_eq!(0, d1.next_doc()?);
  assert_eq!(0, d2.next_doc()?);

  Ok(())
}

#[test]
fn test_term_docs_enum() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  let iw = IndexWriter::new(dir.clone(), iwc)?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "f",
    "j",
    No,
    &mut field_to_type,
  )?);
  iw.add_document(doc.clone())?;
  iw.commit()?;
  iw.add_document(doc)?;

  let reader = directory_reader::open_from_writer(&iw)?;
  iw.close()?;

  let mut de =
    get_term_postings_enum_with_flag(&reader, "f", &BytesRef::from_string("j"), FREQS as i32)?
      .unwrap();

  assert_eq!(0, de.next_doc()?);
  assert_eq!(1, de.next_doc()?);
  assert_eq!(NO_MORE_DOCS, de.next_doc()?);
  Ok(())
}

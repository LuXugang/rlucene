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
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::multi_terms::get_terms;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::{
  get_only_leaf_reader, new_directory_shared, new_index_writer_config_with_analyzer,
  new_text_field, random,
};
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestSegmentTermEnum;
#[test]
fn test_term_enum() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();
  {
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let mut writer = IndexWriter::new(dir.clone(), iwc)?;

    // ADD 100 documents with term : aaa
    // ADD 100 documents with terms: aaa bbb
    // => term "aaa" docFreq = 200, term "bbb" docFreq = 100
    for _ in 0..100 {
      add_doc(&mut random, &mut writer, "aaa", &mut field_types)?;
      add_doc(&mut random, &mut writer, "aaa bbb", &mut field_types)?;
    }

    writer.close()?;
  }

  // verify document frequency of terms in a multi-segment index
  verify_doc_freq(dir.clone())?;

  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), iwc)?;
  writer.force_merge(1)?;
  writer.close()?;
  verify_doc_freq(dir.clone())?;
  Ok(())
}
#[test]
fn test_prev_term_at_end() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let mut writer = IndexWriter::new(dir.clone(), iwc)?;
  add_doc(&mut random, &mut writer, "aaa bbb", &mut field_types)?;
  writer.close()?;

  let reader = get_only_leaf_reader(directory_reader::open(dir.clone())?)?;
  let terms = reader.terms("content")?.expect("terms should exist");
  let mut terms_enum = terms.iterator()?;

  assert!(terms_enum.next()?.is_some());
  assert_eq!(terms_enum.term()?.utf8_to_string()?, "aaa");

  assert!(terms_enum.next()?.is_some());

  let ord_b = match terms_enum.ord() {
    Ok(ord) => ord,
    Err(_) => {
      reader.close()?;
      return Ok(());
    },
  };

  assert_eq!(terms_enum.term()?.utf8_to_string()?, "bbb");
  assert!(terms_enum.next()?.is_none());

  terms_enum.seek_exact_with_ord(ord_b)?;
  assert_eq!(terms_enum.term()?.utf8_to_string()?, "bbb");

  reader.close()?;
  Ok(())
}
fn verify_doc_freq(dir: Arc<DirEnum>) -> Result<()> {
  let reader = directory_reader::open(dir)?;

  let terms = get_terms(&reader, "content")?.expect("terms should exist");
  let mut term_enum = terms.iterator()?;

  assert!(term_enum.next()?.is_some());
  let term = term_enum.term()?;
  assert_eq!(term.utf8_to_string()?, "aaa");
  assert_eq!(term_enum.doc_freq()?, 200);

  assert!(term_enum.next()?.is_some());
  let term = term_enum.term()?;
  assert_eq!(term.utf8_to_string()?, "bbb");
  assert_eq!(term_enum.doc_freq()?, 100);

  term_enum.seek_ceil(&BytesRef::from_string("aaa"))?;
  let term = term_enum.term()?;
  assert_eq!(term.utf8_to_string()?, "aaa");
  assert_eq!(term_enum.doc_freq()?, 200);

  assert!(term_enum.next()?.is_some());
  let term = term_enum.term()?;
  assert_eq!(term.utf8_to_string()?, "bbb");
  assert_eq!(term_enum.doc_freq()?, 100);

  reader.close()?;
  Ok(())
}

fn add_doc<D, R>(
  random: &mut R,
  writer: &mut IndexWriter<D>,
  value: &str,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory,
  R: Rng + ?Sized,
{
  let mut doc = Document::new();
  doc.add(new_text_field(random, "content", value, No, field_types)?);
  writer.add_document(doc)?;
  Ok(())
}

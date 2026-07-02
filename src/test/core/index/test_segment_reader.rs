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
use crate::core::index::BytesRef;
use crate::test_framework::core::util::lucene_test_case::{new_directory_shared, random};

use crate::core::index::field_infos::get_indexed_fields;
use crate::core::index::fields::Fields;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::multi_doc_values::MultiDocValues;
use crate::core::index::multi_reader::MultiReader;
use crate::core::index::multi_terms::{get_term_postings_enum, get_terms};
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::index::segment_reader::SegmentReader;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term_vectors::TermVectors;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::store::IOContext;
use crate::core::store::directory::DirEnum;
use crate::core::util::LATEST;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::doc_helper::NameValue::{Str, String};
use crate::test_framework::core::index::doc_helper::{
  DATA, DocHelper, FIELD_2_TEXT, FIELDS, NAME_VALUES, NO_NORMS_KEY, NO_NORMS_TEXT,
  TEXT_FIELD_1_KEY, TEXT_FIELD_2_KEY,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;
use std::collections::HashSet;
use std::sync::Arc;

pub(crate) struct TestSegmentReader;
impl TestSegmentReader {
  pub(crate) fn check_norms<LR>(reader: LR) -> Result<()>
  where
    LR: LeafReader + Clone,
  {
    let multi_readers = MultiReader::with_leaf_reader(vec![reader.clone()])?;
    for f in FIELDS.iter() {
      if *f.field_type().index_options() != IndexOptions::None {
        let field_name = f.name();
        let norms_opt = reader.get_norm_values(field_name)?;
        assert_eq!(norms_opt.is_some(), !f.field_type().omit_norms());
        assert_eq!(norms_opt.is_some(), !DATA.no_norms.contains_key(field_name));
        if norms_opt.is_none() {
          // test for norms of None
          let norms2 = MultiDocValues::get_norm_values(&multi_readers, field_name)?;
          assert!(norms2.is_none());
        }
      }
    }
    Ok(())
  }
}

fn set_up<R>(random: &mut R) -> Result<(Arc<DirEnum>, Document, SegmentReader<DirEnum>)>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let mut documnet = Document::new();
  DocHelper::setup_doc(&mut documnet);
  let info = DocHelper::write_doc(random, dir.clone(), documnet.clone())?;
  let reader = SegmentReader::new(&info, LATEST.major, &IOContext::default_io_context()?)?;
  Ok((dir, documnet, reader))
}
#[test]
fn test() -> Result<()> {
  let mut random = random();
  let (_dir, document, _reader) = set_up(&mut random)?;
  assert!(!NAME_VALUES.is_empty());
  assert_eq!(DocHelper::num_fields(&document), DATA.all.len());
  Ok(())
}
#[test]
fn test_document() -> Result<()> {
  let mut random = random();
  let (_dir, test_doc, reader) = set_up(&mut random)?;
  assert_eq!(reader.num_docs()?, 1);

  assert!(reader.max_doc()? >= 1);
  let mut stored_fields = reader.stored_fields()?;
  let result = stored_fields.document(0)?;
  assert_eq!(
    DocHelper::num_fields(&result),
    DocHelper::num_fields(&test_doc) - DATA.unstored.len()
  );
  let fields = result.get_fields();
  for field in fields {
    assert!(NAME_VALUES.contains_key(field.name()));
  }

  Ok(())
}
#[test]
fn test_get_field_name_variations() -> Result<()> {
  let mut random = random();
  let (_dir, _doc, reader) = set_up(&mut random)?;

  let mut all_field_names = HashSet::new();
  let mut indexed_field_names = HashSet::new();
  let mut not_indexed_field_names = HashSet::new();
  let mut tv_field_names = HashSet::new();
  let mut no_tv_field_names = HashSet::new();

  let field_infos = reader.get_field_infos()?;
  for field_info in field_infos.iter() {
    let name = field_info.name.to_string();
    all_field_names.insert(name.clone());

    if *field_info.get_index_options() != IndexOptions::None {
      indexed_field_names.insert(name.clone());
    } else {
      not_indexed_field_names.insert(name.clone());
    }

    if field_info.has_term_vectors() {
      tv_field_names.insert(name.clone());
    } else if *field_info.get_index_options() != IndexOptions::None {
      no_tv_field_names.insert(name.clone());
    }
  }

  assert_eq!(all_field_names.len(), DATA.all.len());
  for s in &all_field_names {
    assert!(NAME_VALUES.contains_key(s) || s.is_empty());
  }

  assert_eq!(indexed_field_names.len(), DATA.indexed.len());
  for s in &indexed_field_names {
    assert!(DATA.indexed.contains_key(s) || s.is_empty());
  }

  assert_eq!(not_indexed_field_names.len(), DATA.unindexed.len());
  assert_eq!(tv_field_names.len(), DATA.term_vector.len());
  assert_eq!(no_tv_field_names.len(), DATA.no_term_vector.len());

  Ok(())
}
#[test]
fn test_terms() -> Result<()> {
  let mut random = random();
  let (_dir, _doc, reader) = set_up(&mut random)?;
  let reader = Arc::new(reader);
  let multi_reader = MultiReader::with_leaf_reader(vec![reader.clone()])?;
  let fields = get_indexed_fields(&multi_reader)?;
  for field in fields {
    let terms = get_terms(&multi_reader, &field)?;
    assert!(terms.is_some());
    let terms = terms.unwrap();
    let mut terms_enum = terms.iterator()?;
    while terms_enum.next()?.is_some() {
      let term = terms_enum.term()?;

      let field_value = match NAME_VALUES.get(&field).unwrap() {
        String(v) => v.clone(),
        Str(v) => v.to_string(),
        _ => unreachable!(),
      };
      assert!(field_value.contains(&term.utf8_to_string()?));
    }
  }

  let mut term_docs = TestUtil::docs_with_reader(
    &mut random,
    &multi_reader,
    TEXT_FIELD_1_KEY,
    &BytesRef::from_string("field"),
    None,
    0,
  )?
  .expect("term_docs should be some");
  assert_ne!(term_docs.next_doc()?, NO_MORE_DOCS);

  let mut term_docs = TestUtil::docs_with_reader(
    &mut random,
    &multi_reader,
    NO_NORMS_KEY,
    &BytesRef::from_string(NO_NORMS_TEXT),
    None,
    0,
  )?
  .expect("term_docs should be some");
  assert_ne!(term_docs.next_doc()?, NO_MORE_DOCS);

  let mut positions = get_term_postings_enum(
    &multi_reader,
    TEXT_FIELD_1_KEY,
    &BytesRef::from_string("field"),
  )?
  .expect("positions should be some");
  assert_ne!(positions.next_doc()?, NO_MORE_DOCS);
  assert_eq!(positions.doc_id(), 0);
  assert!(positions.next_position()? >= 0);

  Ok(())
}
#[test]
fn test_norms() -> Result<()> {
  let mut random = random();
  let (_dir, _doc, reader) = set_up(&mut random)?;
  let reader = Arc::new(reader);
  TestSegmentReader::check_norms(reader)?;
  Ok(())
}
#[test]
fn test_term_vectors() -> Result<()> {
  let mut random = random();
  let (_dir, _doc, reader) = set_up(&mut random)?;
  let reader = Arc::new(reader);

  let multi_reader = MultiReader::with_leaf_reader(vec![reader.clone()])?;

  let mut term_vectors = multi_reader.term_vectors()?;
  let tv0 = term_vectors.get(0)?.expect("tv0 should exist");
  let result = tv0.terms(TEXT_FIELD_2_KEY)?;
  assert!(result.is_some());
  let result = result.unwrap();

  assert_eq!(result.size()?, 3);

  let mut terms_enum = result.iterator()?;
  while terms_enum.next()?.is_some() {
    let term = terms_enum.term()?.utf8_to_string()?;
    let freq = terms_enum.total_term_freq()? as i32;
    assert!(FIELD_2_TEXT.contains(&term));
    assert!(freq > 0);
  }

  let results = term_vectors.get(0)?.expect("results should exist");
  assert_eq!(results.size()?, 3);

  Ok(())
}
#[test]
fn test_out_of_bounds_access() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

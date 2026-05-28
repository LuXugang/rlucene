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

use crate::core::index::field_infos::FieldNumbers;
use crate::core::index::fields::Fields;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::merge_state::remove_deletes;
use crate::core::index::multi_reader::MultiReader;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_merger::SegmentMerger;
use crate::core::index::segment_reader::SegmentReader;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term_vectors::TermVectors;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::store::IOContext;
use crate::core::store::merge_info::MergeInfo;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::info_stream::InfoStreamEnum;
use crate::core::util::long_values::LongValues;
use crate::core::util::{LATEST, StringHelper};
use crate::test::core::index::doc_helper::{
  DATA, DocHelper, FIELD_2_FREQS, FIELD_2_TEXT, TEXT_FIELD_2_KEY,
};
use crate::test::core::index::test_segment_reader::TestSegmentReader;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_io_context, new_io_context_with_default, random,
};
use crate::test::core::util::test_util::TestUtil;
use parking_lot::Mutex;
use rand::RngExt;
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestSegmentMerger;

#[test]
fn test_merge() -> Result<()> {
  let mut random = random();
  let merged_segment = "test";
  let mut doc1 = Document::new();
  DocHelper::setup_doc(&mut doc1);
  let dir = new_directory_shared(&mut random)?;
  let info1 = DocHelper::write_doc(&mut random, dir, doc1.clone())?;
  let reader1 = SegmentReader::new(&info1, LATEST.major, &new_io_context(&mut random)?)?;

  let mut doc2 = Document::new();
  DocHelper::setup_doc(&mut doc2);
  let dir = new_directory_shared(&mut random)?;
  let info2 = DocHelper::write_doc(&mut random, dir, doc2.clone())?;
  let reader2 = SegmentReader::new(&info2, LATEST.major, &new_io_context(&mut random)?)?;

  let merged_dir = new_directory_shared(&mut random)?;
  #[allow(clippy::vec_init_then_push)]
  let mut si = SegmentInfo::new(
    merged_dir.clone(),
    Some((*LATEST).clone()),
    None,
    merged_segment,
    -1,
    false,
    false,
    HashMap::new(),
    StringHelper::random_id(),
    HashMap::new(),
    None,
  )?;
  let info_stream = Arc::new(InfoStreamEnum::default());
  let readers = vec![reader1, reader2];
  let context = new_io_context_with_default(
    &mut random,
    &IOContext::with_merge(MergeInfo::new(-1, -1, false, -1))?,
  )?;
  let mut merger = SegmentMerger::new(
    readers.as_ref(),
    &mut si,
    info_stream,
    merged_dir.as_ref(),
    Arc::new(Mutex::new(FieldNumbers::new::<String, String>(None, None)?)),
    &context,
  )?;

  merger.merge()?;
  let docs_merged = merger.merge_state.segment_info.max_doc()?;
  assert_eq!(2, docs_merged);
  // Should be able to open a new SegmentReader against the new directory
  let merged_reader = Arc::new(SegmentReader::new(
    &SegmentCommitInfo::new(si, 0, 0, -1, -1, -1, Some(StringHelper::random_id()))?,
    LATEST.major,
    &new_io_context(&mut random)?,
  )?);

  assert_eq!(2, merged_reader.num_docs()?);

  let new_doc1 = merged_reader.stored_fields()?.document(0)?;
  assert_eq!(
    DocHelper::num_fields(&new_doc1),
    DocHelper::num_fields(&doc1) - DATA.unstored.len()
  );

  let new_doc2 = merged_reader.stored_fields()?.document(1)?;
  assert_eq!(
    DocHelper::num_fields(&new_doc2),
    DocHelper::num_fields(&doc2) - DATA.unstored.len()
  );
  let multi_readers = MultiReader::with_leaf_reader(vec![merged_reader.clone()])?;

  let term_docs = TestUtil::docs_with_reader(
    &mut random,
    &multi_readers,
    TEXT_FIELD_2_KEY,
    &BytesRef::from_string("field"),
    None,
    0,
  )?;
  debug_assert!(term_docs.is_some());
  assert_ne!(NO_MORE_DOCS, term_docs.unwrap().next_doc()?);

  let mut tv_count = 0;
  for field_info in merged_reader.get_field_infos()?.iter() {
    if field_info.has_term_vectors() {
      tv_count += 1;
    }
  }
  assert_eq!(3, tv_count);
  let vector = merged_reader
    .term_vectors()?
    .get(0)?
    .unwrap()
    .terms(TEXT_FIELD_2_KEY)?;
  let v = vector.unwrap();
  assert_eq!(3, v.size()?);

  let mut terms_enum = v.iterator()?;
  let mut i = 0;
  while (terms_enum.next()?).is_some() {
    let term = terms_enum.term()?.as_ref().utf8_to_string()?;
    let freq = terms_enum.total_term_freq()? as i32;
    assert!(FIELD_2_TEXT.contains(&term));
    assert_eq!(FIELD_2_FREQS[i], freq);
    i += 1;
  }

  TestSegmentReader::check_norms(merged_reader)?;

  Ok(())
}

#[test]
fn test_build_doc_map() -> Result<()> {
  let mut random = random();
  let max_doc = TestUtil::next_usize(&mut random, 1, 128);
  let num_docs = TestUtil::next_usize(&mut random, 0, max_doc);

  let mut live_docs = FixedBitSet::new(max_doc);
  for _ in 0..num_docs {
    loop {
      let doc_id = random.random_range(0..max_doc);
      if !live_docs.get(doc_id)? {
        live_docs.set(doc_id);
        break;
      }
    }
  }

  let doc_map = remove_deletes(max_doc as i32, &live_docs)?;

  let mut del = 0;
  for i in 0..max_doc {
    if !live_docs.get(i)? {
      del += 1;
    } else {
      assert_eq!(i - del, doc_map.get(i)? as usize);
    }
  }

  Ok(())
}

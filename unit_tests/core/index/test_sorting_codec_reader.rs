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
use crate::core::codecs::doc_values_producer::DocValuesProducer;
use crate::core::document::document::Document;
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::index::BytesRef;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, random,
};

use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::slow_codec_reader_wrapper::SlowCodecReaderWrapper;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::sorting_codec_reader::{SortingCodecReaderEnum, wrap};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::sort::Sort;
use crate::core::search::sorted_set_selector::SortedSetSelectorType::Min;
use crate::core::search::sorted_set_sort_field::SortedSetSortField;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
#[allow(dead_code)] // for quick search
struct TestSortingCodecReader;
#[test]
fn test_sort_on_add_indices_ord() -> Result<()> {
  let mut random = random();
  let tmp_dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  let w = IndexWriter::new(tmp_dir.clone(), iwc)?;

  let mut doc = Document::new();
  doc.add(SortedSetDocValuesField::new(
    "foo",
    BytesRef::from_string("b"),
  ));
  w.add_document(doc.clone())?;

  doc.add(SortedSetDocValuesField::new(
    "foo",
    BytesRef::from_string("a"),
  ));
  doc.add(SortedSetDocValuesField::new(
    "foo",
    BytesRef::from_string("b"),
  ));
  doc.add(SortedSetDocValuesField::new(
    "foo",
    BytesRef::from_string("b"),
  ));
  w.add_document(doc)?;

  w.commit()?;

  let index_sort = Sort::with_fields(vec![SortedSetSortField::with_selector("foo", false, Min)?])?;

  let reader = directory_reader::open(tmp_dir.clone())?;
  let reader = get_context(reader)?;
  for ctx in reader.leaves()? {
    let leaf_reader = ctx.reader().clone();
    let slow = SlowCodecReaderWrapper::wrap_leaf_reader(leaf_reader);
    let wrap = wrap(slow, index_sort.clone())?;

    let s = wrap.to_string();
    assert!(s.starts_with("SortingCodecReader("), "{}", s);
    match wrap {
      SortingCodecReaderEnum::Sorting(sorting_codec_reader) => {
        let fi = ctx
          .reader()
          .get_field_infos()?
          .field_info_by_name("foo")
          .expect("field foo must exist");

        let mut sorted_set_doc_values = sorting_codec_reader
          .get_doc_values_reader()?
          .expect("doc values reader must exist")
          .get_sorted_set(&fi)?;

        sorted_set_doc_values.next_doc()?;
        assert_eq!(sorted_set_doc_values.doc_value_count()?, 2);

        sorted_set_doc_values.next_doc()?;
        assert_eq!(sorted_set_doc_values.doc_value_count()?, 1);

        assert_eq!(sorted_set_doc_values.next_doc()?, NO_MORE_DOCS);
      },
      _ => unreachable!("wrap should be SortingCodecReader"),
    }
  }
  Ok(())
}

#[test]
fn test_sort_on_add_indices_int() -> Result<()> {
  // add_indexes未实现
  Ok(())
}

#[test]
fn test_sort_on_add_indices_random() -> Result<()> {
  // add_indexes未实现
  Ok(())
}

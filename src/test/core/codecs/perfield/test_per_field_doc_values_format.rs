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
use crate::core::codecs::Codecs;
use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::text_field::TextField;
use crate::core::index::binary_doc_values::BinaryDocValues;
use crate::core::index::directory_reader;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::term_query::TermQuery;
use crate::core::store::directory::Directory;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::codecs::asserting_codec::{AssertingCodec, AssertingCodecHook};
use crate::test_framework::core::codecs::asserting_doc_values_format::AssertingDocValuesFormat;
use crate::test_framework::core::codecs::perfield::test_per_field_doc_values_format::{
  DocValuesMergeWithIndexedFieldsAssertingCodec, MergeCalledOnTwoFormatsAssertingCodec,
  MergeRecordingDocValueFormatWrapper, TwoFieldsTwoFormatsDocValuesAssertingCodec,
};
use crate::test_framework::core::index::base_doc_values_format_test_case::BaseDocValuesFormatTestCase;
use crate::test_framework::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test_framework::core::index::legacy_base_doc_values_format_test_case::LegacyBaseDocValuesFormatTestCase;
use crate::test_framework::core::util::lucene_test_case::{
  new_bytes_ref_from_string, new_directory_shared, new_index_writer_config_with_analyzer,
  new_searcher_with_reader, new_text_field, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::prelude::StdRng;
use std::collections::{HashMap, HashSet};

/// Basic tests of PerFieldDocValuesFormat.
#[allow(dead_code)] // for quick search
struct TestPerFieldDocValuesFormat;

impl BaseIndexFileFormatTestCase for TestPerFieldDocValuesFormat {
  type Defaults = crate::test_framework::core::index::legacy_base_doc_values_format_test_case::LegacyBaseDocValuesFormatTestCaseDefaults;

  fn get_codec(&self) -> Result<Codecs> {
    // TODO IMPORTANT: Use the Java test's per-test RandomCodec once RandomCodec is implemented.
    Ok(TestUtil::get_default_codec().into())
  }
}

impl LegacyBaseDocValuesFormatTestCase for TestPerFieldDocValuesFormat {
  fn codec_accepts_huge_binary_values(&self, _field: &str) -> bool {
    true
  }
}
impl BaseDocValuesFormatTestCase for TestPerFieldDocValuesFormat {}

// just a simple trivial test
// TODO: we should come up with a test that somehow checks that segment suffix
// is respected by all codec apis (not just docvalues and postings)
#[test]
fn test_two_fields_two_formats() -> Result<()> {
  let mut random = random();
  let analyzer = MockAnalyzer::new(&mut random);
  let directory = new_directory_shared(&mut random)?;
  // we don't use RandomIndexWriter because it might add more docvalues than we expect !!!!1
  let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  let fast = TestUtil::get_default_doc_values_format().into();
  let slow = AssertingDocValuesFormat::new().into();
  config.set_codec(AssertingCodec::with_hook(
    AssertingCodecHook::TwoFieldsTwoFormatsDocValues(
      TwoFieldsTwoFormatsDocValuesAssertingCodec::new(fast, slow),
    ),
  ));

  let writer = IndexWriter::new(directory.clone(), config)?;
  let mut doc = Document::new();
  let long_term = "longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
       longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
       longtermlongterm";
  let text = format!("This is the text to be indexed. {long_term}");
  doc.add(new_text_field(
    &mut random,
    "fieldname",
    &text,
    Store::Yes,
    &mut HashMap::new(),
  )?);
  doc.add(NumericDocValuesField::new("dv1", 5));
  doc.add(BinaryDocValuesField::new(
    "dv2",
    new_bytes_ref_from_string(&mut random, "hello world")?,
  ));
  writer.add_document(doc)?;
  writer.close()?;

  // Now search the index:
  let reader = directory_reader::open(directory.clone())?; // read-only=true
  let searcher = new_searcher_with_reader(reader)?;
  assert_eq!(
    1,
    searcher.count(TermQuery::new(Term::from_text("fieldname", long_term)))?
  );
  let query = TermQuery::new(Term::from_text("fieldname", "text"));
  let hits = searcher.search(query, 1)?;
  assert_eq!(1, hits.total_hits.value());
  let mut stored_fields = searcher.stored_fields()?;
  // Iterate through the results:
  for score_doc in &hits.score_docs {
    let hit_doc_id = score_doc.doc;
    let hit_doc = stored_fields.document(hit_doc_id)?;
    assert_eq!(&text, hit_doc.get("fieldname")?.unwrap().as_ref());
    assert_eq!(1, searcher.get_leaf_contexts()?.len());

    let leaf = &searcher.get_leaf_contexts()?[0];
    let mut dv = leaf.reader().get_numeric_doc_values("dv1")?.unwrap();
    assert_eq!(hit_doc_id, dv.advance(hit_doc_id)?);
    assert_eq!(5, dv.long_value()?);

    let mut dv2 = leaf.reader().get_binary_doc_values("dv2")?.unwrap();
    assert_eq!(hit_doc_id, dv2.advance(hit_doc_id)?);
    assert_eq!(
      &new_bytes_ref_from_string(&mut random, "hello world")?,
      dv2.binary_value()?.as_ref()
    );
  }

  searcher.reader_context.reader().close()?;
  drop(searcher);
  directory.close()
}

#[test]
fn test_merge_called_on_two_formats() -> Result<()> {
  let mut random = random();
  let format1 = MergeRecordingDocValueFormatWrapper::new(TestUtil::get_default_doc_values_format());
  let format2 = MergeRecordingDocValueFormatWrapper::new(TestUtil::get_default_doc_values_format());

  let mut config = IndexWriterConfig::new()?;
  config.set_codec(AssertingCodec::with_hook(
    AssertingCodecHook::MergeCalledOnTwoFormats(MergeCalledOnTwoFormatsAssertingCodec::new(
      format1.clone().into(),
      format2.clone().into(),
    )),
  ));

  let directory = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(directory.clone(), config)?;

  let mut doc = Document::new();
  doc.add(NumericDocValuesField::new("dv1", 5));
  doc.add(NumericDocValuesField::new("dv2", 42));
  doc.add(BinaryDocValuesField::new(
    "dv3",
    new_bytes_ref_from_string(&mut random, "hello world")?,
  ));
  writer.add_document(doc)?;
  writer.commit()?;

  let mut doc = Document::new();
  doc.add(NumericDocValuesField::new("dv1", 8));
  doc.add(NumericDocValuesField::new("dv2", 45));
  doc.add(BinaryDocValuesField::new(
    "dv3",
    new_bytes_ref_from_string(&mut random, "goodbye world")?,
  ));
  writer.add_document(doc)?;
  writer.commit()?;

  writer.force_merge(1)?;
  writer.close()?;

  assert_eq!(1, format1.nb_merge_calls());
  assert_eq!(
    HashSet::from(["dv1".to_string(), "dv2".to_string()]),
    format1.field_names().into_iter().collect()
  );
  assert_eq!(1, format2.nb_merge_calls());
  assert_eq!(vec!["dv3".to_string()], format2.field_names());
  directory.close()
}

#[test]
fn test_doc_values_merge_with_indexed_fields() -> Result<()> {
  let mut random = random();
  let format = MergeRecordingDocValueFormatWrapper::new(TestUtil::get_default_doc_values_format());

  let mut config = IndexWriterConfig::new()?;
  config.set_codec(AssertingCodec::with_hook(
    AssertingCodecHook::DocValuesMergeWithIndexedFields(
      DocValuesMergeWithIndexedFieldsAssertingCodec::new(format.clone().into()),
    ),
  ));

  let directory = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(directory.clone(), config)?;

  let mut doc = Document::new();
  doc.add(NumericDocValuesField::new("dv1", 5));
  doc.add(TextField::from_string(
    "normalField",
    "not a doc value",
    Store::No,
  )?);
  writer.add_document(doc)?;
  writer.commit()?;

  let mut doc = Document::new();
  doc.add(TextField::from_string(
    "anotherField",
    "again no doc values here",
    Store::No,
  )?);
  doc.add(TextField::from_string(
    "normalField",
    "my document without doc values",
    Store::No,
  )?);
  writer.add_document(doc)?;
  writer.commit()?;

  writer.force_merge(1)?;
  writer.close()?;

  // "normalField" and "anotherField" are ignored when merging doc values.
  assert_eq!(1, format.nb_merge_calls());
  assert_eq!(vec!["dv1".to_string()], format.field_names());
  directory.close()
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestPerFieldDocValuesFormat, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestPerFieldDocValuesFormat;
  f(&case, &mut random)
}

mod base_doc_values_format_test_case_tests {
  use super::run_case;
  use crate::core::util::error::lucene_error::Result;
  use crate::test_framework::core::index::base_doc_values_format_test_case::BaseDocValuesFormatTestCase;

  #[test]
  fn test_sorted_merge_away_all_values_with_skipper() -> Result<()> {
    run_case(|case, random| case.test_sorted_merge_away_all_values_with_skipper(random))
  }

  #[test]
  fn test_sorted_set_merge_away_all_values_with_skipper() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_merge_away_all_values_with_skipper(random))
  }

  #[test]
  fn test_number_merge_away_all_values_with_skipper() -> Result<()> {
    run_case(|case, random| case.test_number_merge_away_all_values_with_skipper(random))
  }

  #[test]
  fn test_sorted_number_merge_away_all_values_with_skipper() -> Result<()> {
    run_case(|case, random| case.test_sorted_number_merge_away_all_values_with_skipper(random))
  }

  #[test]
  fn test_sorted_merge_away_all_values_large_segment_with_skipper() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_merge_away_all_values_large_segment_with_skipper(random)
    })
  }

  #[test]
  fn test_sorted_set_merge_away_all_values_large_segment_with_skipper() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_set_merge_away_all_values_large_segment_with_skipper(random)
    })
  }

  #[test]
  fn test_numeric_merge_away_all_values_large_segment_with_skipper() -> Result<()> {
    run_case(|case, random| {
      case.test_numeric_merge_away_all_values_large_segment_with_skipper(random)
    })
  }

  #[test]
  fn test_sorted_numeric_merge_away_all_values_large_segment_with_skipper() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_numeric_merge_away_all_values_large_segment_with_skipper(random)
    })
  }

  #[test]
  fn test_numeric_doc_values_with_skipper_small() -> Result<()> {
    run_case(|case, random| case.test_numeric_doc_values_with_skipper_small(random))
  }

  #[test]
  fn test_numeric_doc_values_with_skipper_medium() -> Result<()> {
    run_case(|case, random| case.test_numeric_doc_values_with_skipper_medium(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_numeric_doc_values_with_skipper_big() -> Result<()> {
    run_case(|case, random| case.test_numeric_doc_values_with_skipper_big(random))
  }

  #[test]
  fn test_sorted_numeric_doc_values_with_skipper_small() -> Result<()> {
    run_case(|case, random| case.test_sorted_numeric_doc_values_with_skipper_small(random))
  }

  #[test]
  fn test_sorted_numeric_doc_values_with_skipper_medium() -> Result<()> {
    run_case(|case, random| case.test_sorted_numeric_doc_values_with_skipper_medium(random))
  }
  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_sorted_numeric_doc_values_with_skipper_big() -> Result<()> {
    run_case(|case, random| case.test_sorted_numeric_doc_values_with_skipper_big(random))
  }
  #[test]
  fn test_sorted_doc_values_with_skipper_small() -> Result<()> {
    run_case(|case, random| case.test_sorted_doc_values_with_skipper_small(random))
  }

  #[test]
  fn test_sorted_doc_values_with_skipper_medium() -> Result<()> {
    run_case(|case, random| case.test_sorted_doc_values_with_skipper_medium(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_sorted_doc_values_with_skipper_big() -> Result<()> {
    run_case(|case, random| case.test_sorted_doc_values_with_skipper_big(random))
  }

  #[test]
  fn test_sorted_set_doc_values_with_skipper_small() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_doc_values_with_skipper_small(random))
  }

  #[test]
  fn test_sorted_set_doc_values_with_skipper_medium() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_doc_values_with_skipper_medium(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_sorted_set_doc_values_with_skipper_big() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_doc_values_with_skipper_big(random))
  }

  #[test]
  fn test_mismatched_fields() -> Result<()> {
    run_case(|case, random| case.test_mismatched_fields(random))
  }
}

mod legacy_base_doc_values_format_test_case_tests {
  use super::run_case;
  use crate::core::util::error::lucene_error::Result;
  use crate::test_framework::core::index::legacy_base_doc_values_format_test_case::LegacyBaseDocValuesFormatTestCase;

  #[test]
  fn test_one_number() -> Result<()> {
    run_case(|case, random| case.test_one_number(random))
  }

  #[test]
  fn test_one_float() -> Result<()> {
    run_case(|case, random| case.test_one_float(random))
  }

  #[test]
  fn test_two_numbers() -> Result<()> {
    run_case(|case, random| case.test_two_numbers(random))
  }

  #[test]
  fn test_two_binary_values() -> Result<()> {
    run_case(|case, random| case.test_two_binary_values(random))
  }

  #[test]
  fn test_variously_compressible_binary_values() -> Result<()> {
    run_case(|case, random| case.test_variously_compressible_binary_values(random))
  }

  #[test]
  fn test_two_fields_mixed() -> Result<()> {
    run_case(|case, random| case.test_two_fields_mixed(random))
  }

  #[test]
  fn test_three_fields_mixed() -> Result<()> {
    run_case(|case, random| case.test_three_fields_mixed(random))
  }

  #[test]
  fn test_three_fields_mixed2() -> Result<()> {
    run_case(|case, random| case.test_three_fields_mixed2(random))
  }

  #[test]
  fn test_two_documents_numeric() -> Result<()> {
    run_case(|case, random| case.test_two_documents_numeric(random))
  }

  #[test]
  fn test_two_documents_merged() -> Result<()> {
    run_case(|case, random| case.test_two_documents_merged(random))
  }

  #[test]
  fn test_big_numeric_range() -> Result<()> {
    run_case(|case, random| case.test_big_numeric_range(random))
  }

  #[test]
  fn test_big_numeric_range2() -> Result<()> {
    run_case(|case, random| case.test_big_numeric_range2(random))
  }

  #[test]
  fn test_bytes() -> Result<()> {
    run_case(|case, random| case.test_bytes(random))
  }

  #[test]
  fn test_bytes_two_documents_merged() -> Result<()> {
    run_case(|case, random| case.test_bytes_two_documents_merged(random))
  }

  #[test]
  fn test_bytes_merge_away_all_values() -> Result<()> {
    run_case(|case, random| case.test_bytes_merge_away_all_values(random))
  }

  #[test]
  fn test_sorted_bytes() -> Result<()> {
    run_case(|case, random| case.test_sorted_bytes(random))
  }

  #[test]
  fn test_sorted_bytes_two_documents() -> Result<()> {
    run_case(|case, random| case.test_sorted_bytes_two_documents(random))
  }

  #[test]
  fn test_sorted_bytes_three_documents() -> Result<()> {
    run_case(|case, random| case.test_sorted_bytes_three_documents(random))
  }

  #[test]
  fn test_sorted_bytes_two_documents_merged() -> Result<()> {
    run_case(|case, random| case.test_sorted_bytes_two_documents_merged(random))
  }

  #[test]
  fn test_sorted_merge_away_all_values() -> Result<()> {
    run_case(|case, random| case.test_sorted_merge_away_all_values(random))
  }

  #[test]
  fn test_bytes_with_newline() -> Result<()> {
    run_case(|case, random| case.test_bytes_with_newline(random))
  }

  #[test]
  fn test_missing_sorted_bytes() -> Result<()> {
    run_case(|case, random| case.test_missing_sorted_bytes(random))
  }

  #[test]
  fn test_sorted_terms_enum() -> Result<()> {
    run_case(|case, random| case.test_sorted_terms_enum(random))
  }

  #[test]
  fn test_empty_sorted_bytes() -> Result<()> {
    run_case(|case, random| case.test_empty_sorted_bytes(random))
  }

  #[test]
  fn test_empty_bytes() -> Result<()> {
    run_case(|case, random| case.test_empty_bytes(random))
  }

  #[test]
  fn test_very_large_but_legal_bytes() -> Result<()> {
    run_case(|case, random| case.test_very_large_but_legal_bytes(random))
  }

  #[test]
  fn test_very_large_but_legal_sorted_bytes() -> Result<()> {
    run_case(|case, random| case.test_very_large_but_legal_sorted_bytes(random))
  }

  #[test]
  fn test_codec_uses_own_bytes() -> Result<()> {
    run_case(|case, random| case.test_codec_uses_own_bytes(random))
  }

  #[test]
  fn test_codec_uses_own_sorted_bytes() -> Result<()> {
    run_case(|case, random| case.test_codec_uses_own_sorted_bytes(random))
  }

  #[test]
  fn test_doc_values_simple() -> Result<()> {
    run_case(|case, random| case.test_doc_values_simple(random))
  }
  #[test]
  fn test_random_sorted_bytes() -> Result<()> {
    run_case(|case, random| case.test_random_sorted_bytes(random))
  }
  #[test]
  fn test_boolean_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_boolean_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_boolean_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_boolean_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_byte_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_byte_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_byte_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_byte_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_short_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_short_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_short_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_short_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_int_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_int_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_int_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_int_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_long_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_long_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_long_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_long_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_binary_fixed_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_binary_fixed_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_binary_fixed_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_binary_fixed_length_vs_stored_fields(random))
  }

  #[test]
  fn test_binary_variable_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_binary_variable_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_binary_variable_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_binary_variable_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_fixed_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_fixed_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_sorted_fixed_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_sorted_fixed_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_variable_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_variable_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_sorted_variable_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_sorted_variable_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_set_one_value() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_one_value(random))
  }

  #[test]
  fn test_sorted_set_two_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_fields(random))
  }

  #[test]
  fn test_sorted_set_two_documents_merged() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_documents_merged(random))
  }

  #[test]
  fn test_sorted_set_two_values() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_values(random))
  }

  #[test]
  fn test_sorted_set_two_values_unordered() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_values_unordered(random))
  }

  #[test]
  fn test_sorted_set_three_values_two_docs() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_three_values_two_docs(random))
  }

  #[test]
  fn test_sorted_set_two_documents_last_missing() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_documents_last_missing(random))
  }

  #[test]
  fn test_sorted_set_two_documents_last_missing_merge() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_documents_last_missing_merge(random))
  }

  #[test]
  fn test_sorted_set_two_documents_first_missing() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_documents_first_missing(random))
  }

  #[test]
  fn test_sorted_set_two_documents_first_missing_merge() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_documents_first_missing_merge(random))
  }

  #[test]
  fn test_sorted_set_merge_away_all_values() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_merge_away_all_values(random))
  }

  #[test]
  fn test_sorted_set_terms_enum() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_terms_enum(random))
  }

  #[test]
  fn test_sorted_set_fixed_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_fixed_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_numerics_single_valued_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_numerics_single_valued_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_numerics_single_valued_missing_vs_stored_fields() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_numerics_single_valued_missing_vs_stored_fields(random)
    })
  }

  #[test]
  fn test_sorted_numerics_multiple_values_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_numerics_multiple_values_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_numerics_few_unique_sets_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_numerics_few_unique_sets_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_set_variable_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_variable_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_set_fixed_length_single_valued_vs_stored_fields() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_set_fixed_length_single_valued_vs_stored_fields(random)
    })
  }

  #[test]
  fn test_sorted_set_variable_length_single_valued_vs_stored_fields() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_set_variable_length_single_valued_vs_stored_fields(random)
    })
  }

  #[test]
  fn test_sorted_set_fixed_length_few_unique_sets_vs_stored_fields() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_set_fixed_length_few_unique_sets_vs_stored_fields(random)
    })
  }

  #[test]
  fn test_sorted_set_variable_length_few_unique_sets_vs_stored_fields() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_set_variable_length_few_unique_sets_vs_stored_fields(random)
    })
  }

  #[test]
  fn test_sorted_set_variable_length_many_values_per_doc_vs_stored_fields() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_set_variable_length_many_values_per_doc_vs_stored_fields(random)
    })
  }

  #[test]
  fn test_sorted_set_fixed_length_many_values_per_doc_vs_stored_fields() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_set_fixed_length_many_values_per_doc_vs_stored_fields(random)
    })
  }

  #[test]
  fn test_gcd_compression() -> Result<()> {
    run_case(|case, random| case.test_gcd_compression(random))
  }

  #[test]
  fn test_sparse_gcd_compression() -> Result<()> {
    run_case(|case, random| case.test_sparse_gcd_compression(random))
  }

  #[test]
  fn test_zeros() -> Result<()> {
    run_case(|case, random| case.test_zeros(random))
  }

  #[test]
  fn test_sparse_zeros() -> Result<()> {
    run_case(|case, random| case.test_sparse_zeros(random))
  }

  #[test]
  fn test_zero_or_min() -> Result<()> {
    run_case(|case, random| case.test_zero_or_min(random))
  }

  #[test]
  fn test_two_numbers_one_missing() -> Result<()> {
    run_case(|case, random| case.test_two_numbers_one_missing(random))
  }

  #[test]
  fn test_two_numbers_one_missing_with_merging() -> Result<()> {
    run_case(|case, random| case.test_two_numbers_one_missing_with_merging(random))
  }

  #[test]
  fn test_three_numbers_one_missing_with_merging() -> Result<()> {
    run_case(|case, random| case.test_three_numbers_one_missing_with_merging(random))
  }

  #[test]
  fn test_two_bytes_one_missing() -> Result<()> {
    run_case(|case, random| case.test_two_bytes_one_missing(random))
  }

  #[test]
  fn test_two_bytes_one_missing_with_merging() -> Result<()> {
    run_case(|case, random| case.test_two_bytes_one_missing_with_merging(random))
  }

  #[test]
  fn test_three_bytes_one_missing_with_merging() -> Result<()> {
    run_case(|case, random| case.test_three_bytes_one_missing_with_merging(random))
  }
  #[test]
  fn test_threads() -> Result<()> {
    run_case(|case, random| case.test_threads(random))
  }
  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_threads2() -> Result<()> {
    run_case(|case, random| case.test_threads2(random))
  }
  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_threads3() -> Result<()> {
    run_case(|case, random| case.test_threads3(random))
  }
  #[test]
  fn test_empty_binary_value_on_page_sizes() -> Result<()> {
    run_case(|case, random| case.test_empty_binary_value_on_page_sizes(random))
  }

  #[test]
  fn test_one_sorted_number() -> Result<()> {
    run_case(|case, random| case.test_one_sorted_number(random))
  }

  #[test]
  fn test_one_sorted_number_one_missing() -> Result<()> {
    run_case(|case, random| case.test_one_sorted_number_one_missing(random))
  }

  #[test]
  fn test_number_merge_away_all_values() -> Result<()> {
    run_case(|case, random| case.test_number_merge_away_all_values(random))
  }

  #[test]
  fn test_two_sorted_number() -> Result<()> {
    run_case(|case, random| case.test_two_sorted_number(random))
  }

  #[test]
  fn test_two_sorted_number_same_value() -> Result<()> {
    run_case(|case, random| case.test_two_sorted_number_same_value(random))
  }

  #[test]
  fn test_two_sorted_number_one_missing() -> Result<()> {
    run_case(|case, random| case.test_two_sorted_number_one_missing(random))
  }

  #[test]
  fn test_sorted_number_merge() -> Result<()> {
    run_case(|case, random| case.test_sorted_number_merge(random))
  }

  #[test]
  fn test_sorted_number_merge_away_all_values() -> Result<()> {
    run_case(|case, random| case.test_sorted_number_merge_away_all_values(random))
  }

  #[test]
  fn test_sorted_enum_advance_independently() -> Result<()> {
    run_case(|case, random| case.test_sorted_enum_advance_independently(random))
  }

  #[test]
  fn test_sorted_set_enum_advance_independently() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_enum_advance_independently(random))
  }

  #[test]
  fn test_sorted_merge_away_all_values_large_segment() -> Result<()> {
    run_case(|case, random| case.test_sorted_merge_away_all_values_large_segment(random))
  }

  #[test]
  fn test_sorted_set_merge_away_all_values_large_segment() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_merge_away_all_values_large_segment(random))
  }

  #[test]
  fn test_numeric_merge_away_all_values_large_segment() -> Result<()> {
    run_case(|case, random| case.test_numeric_merge_away_all_values_large_segment(random))
  }

  #[test]
  fn test_sorted_numeric_merge_away_all_values_large_segment() -> Result<()> {
    run_case(|case, random| case.test_sorted_numeric_merge_away_all_values_large_segment(random))
  }

  #[test]
  fn test_binary_merge_away_all_values_large_segment() -> Result<()> {
    run_case(|case, random| case.test_binary_merge_away_all_values_large_segment(random))
  }

  #[test]
  fn test_random_advance_numeric() -> Result<()> {
    run_case(|case, random| case.test_random_advance_numeric(random))
  }

  #[test]
  fn test_random_advance_binary() -> Result<()> {
    run_case(|case, random| case.test_random_advance_binary(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_high_ords_sorted_set_dv() -> Result<()> {
    run_case(|case, random| case.test_high_ords_sorted_set_dv(random))
  }
}

mod base_index_file_format_test_case_test {
  use super::run_case;
  use crate::core::util::error::lucene_error::Result;
  use crate::test_framework::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;

  #[test]
  fn test_merge_stability() -> Result<()> {
    run_case(|case, random| case.test_merge_stability(random))
  }

  #[test]
  fn test_multi_close() -> Result<()> {
    run_case(|case, random| case.test_multi_close(random))
  }

  #[test]
  fn test_random_exceptions() -> Result<()> {
    run_case(|case, random| case.test_random_exceptions(random))
  }

  #[test]
  fn test_check_integrity_reads_all_bytes() -> Result<()> {
    run_case(|case, random| case.test_check_integrity_reads_all_bytes(random))
  }
}

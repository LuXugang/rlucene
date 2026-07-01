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
use crate::core::document::field_type::FieldType;
use crate::core::index::BytesRef;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::term_vectors::TermVectors;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::support::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::support::core::index::base_term_vectors_format_test_case::BaseTermVectorsFormatTestCase;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
use crate::test::support::core::util::lucene_test_case::{
  get_only_leaf_reader, new_directory_shared, new_field, random,
};
use rand::Rng;
use rand::prelude::StdRng;
use std::collections::HashMap;

pub struct TestCompressingTermVectorsFormat;

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestCompressingTermVectorsFormat, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestCompressingTermVectorsFormat;
  f(&case, &mut random)
}
mod base_term_vectors_format_test_case_tests {
  use crate::codecs_tests::lucene90::compressing::test_compressing_term_vectors_format::run_case;
  use crate::core::util::error::lucene_error::Result;
  use crate::test::support::core::index::base_term_vectors_format_test_case::BaseTermVectorsFormatTestCase;

  #[test]
  fn test_rare_vectors() -> Result<()> {
    run_case(|case, random| case.test_rare_vectors(random))
  }

  #[test]
  fn test_high_freqs() -> Result<()> {
    run_case(|case, random| case.test_high_freqs(random))
  }

  #[test]
  fn test_lots_of_fields() -> Result<()> {
    run_case(|case, random| case.test_lots_of_fields(random))
  }

  #[test]
  fn test_mixed_options() -> Result<()> {
    run_case(|case, random| case.test_mixed_options(random))
  }

  #[test]
  fn test_random() -> Result<()> {
    run_case(|case, random| case.test_random(random))
  }

  #[test]
  fn test_merge() -> Result<()> {
    run_case(|case, random| case.test_merge(random))
  }

  #[test]
  fn test_merge_with_deletes() -> Result<()> {
    run_case(|case, random| case.test_merge_with_deletes(random))
  }

  #[test]
  fn test_merge_with_index_sort() -> Result<()> {
    run_case(|case, random| case.test_merge_with_index_sort(random))
  }

  #[test]
  fn test_merge_with_index_sort_and_deletes() -> Result<()> {
    run_case(|case, random| case.test_merge_with_index_sort_and_deletes(random))
  }

  #[test]
  fn test_clone() -> Result<()> {
    run_case(|case, random| case.test_clone(random))
  }
  #[test]
  fn test_postings_enum_freqs() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_freqs(random))
  }
  #[test]
  fn test_postings_enum_positions() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_positions(random))
  }
  #[test]
  fn test_postings_enum_offsets() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_offsets(random))
  }
  #[test]
  fn test_postings_enum_offsets_without_positions() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_offsets_without_positions(random))
  }
  #[test]
  fn test_postings_enum_payloads() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_payloads(random))
  }
  #[test]
  fn test_postings_enum_all() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_all(random))
  }
}

impl BaseIndexFileFormatTestCase for TestCompressingTermVectorsFormat {
  fn add_random_fields<R>(_random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    todo!()
  }
}

impl BaseTermVectorsFormatTestCase for TestCompressingTermVectorsFormat {}

#[test]
fn test_no_ords() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir)?;

  let mut doc = Document::new();
  let mut ft = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  ft.set_store_term_vectors(true)?;
  doc.add(new_field(
    &mut random,
    "foo",
    "this is a test",
    &ft,
    &mut HashMap::new(),
  )?);
  iw.add_document(&mut random, doc)?;

  let ir = get_only_leaf_reader(&iw.get_reader(&mut random)?)?;
  let mut term_vectors = ir.term_vectors()?;
  let terms = term_vectors.get_field_terms(0, "foo")?;
  assert!(terms.is_some());

  let terms = terms.unwrap();
  let mut terms_enum = terms.iterator()?;
  assert_eq!(
    SeekStatus::Found,
    terms_enum.seek_ceil(&BytesRef::from_string("this"))?
  );

  let err = terms_enum.ord();
  assert!(matches!(err, Err(LuceneError::UnsupportedOperation(_))));

  let err = terms_enum.seek_exact_with_ord(0);
  assert!(matches!(err, Err(LuceneError::UnsupportedOperation(_))));

  iw.close(&mut random)?;
  Ok(())
}
#[test]
fn test_chunk_cleanup() -> Result<()> {
  // TODO IMPORTANT setCodec未实现
  Ok(())
}

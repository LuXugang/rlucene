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
use crate::core::codecs::competitive_impact_accumulator::CompetitiveImpactAccumulator;
use crate::core::codecs::lucene90::block_tree::field_reader::FieldReader;
use crate::core::codecs::lucene90::block_tree::stats::Stats;
use crate::core::codecs::lucene101::lucene101_postings_reader::{
  MutableImpactList, read_impacts, read_vint15, read_vlong15,
};
use crate::core::codecs::lucene101::lucene101_postings_writer::{
  write_impacts, write_vint15, write_vlong15,
};
use crate::core::codecs::postings_reader_base::PostingsReaderBase;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::index::directory_reader;
use crate::core::index::impact::Impact;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::terms::{Terms, TermsEnum2};
use crate::core::store::directory::Directory;
use crate::core::store::{
  ByteArrayDataInput, ByteArrayDataOutput, DataInput, IOContext, IndexInput,
};
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::asserting_leaf_reader::AssertingTerms;
use crate::test_framework::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test_framework::core::index::base_postings_format_test_case::BasePostingsFormatTestCase;
use crate::test_framework::core::index::random_postings_tester::RandomPostingsTester;
use crate::test_framework::core::util::lucene_test_case::{
  get_only_leaf_reader, new_directory_shared, new_string_field, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use parking_lot::Mutex;
use rand::prelude::StdRng;
use rand::{Rng, RngExt};
use std::collections::HashMap;
use std::sync::LazyLock;

#[allow(dead_code)] // for quick search
struct TestLucene101PostingsFormat;

trait BlockTreeStats {
  fn block_stats(&self) -> Result<Stats>;
}

impl<I, PR> BlockTreeStats for FieldReader<I, PR>
where
  I: IndexInput,
  PR: PostingsReaderBase,
{
  fn block_stats(&self) -> Result<Stats> {
    self.get_stats_object()
  }
}

impl<A, B> BlockTreeStats for TermsEnum2<A, B>
where
  A: BlockTreeStats,
{
  fn block_stats(&self) -> Result<Stats> {
    match self {
      Self::A(inner) => inner.block_stats(),
      Self::B(_) => Err(LuceneError::unsupported_operation(
        "block statistics are only available for the BlockTree terms variant",
      )),
    }
  }
}

impl<T> BlockTreeStats for AssertingTerms<T>
where
  T: Terms + BlockTreeStats,
{
  fn block_stats(&self) -> Result<Stats> {
    self.with_inner(|inner| inner.block_stats())
  }
}

static POSTINGS_TESTER: LazyLock<Mutex<RandomPostingsTester>> = LazyLock::new(|| {
  let mut random = random();
  Mutex::new(
    RandomPostingsTester::new(&mut random)
      .expect("failed to initialize TestLucene101PostingsFormat"),
  )
});

impl BaseIndexFileFormatTestCase for TestLucene101PostingsFormat {
  type Defaults = crate::test_framework::core::index::base_postings_format_test_case::BasePostingsFormatTestCaseDefaults;

  fn get_codec(&self) -> Result<Codecs> {
    Ok(TestUtil::get_default_codec().into())
  }
}
impl BasePostingsFormatTestCase for TestLucene101PostingsFormat {
  fn create_postings<R>(&self, _random: &mut R) -> &Mutex<RandomPostingsTester>
  where
    R: Rng + ?Sized,
  {
    &POSTINGS_TESTER
  }
}
fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestLucene101PostingsFormat, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestLucene101PostingsFormat;
  let codec_guard = case.set_up()?;
  let result = f(&case, &mut random);
  case.tear_down(codec_guard);
  result
}

#[test]
fn test_vint15() -> Result<()> {
  let buffer = vec![0u8; 5];
  let mut out = ByteArrayDataOutput::with_bytes(buffer);
  for &i in &[0i32, 1, 127, 128, 32767, 32768, i32::MAX] {
    out.reset()?;
    write_vint15(&mut out, i)?;
    let mut inp = ByteArrayDataInput::with_bytes(out.bytes.as_slice());
    let v = read_vint15(&mut inp)?;
    assert_eq!(v, i);
    assert_eq!(inp.get_position(), out.get_position());
  }
  Ok(())
}
#[test]
fn test_vlong15() -> Result<()> {
  // buffer size should accommodate the largest encoded value
  let mut out = ByteArrayDataOutput::with_bytes(vec![0u8; 9]);
  for &i in &[0i64, 1, 127, 128, 32_767, 32_768, i32::MAX as i64, i64::MAX] {
    out.reset()?;
    write_vlong15(&mut out, i)?;
    let mut inp = ByteArrayDataInput::with_bytes(out.bytes.as_slice());
    let v = read_vlong15(&mut inp)?;
    assert_eq!(v, i);
    assert_eq!(inp.get_position(), out.get_position());
  }
  Ok(())
}
#[test]
fn test_final_block() -> Result<()> {
  run_case(|_, random| {
    let dir = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let iwc = IndexWriterConfig::with_analyzer(analyzer)?;
    let w = IndexWriter::new(dir.clone(), iwc)?;
    let mut field_types = HashMap::new();

    for i in 0..25 {
      let suffix = char::from(b'a' + i);
      let mut doc = Document::new();
      doc.add(new_string_field(
        random,
        "field",
        suffix.to_string(),
        Store::No,
        &mut field_types,
      )?);
      doc.add(new_string_field(
        random,
        "field",
        format!("z{suffix}"),
        Store::No,
        &mut field_types,
      )?);
      w.add_document(doc)?;
    }
    w.force_merge(1)?;

    let reader = directory_reader::open_from_writer(&w)?;
    let leaf = get_only_leaf_reader(&reader)?;
    let terms = leaf.terms("field")?.expect("field terms should exist");
    let stats = BlockTreeStats::block_stats(&terms)?;
    assert_eq!(stats.floor_block_count, 0);
    assert_eq!(stats.non_floor_block_count, 2);

    reader.close()?;
    w.close()?;
    dir.close()?;
    Ok(())
  })
}
#[test]
fn test_impact_serialization() -> Result<()> {
  let cases = vec![
    vec![Impact { freq: 1, norm: 1 }],
    vec![Impact { freq: 1, norm: 42 }],
    vec![Impact {
      freq: 1,
      norm: -100,
    }],
    vec![Impact { freq: 30, norm: 1 }],
    vec![Impact { freq: 500, norm: 1 }],
    vec![
      Impact { freq: 1, norm: 7 },
      Impact { freq: 3, norm: 9 },
      Impact { freq: 7, norm: 10 },
      Impact { freq: 15, norm: 11 },
      Impact { freq: 20, norm: 13 },
      Impact { freq: 28, norm: 14 },
    ],
    vec![
      Impact { freq: 2, norm: 2 },
      Impact { freq: 10, norm: 10 },
      Impact { freq: 12, norm: 50 },
      Impact {
        freq: 50,
        norm: -100,
      },
      Impact {
        freq: 1000,
        norm: -80,
      },
      Impact {
        freq: 1005,
        norm: -3,
      },
    ],
  ];

  for impacts in cases {
    do_test_impact_serialization(&impacts)?;
  }

  Ok(())
}
fn do_test_impact_serialization(impacts: &[Impact]) -> Result<()> {
  let mut random = random();
  let mut acc = CompetitiveImpactAccumulator::new();
  for imp in impacts {
    acc.add(imp.freq, imp.norm);
  }
  let dir = new_directory_shared(&mut random)?;
  {
    let mut out = dir.create_output("foo", &IOContext::default_io_context()?)?;
    write_impacts(&acc.get_competitive_freq_norm_pairs(), &mut out)?;
  }
  let mut input = dir.open_input("foo", &IOContext::default_io_context()?)?;
  let len = input.length()?;
  let mut buffer = vec![0u8; len];
  input.read_bytes(&mut buffer, 0, len)?;

  let mut data_in = ByteArrayDataInput::with_bytes(buffer.as_slice());
  let mut mutable_impacts_list =
    MutableImpactList::with_capacity(impacts.len() + random.random_range(0..3));
  read_impacts(&mut data_in, &mut mutable_impacts_list)?;
  let len = mutable_impacts_list.length;
  assert_eq!(&mutable_impacts_list.impacts[0..len], impacts);
  Ok(())
}

mod base_postings_format_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::codecs::lucene101::test_lucene101_postings_format::run_case;
  use crate::test_framework::core::index::base_postings_format_test_case::BasePostingsFormatTestCase;
  #[test]
  fn test_docs_only() -> Result<()> {
    run_case(|case, random| case.test_docs_only(random))
  }
  #[test]
  fn test_docs_and_freqs() -> Result<()> {
    run_case(|case, random| case.test_docs_and_freqs(random))
  }
  #[test]
  fn test_docs_and_freqs_and_positions() -> Result<()> {
    run_case(|case, random| case.test_docs_and_freqs_and_positions(random))
  }
  #[test]
  fn test_docs_and_freqs_and_positions_and_payloads() -> Result<()> {
    run_case(|case, random| case.test_docs_and_freqs_and_positions_and_payloads(random))
  }
  #[test]
  fn test_docs_and_freqs_and_positions_and_offsets() -> Result<()> {
    run_case(|case, random| case.test_docs_and_freqs_and_positions_and_offsets(random))
  }
  #[test]
  fn test_docs_and_freqs_and_positions_and_offsets_and_payloads() -> Result<()> {
    run_case(|case, random| case.test_docs_and_freqs_and_positions_and_offsets_and_payloads(random))
  }
  #[test]
  fn test_random() -> Result<()> {
    run_case(|case, random| case.test_random(random))
  }
  #[test]
  fn test_postings_enum_reuse() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_reuse(random))
  }
  #[test]
  fn test_just_empty_field() -> Result<()> {
    run_case(|case, random| case.test_just_empty_field(random))
  }
  #[test]
  fn test_empty_field_and_empty_term() -> Result<()> {
    run_case(|case, random| case.test_empty_field_and_empty_term(random))
  }
  #[test]
  fn test_didnt_want_freqs_but_asked_anyway() -> Result<()> {
    run_case(|case, random| case.test_didnt_want_freqs_but_asked_anyway(random))
  }
  #[test]
  fn test_ask_for_positions_when_not_there() -> Result<()> {
    run_case(|case, random| case.test_ask_for_positions_when_not_there(random))
  }
  #[test]
  fn test_ghosts() -> Result<()> {
    run_case(|case, random| case.test_ghosts(random))
  }
  #[test]
  fn test_disorder() -> Result<()> {
    run_case(|case, random| case.test_disorder(random))
  }
  #[test]
  fn test_binary_search_term_leaf() -> Result<()> {
    run_case(|case, random| case.test_binary_search_term_leaf(random))
  }
  #[test]
  fn test_level2_ghosts() -> Result<()> {
    run_case(|case, random| case.test_level2_ghosts(random))
  }
  #[test]
  fn test_inverted_write() -> Result<()> {
    run_case(|case, random| case.test_inverted_write(random))
  }
  #[test]
  fn test_postings_enum_docs_only() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_docs_only(random))
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
  fn test_postings_enum_payloads() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_payloads(random))
  }
  #[test]
  fn test_postings_enum_all() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_all(random))
  }
  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_line_file_docs() -> Result<()> {
    run_case(|case, random| case.test_line_file_docs(random))
  }
  #[test]
  fn test_mismatched_fields() -> Result<()> {
    run_case(|case, random| case.test_mismatched_fields(random))
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

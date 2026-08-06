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
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicy;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::Directory;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::codecs::test_minimal_codec::{MinimalCodec, MinimalCompoundCodec};
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_merge_policy_with_mock_mp,
  random,
};
use rand::RngExt;

/// Tests to ensure that codecs won't need to implement all formats when only a small subset of
/// Lucene's functionality is used.
#[allow(dead_code)] // for quick search
struct TestMinimalCodec;

#[test]
fn test_minimal_codec() -> Result<()> {
  run_minimal_codec_test(false)
}

#[test]
fn test_minimal_compound_codec() -> Result<()> {
  run_minimal_codec_test(true)
}

fn run_minimal_codec_test(use_compound_file: bool) -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut writer_config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  if use_compound_file {
    writer_config.set_codec(MinimalCompoundCodec::new());
  } else {
    writer_config.set_codec(MinimalCodec::new());
  }
  writer_config.set_use_compound_file(use_compound_file);
  if !use_compound_file {
    // Avoid using MockMP as it randomly enables compound file creation
    writer_config.set_merge_policy(new_merge_policy_with_mock_mp(&mut random, false)?);
    writer_config
      .get_merge_policy_mut()
      .get_base_mut()
      .set_no_cfs_ratio(0.0)?;
    writer_config
      .get_merge_policy_mut()
      .get_base_mut()
      .set_max_cfs_segment_size_mb(f64::INFINITY)?;
  }

  let writer = IndexWriter::new(dir.clone(), writer_config)?;
  writer.add_document(basic_document())?;
  writer.flush()?;
  // create second segment
  writer.add_document(basic_document())?;
  writer.force_merge(1)?; // test merges
  if random.random_bool(0.5) {
    writer.commit()?;
  }

  let reader = directory_reader::open_from_writer(&writer)?;
  assert_eq!(2, reader.num_docs()?);
  reader.close()?;
  writer.close()?;
  dir.close()
}

/// Returns a basic document with no indexed fields.
fn basic_document() -> Document {
  Document::new()
}

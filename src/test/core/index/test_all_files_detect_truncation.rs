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
use crate::core::codecs::CodecUtil;
use crate::core::document::document::Document;
use crate::core::document::field::{Field, FieldBase, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::knn_float_vector_field::KnnFloatVectorField;
use crate::core::document::long_point::LongPoint;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::TYPE_STORED;
use crate::core::index::BytesRef;
use crate::core::index::check_index::Level;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::WRITE_LOCK_NAME;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicy;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::store::{DataOutput, IOContext, IndexInput, IndexOutput};
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  is_night_mode, new_directory_shared, new_index_writer_config_with_analyzer, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::sync::Arc;

/// Test that a plain default detects index file truncation early (on opening a reader).
#[allow(dead_code)] // for quick search
struct TestAllFilesDetectTruncation;

#[test]
fn test() -> Result<()> {
  let mut random = random();
  do_test(&mut random, false)
}

#[test]
fn test_cfs() -> Result<()> {
  let mut random = random();
  do_test(&mut random, true)
}

fn do_test<R>(random: &mut R, cfs: bool) -> Result<()>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;

  let analyzer = MockAnalyzer::new(random);
  let mut conf = new_index_writer_config_with_analyzer(random, analyzer)?;
  conf.set_codec(TestUtil::get_default_codec());

  // Disable CFS 80% of the time so we can truncate individual files, but the other 20% of the
  // time we test truncation of .cfs/.cfe too:
  if !cfs {
    conf.set_use_compound_file(false);
    conf
      .get_merge_policy_mut()
      .get_base_mut()
      .set_no_cfs_ratio(0.0)?;
  }

  let riw = RandomIndexWriter::with_config(random, dir.clone(), conf);
  let mut text_with_term_vectors_type = FieldType::from_ref(&*TYPE_STORED)?;
  text_with_term_vectors_type.set_store_term_vectors(true)?;
  let mut text = Field::from_string("text", "", text_with_term_vectors_type)?;
  let mut term_string = StringField::from_string("string", "", Store::Yes)?;
  let mut dv_string = SortedDocValuesField::new("string", BytesRef::new());
  let mut point_number = LongPoint::new("long", [0])?;
  let mut dv_number = NumericDocValuesField::new("long", 0);
  let mut vector = KnnFloatVectorField::new("vector", vec![0.0; 16])?;

  for i in 0..100 {
    text.set_string_value(TestUtil::random_analysis_string(random, 20, true))?;
    let random_string = TestUtil::random_simple_string_with_len(random, 5);
    term_string.set_string_value(&random_string)?;
    dv_string.set_bytes_value(BytesRef::from_string(&random_string))?;
    let number = random.random_range(0..10_i64);
    point_number.set_long_value(number)?;
    dv_number.set_long_value(number)?;
    vector.set_vector_value(vec![(i % 4) as f32; 16])?;

    let mut doc = Document::new();
    doc.add(text.clone());
    doc.add(term_string.clone());
    doc.add(dv_string.clone());
    doc.add(point_number.clone());
    doc.add(dv_number.clone());
    doc.add(vector.clone());
    riw.add_document(random, doc)?;
  }

  if !is_night_mode() {
    riw.force_merge(random, 1)?;
  }

  riw.delete_documents_with_queries(
    random,
    vec![LongPoint::new_range_query("long", 0, 2)?.into()],
  )?;

  riw.close(random)?;
  check_truncation(random, dir.clone())?;
  dir.close()
}

fn check_truncation<R>(random: &mut R, dir: Arc<DirEnum>) -> Result<()>
where
  R: Rng + ?Sized,
{
  for name in dir.list_all()? {
    if name != WRITE_LOCK_NAME {
      truncate_one_file(random, dir.clone(), &name)?;
    }
  }
  Ok(())
}

fn truncate_one_file<R>(random: &mut R, dir: Arc<DirEnum>, victim: &str) -> Result<()>
where
  R: Rng + ?Sized,
{
  let dir_copy = new_directory_shared(random)?;
  let result = (|| -> Result<()> {
    let victim_length = dir.file_length(victim)?;
    assert!(victim_length > 0);
    let lost_bytes = TestUtil::next_usize(random, 1, 100.min(victim_length));

    if cfg!(feature = "test_log_verbose") {
      println!(
        "TEST: now truncate file {} by removing {} of {} bytes",
        victim, lost_bytes, victim_length
      );
    }

    let default_context = IOContext::default_io_context()?;
    let read_once_context = IOContext::read_once_io_context()?;
    for name in dir.list_all()? {
      if name != victim {
        dir_copy.copy_from(dir.as_ref(), &name, &name, &default_context)?;
      } else {
        let mut input = dir.open_checksum_input(&name)?;
        let footer_result = CodecUtil::check_footer(&mut input);
        let footer_result = IOUtils::use_or_suppress_result(footer_result, input.close());
        match footer_result {
          Ok(_) => {
            // In some rare cases, the codec footer would still appear as correct even though the
            // file has been truncated. We just skip the test is this rare case.
            return Ok(());
          },
          Err(LuceneError::CorruptIndex(_)) => {
            // expected
          },
          Err(err) => return Err(err),
        }

        let mut out = dir_copy.create_output(&name, &default_context)?;
        let mut input = match dir.open_input(&name, &read_once_context) {
          Ok(input) => input,
          Err(err) => {
            return IOUtils::use_or_suppress_result(Err(err), out.close());
          },
        };
        let copy_result = out.copy_bytes(&mut input, victim_length - lost_bytes);
        let copy_result = IOUtils::use_or_suppress_result(copy_result, input.close());
        IOUtils::use_or_suppress_result(copy_result, out.close())?;
      }
      dir_copy.sync(std::slice::from_ref(&name))?;
    }

    // There needs to be an exception thrown, but we don't care about its type, it's too heroic to
    // ensure that a specific exception type gets throws upon opening an index.
    // NOTE: we .close so that if the test fails (truncation not detected) we don't also get all
    // these confusing errors about open files:
    let open_result = directory_reader::open(dir_copy.clone()).and_then(|reader| reader.close());
    if open_result.is_ok() {
      return Err(LuceneError::illegal_state(format!(
        "truncation of {victim} was not detected"
      )));
    }

    // CheckIndex should also fail:
    if TestUtil::check_index_with_options(
      random,
      dir_copy.clone(),
      Level::MIN_LEVEL_FOR_SLOW_CHECKS,
      true,
      true,
      None,
    )
    .is_ok()
    {
      return Err(LuceneError::illegal_state(format!(
        "CheckIndex did not detect truncation of {victim}"
      )));
    }

    Ok(())
  })();

  IOUtils::use_or_suppress_result(result, dir_copy.close())
}

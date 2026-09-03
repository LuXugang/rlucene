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
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::WRITE_LOCK_NAME;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicy;
use crate::core::store::IO_CONTEXT_DEFAULT;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::store::{DataInput, DataOutput, IOContext, IndexInput, IndexOutput};
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::sync::Arc;

/// Test that the default codec detects mismatched checksums at open or checkIntegrity time.
#[allow(dead_code)] // for quick search
struct TestAllFilesDetectMismatchedChecksum;

#[test]
fn test() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  conf.set_codec(TestUtil::get_default_codec());
  // Disable CFS, which makes it harder to test due to its double checksumming
  conf.set_use_compound_file(false);
  conf
    .get_merge_policy_mut()
    .get_base_mut()
    .set_no_cfs_ratio(0.0)?;

  let riw = RandomIndexWriter::with_config(&mut random, dir.clone(), conf);
  let mut text_with_term_vectors_type = FieldType::from_ref(&*TYPE_STORED)?;
  text_with_term_vectors_type.set_store_term_vectors(true)?;
  let mut text = Field::from_string("text", "", text_with_term_vectors_type)?;
  let mut term_string = StringField::from_string("string", "", Store::Yes)?;
  let mut dv_string = SortedDocValuesField::new("string", BytesRef::new());
  let mut point_number = LongPoint::new("long", [0])?;
  let mut dv_number = NumericDocValuesField::new("long", 0);
  let mut vector = KnnFloatVectorField::new("vector", vec![0.0; 16])?;

  for i in 0..100 {
    text.set_string_value(TestUtil::random_analysis_string(&mut random, 20, true))?;
    let random_string = TestUtil::random_simple_string_with_len(&mut random, 5);
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
    riw.add_document(&mut random, doc)?;
  }
  riw.delete_documents_with_queries(
    &mut random,
    vec![LongPoint::new_range_query("long", 0, 2)?.into()],
  )?;
  riw.close(&mut random)?;
  check_mismatched_checksum(&mut random, dir.clone())?;
  dir.close()
}

fn check_mismatched_checksum<R>(random: &mut R, dir: Arc<DirEnum>) -> Result<()>
where
  R: Rng + ?Sized,
{
  for name in dir.list_all()? {
    if name != WRITE_LOCK_NAME {
      corrupt_file(random, dir.clone(), &name)?;
    }
  }
  Ok(())
}

fn corrupt_file<R>(random: &mut R, dir: Arc<DirEnum>, victim: &str) -> Result<()>
where
  R: Rng + ?Sized,
{
  let dir_copy = new_directory_shared(random)?;
  dir_copy.set_check_index_on_close(false);
  let result = (|| -> Result<()> {
    let victim_length = dir.file_length(victim)?;
    let flip_offset = TestUtil::next_usize(
      random,
      victim_length.saturating_sub(CodecUtil::footer_length()),
      victim_length - 1,
    );

    if cfg!(feature = "test_log_verbose") {
      println!(
        "TEST: now corrupt file {} by changing byte at offset {} (length= {})",
        victim, flip_offset, victim_length
      );
    }

    let default_context = IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?;
    let read_once_context = IOContext::read_once_io_context()?;
    for name in dir.list_all()? {
      if name != victim {
        dir_copy.copy_from(dir.as_ref(), &name, &name, default_context)?;
      } else {
        let mut out = dir_copy.create_output(&name, default_context)?;
        let mut input = match dir.open_input(&name, &read_once_context) {
          Ok(input) => input,
          Err(err) => {
            return IOUtils::use_or_suppress_result(Err(err), out.close());
          },
        };
        let copy_result = (|| -> Result<()> {
          out.copy_bytes(&mut input, flip_offset)?;
          let value = input.read_byte()?;
          out.write_byte(value.wrapping_add(random.random_range(0x01..=0xff)))?;
          out.copy_bytes(&mut input, victim_length - flip_offset - 1)
        })();
        let copy_result = IOUtils::use_or_suppress_result(copy_result, input.close());
        IOUtils::use_or_suppress_result(copy_result, out.close())?;
      }
      dir_copy.sync(std::slice::from_ref(&name))?;
    }

    // corruption must be detected
    let corruption_result = match directory_reader::open(dir_copy.clone()) {
      Ok(reader) => {
        let integrity_result = (|| -> Result<()> {
          let context = (&reader).get_context()?;
          for leaf in context.leaves()? {
            leaf.reader().check_integrity()?;
          }
          Ok(())
        })();
        IOUtils::use_or_suppress_result(integrity_result, reader.close())
      },
      Err(err) => Err(err),
    };

    match corruption_result {
      Err(LuceneError::CorruptIndex(_)) => Ok(()),
      Err(err) => Err(err),
      Ok(()) => Err(LuceneError::illegal_state(format!(
        "mismatched checksum in {victim} was not detected"
      ))),
    }
  })();

  IOUtils::use_or_suppress_result(result, dir_copy.close())
}

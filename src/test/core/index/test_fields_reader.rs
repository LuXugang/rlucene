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
use crate::core::codecs::dummy::stored_fields_writer::DummyStoredFieldsWriter;
use crate::core::document::document::Document;
use crate::core::document::document_stored_field_visitor::DocumentStoredFieldVisitor;
use crate::core::index::directory_reader;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::field_infos::FieldNumbers;
use crate::core::index::field_infos::build::Builder;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicy;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::store::directory::DirEnum;
use crate::core::store::{BufferedIndexInput, BufferedIndexInputBase, IndexInput};
use crate::core::util::ReadableCursorExt;
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::doc_helper::{
  DocHelper, NO_TF_KEY, TEXT_FIELD_1_KEY, TEXT_FIELD_2_KEY, TEXT_FIELD_3_KEY,
};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_log_merge_policy, random,
};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[allow(dead_code)] // for quick search
struct TestFieldsReader;

fn before_class() -> Result<(Document, Arc<DirEnum>)> {
  let mut random = random();

  let mut test_doc = Document::new();
  let mut field_infos = Builder::new(Arc::new(Mutex::new(FieldNumbers::new::<String, String>(
    None, None,
  )?)));
  DocHelper::setup_doc(&mut test_doc);

  for field in test_doc.get_fields() {
    let ift = field.field_type();
    field_infos.add(Arc::new(FieldInfo::new(
      field.name().to_string(),
      -1,
      false,
      ift.omit_norms(),
      false,
      *ift.index_options(),
      *ift.doc_values_type(),
      *ift.doc_values_skip_index_type(),
      -1,
      HashMap::new(),
      0,
      0,
      0,
      0,
      VectorEncoding::FLOAT32(4),
      VectorSimilarityFunction::Euclidean,
      false,
      false,
    )?))?;
  }

  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, mock);
  let mut mp = new_log_merge_policy(&mut random)?;
  mp.get_base_mut().set_no_cfs_ratio(0.0)?;
  conf.set_merge_policy(mp);

  let writer = IndexWriter::new(dir.clone(), conf)?;
  writer.add_document(test_doc.clone())?;
  writer.close()?;

  Ok((test_doc, dir))
}
#[test]
fn test() -> Result<()> {
  let (_, dir) = before_class()?;
  let reader = directory_reader::open(dir.clone())?;
  let doc = reader.stored_fields()?.document(0)?;
  assert!(doc.get_field(TEXT_FIELD_1_KEY).is_some());

  let field = doc.get_field(TEXT_FIELD_2_KEY);
  assert!(field.is_some());
  let field = field.unwrap();
  assert!(field.field_type().store_term_vectors());
  assert!(!field.field_type().omit_norms());
  assert_eq!(
    IndexOptions::DocsAndFreqsAndPositions,
    *field.field_type().index_options()
  );

  let field = doc.get_field(TEXT_FIELD_3_KEY);
  assert!(field.is_some());
  let field = field.unwrap();
  assert!(!field.field_type().store_term_vectors());
  assert!(field.field_type().omit_norms());
  assert_eq!(
    IndexOptions::DocsAndFreqsAndPositions,
    *field.field_type().index_options()
  );

  let field = doc.get_field(NO_TF_KEY);
  assert!(field.is_some());
  let field = field.unwrap();
  assert!(!field.field_type().store_term_vectors());
  assert!(!field.field_type().omit_norms());
  assert_eq!(IndexOptions::Docs, *field.field_type().index_options());
  let mut v = HashSet::new();
  v.insert(TEXT_FIELD_3_KEY.to_string());
  let mut visitor = DocumentStoredFieldVisitor::with_fields(&v);
  reader.stored_fields()?.document_with_visitor(
    0,
    &mut visitor,
    Some(&mut DummyStoredFieldsWriter),
  )?;
  let visited_doc = visitor.get_document_ref();
  let fields = visited_doc.get_fields();

  assert_eq!(1, fields.len());
  assert_eq!(TEXT_FIELD_3_KEY, fields[0].name());

  Ok(())
}
#[test]
fn test_exceptions() -> Result<()> {
  // TODO FaultyIndexInput 的 clone 未实现
  Ok(())
}

struct FaultyIndexInput<I>
where
  I: IndexInput<IndexInput = I>,
{
  do_fail: Arc<AtomicBool>,
  delegate: I,
  count: i32,
}
impl<I> FaultyIndexInput<I>
where
  I: IndexInput<IndexInput = I>,
{
  fn new(do_fail: Arc<AtomicBool>, delegate: I) -> Self {
    Self {
      do_fail,
      delegate,
      count: 0,
    }
  }
  fn sim_outage(&mut self) -> Result<()> {
    if self.do_fail.load(Ordering::SeqCst) {
      let count = self.count;
      self.count += 1;

      if count % 2 == 1 {
        return Err(LuceneError::illegal_state("Simulated network outage"));
      }
    }
    Ok(())
  }
}

impl<I> TryClone for FaultyIndexInput<I>
where
  I: IndexInput<IndexInput = I>,
{
  fn try_clone(&self) -> Result<Self>
  where
    Self: Sized,
  {
    let _i = FaultyIndexInput::new(self.do_fail.clone(), self.delegate.try_clone()?);
    todo!()
  }
}

impl<I> BufferedIndexInputBase for FaultyIndexInput<I>
where
  I: IndexInput<IndexInput = I>,
{
  fn seek_internal(&mut self, _pos: usize) -> Result<()> {
    Ok(())
  }

  fn read_internal(
    &mut self,
    b: &mut Cursor<Vec<u8>>,
    _len: usize,
    file_pointer: usize,
  ) -> Result<()> {
    self.sim_outage()?;
    self.delegate.seek(file_pointer)?;
    let len = b.remain()?;
    let offset = b.position();
    self
      .delegate
      .read_bytes(b.get_mut(), offset as usize, len)?;

    b.set_position(len as u64);
    Ok(())
  }

  type Slice = BufferedIndexInput<FaultyIndexInput<I>>;

  fn slice(&self, slice_description: &str, offset: usize, length: usize) -> Result<Self::Slice> {
    let slice = self.delegate.slice(slice_description, offset, length)?;
    let fii = FaultyIndexInput::new(self.do_fail.clone(), slice);
    let d = format!("FaultyIndexInput({})", self.delegate);
    BufferedIndexInput::with_buffer_size(fii, &d, 1024)
  }

  fn length(&self) -> usize {
    self.delegate.length()
  }
}

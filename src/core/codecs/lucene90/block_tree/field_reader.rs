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
use crate::core::codecs::block_tree::intersect_terms_enum::IntersectTermsEnum;
use crate::core::codecs::block_tree::lucene90_block_tree_terms_reader::{
  OUTPUT_FLAG_HAS_TERMS, VERSION_MSB_VLONG_OUTPUT,
};
use crate::core::codecs::lucene90::block_tree::lucene90_block_tree_terms_reader::TermsReader;
use crate::core::codecs::lucene90::block_tree::segment_terms_enum::SegmentTermsEnum;
use crate::core::codecs::postings_reader_base::PostingsReaderBase;
use crate::core::index::BytesRef;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::terms::Terms;
use crate::core::store::{ByteArrayDataInput, DataInput, IndexInput};
use crate::core::util::automation::compiled_automaton::{AutomatonType, CompiledAutomaton};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fst_impl::byte_sequence_outputs::ByteSequenceOutputs;
use crate::core::util::fst_impl::fst::{FST, FSTMetadata, read_metadata};
use crate::core::util::fst_impl::off_heap_fst_store::OffHeapFSTStore;
use crate::core::util::{ToInt, TryIntoInt};
use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

/// BlockTree's implementation of [`Terms`].
#[allow(clippy::type_complexity)]
pub struct FieldReader<I, PR>
where
  I: IndexInput,
  PR: PostingsReaderBase,
{
  pub(crate) num_terms: i64,
  pub(crate) field_info: Arc<FieldInfo>,
  pub(crate) sum_total_term_freq: i64,
  pub(crate) sum_doc_freq: i64,
  pub(crate) doc_count: i32,
  pub(crate) root_block_fp: i64,
  pub(crate) root_code: BytesRef<Arc<Vec<u8>>>,
  pub(crate) min_term: Arc<BytesRef<Vec<u8>>>,
  pub(crate) max_term: Arc<BytesRef<Vec<u8>>>,
  pub(crate) parent: Option<Arc<TermsReader<I, PR>>>,
  pub(crate) index: Option<Arc<FST<ByteSequenceOutputs, OffHeapFSTStore<I>>>>,
  tmp_data: Option<TmpData>,
}
struct TmpData {
  pub(crate) metadata: FSTMetadata<ByteSequenceOutputs>,
  index_start_fp: i64,
  pub(crate) root_code: BytesRef<Vec<u8>>,
}
impl<I, PR> FieldReader<I, PR>
where
  I: IndexInput,
  PR: PostingsReaderBase,
{
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn new<I1>(
    field_info: Arc<FieldInfo>,
    num_terms: i64,
    root_code: BytesRef<Vec<u8>>,
    sum_total_term_freq: i64,
    sum_doc_freq: i64,
    doc_count: i32,
    index_start_fp: i64,
    meta_in: &mut I1,
    min_term: Arc<BytesRef<Vec<u8>>>,
    max_term: Arc<BytesRef<Vec<u8>>>,
  ) -> Result<Self>
  where
    I1: IndexInput,
  {
    debug_assert!(num_terms > 0);
    // Read FST metadata and build the index
    let metadata = read_metadata(meta_in, ByteSequenceOutputs)?;
    let tmp = {
      TmpData {
        metadata,
        index_start_fp,
        root_code,
      }
    };
    Ok(Self {
      parent: None,
      field_info,
      num_terms,
      sum_total_term_freq,
      sum_doc_freq,
      doc_count,
      // init with padding value
      root_block_fp: 0,
      // init with padding value
      root_code: BytesRef::new(),
      min_term,
      max_term,
      index: None,
      tmp_data: Some(tmp),
    })
  }
  pub fn init_field_reader(index_in: Arc<I>, reader: &mut FieldReader<I, PR>) -> Result<()> {
    let tmp_data = match reader.tmp_data.take() {
      Some(tmp_data) => tmp_data,
      None => {
        return Err(LuceneError::illegal_state(
          "TmpData is None, cannot init FieldReader".to_string(),
        ));
      },
    };
    let store = OffHeapFSTStore::new(
      index_in,
      tmp_data.index_start_fp as usize,
      tmp_data.metadata.num_bytes as usize,
    );
    let index = match FST::from_fst_reader(tmp_data.metadata, store) {
      Some(fst) => fst,
      None => {
        return Err(LuceneError::illegal_state(
          "FST metadata and store are some, should not return None".to_string(),
        ));
      },
    };
    let empty_output = index.metadata().empty_output().cloned();
    reader.index = Some(Arc::new(index));
    // ownership to ByteArrayDataInput
    let mut input = ByteArrayDataInput::with_range(
      tmp_data.root_code.bytes.as_slice(),
      tmp_data.root_code.offset,
      tmp_data.root_code.length,
    );
    reader.root_block_fp =
      ((reader.read_vlong_output(&mut input)? as u64) >> OUTPUT_FLAG_HAS_TERMS).try_convert()?;
    // ownership from ByteArrayDataInput
    let root_code = BytesRef {
      bytes: Arc::new(tmp_data.root_code.bytes),
      offset: tmp_data.root_code.offset,
      length: tmp_data.root_code.length,
    };
    // Get empty output and adjust rootCode
    let root_code_final = match empty_output {
      Some(empty_output) => {
        if root_code.bytes_equals(&empty_output) {
          empty_output
        } else {
          root_code
        }
      },
      None => root_code,
    };
    reader.root_code = root_code_final;
    Ok(())
  }

  pub(crate) fn read_vlong_output(&self, input: &mut impl DataInput) -> Result<i64> {
    let version = self.parent.as_ref().unwrap().version;
    if version >= VERSION_MSB_VLONG_OUTPUT {
      read_msb_vlong(input)
    } else {
      input.read_vlong()
    }
  }
}
impl<I, PR> Terms for FieldReader<I, PR>
where
  I: IndexInput,
  PR: PostingsReaderBase,
{
  type TermsEnum = SegmentTermsEnum<I, PR>;

  fn iterator(&self) -> Result<Self::TermsEnum> {
    SegmentTermsEnum::new(self.clone())
  }

  type IntersectIter = IntersectTermsEnum<I, PR>;

  fn intersect(
    &self,
    compiled: &CompiledAutomaton,
    start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<Self::IntersectIter> {
    if compiled.type_ != AutomatonType::Normal {
      return Err(LuceneError::illegal_argument(
        "please use CompiledAutomaton.getTermsEnum instead",
      ));
    }
    IntersectTermsEnum::new(
      self.clone(),
      compiled.get_transition_accessor()?,
      compiled.get_byte_runnable()?,
      compiled.common_suffix_ref.clone(),
      start_term,
    )
  }

  fn size(&self) -> Result<i64> {
    Ok(self.num_terms)
  }

  fn get_sum_total_term_freq(&self) -> Result<i64> {
    Ok(self.sum_total_term_freq)
  }

  fn get_sum_doc_freq(&self) -> Result<i64> {
    Ok(self.sum_doc_freq)
  }

  fn get_doc_count(&self) -> Result<i32> {
    Ok(self.doc_count)
  }

  fn has_freqs(&self) -> bool {
    self
      .field_info
      .get_index_options()
      .cmp(&IndexOptions::DocsAndFreqs)
      .to_int()
      >= 0
  }

  fn has_offsets(&self) -> bool {
    self
      .field_info
      .get_index_options()
      .cmp(&IndexOptions::DocsAndFreqsAndPositionsAndOffsets)
      .to_int()
      >= 0
  }

  fn has_positions(&self) -> bool {
    self
      .field_info
      .get_index_options()
      .cmp(&IndexOptions::DocsAndFreqsAndPositions)
      .to_int()
      >= 0
  }

  fn has_payloads(&self) -> bool {
    self.field_info.has_payloads()
  }

  fn get_min(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(Some(Cow::Borrowed(self.min_term.as_ref())))
  }

  fn get_max(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(Some(Cow::Borrowed(self.max_term.as_ref())))
  }
}
impl<I, PR> fmt::Display for FieldReader<I, PR>
where
  I: IndexInput,
  PR: PostingsReaderBase,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "BlockTreeTerms(seg={} terms={} postings={} positions={} docs={})",
      self.parent.as_ref().unwrap().segment,
      self.num_terms,
      self.sum_doc_freq,
      self.sum_total_term_freq,
      self.doc_count
    )
  }
}
impl<I, PR> Clone for FieldReader<I, PR>
where
  I: IndexInput,
  PR: PostingsReaderBase,
{
  // used to init SegmentTermsEnum
  fn clone(&self) -> Self {
    Self {
      num_terms: self.num_terms,
      field_info: self.field_info.clone(),
      sum_total_term_freq: self.sum_total_term_freq,
      sum_doc_freq: self.sum_doc_freq,
      doc_count: self.doc_count,
      root_block_fp: self.root_block_fp,
      root_code: self.root_code.clone(),
      min_term: self.min_term.clone(),
      max_term: self.max_term.clone(),
      parent: self.parent.clone(),
      index: Some(Arc::clone(self.index.as_ref().unwrap())),
      tmp_data: None,
    }
  }
}

/// Decodes a variable-length `byte[]` in MSB order back to a `long`,
/// as written by
/// [`Lucene90BlockTreeTermsWriter::write_msb_vlong`](crate::core::codecs::lucene90::block_tree::lucene90_block_tree_terms_writer::write_msb_vlong).
///
///
/// Package-private for testing.
pub(crate) fn read_msb_vlong(input: &mut impl DataInput) -> Result<i64> {
  let mut l: i64 = 0;
  loop {
    let b = input.read_byte()?;
    l = (l << 7) | (b & 0x7F) as i64;
    if (b & 0x80) == 0 {
      break;
    }
  }
  Ok(l)
}

#[cfg(test)]
mod tests {
  use crate::core::codecs::dummy::stored_fields_writer::DummyStoredFieldsWriter;
  use crate::core::document::document::Document;
  use crate::core::document::document_stored_field_visitor::DocumentStoredFieldVisitor;
  use crate::core::index::directory_reader::directory_reader_util;
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
      )))?;
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
    let reader = directory_reader_util::open(dir.clone())?;
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
}

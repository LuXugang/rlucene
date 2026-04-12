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
use crate::core::codecs::block_term_state::TermStateEnum;
use crate::core::codecs::fields_consumer::FieldsConsumer;
use crate::core::codecs::norms_producer::NormsProducer;
use crate::core::codecs::postings_format::PostingsFormat;
use crate::core::index::BytesRef;
use crate::core::index::automaton_terms_enum::AutomatonTermsEnum;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::fields::Fields;
use crate::core::index::filtered_terms_enum::FilteredTermsEnum;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::index::postings_enum::{FREQS, OFFSETS, PAYLOADS, POSITIONS, feature_requested};
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::store::flush_info::FlushInfo;
use crate::core::util::ToInt;
use crate::core::util::automation::compiled_automaton::CompiledAutomaton;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::dummy::dummy_attribute_source::DummyAttributeSource;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::info_stream::get_default_info_stream;
use crate::core::util::iterator::{VecIter, VecIteratorExt};
use crate::core::util::string_helper::StringHelper;
use crate::core::util::version::LATEST;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random_from_seed;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random_multiplier;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{at_least, is_night_mode};
use crate::test::core::util::test_util::TestUtil;
use rand::rngs::StdRng;
use rand::{Rng, RngExt};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

/// Which features to test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Option_ {
  /// Sometimes use `.advance()`.
  Skipping,

  /// Sometimes reuse the `PostingsEnum` across terms.
  ReuseEnums,

  /// Sometimes pass non-null live docs.
  LiveDocs,

  /// Sometimes seek to term using previously saved `TermState`.
  TermState,

  /// Sometimes don't fully consume docs from the enum.
  PartialDocConsume,

  /// Sometimes don't fully consume positions at each doc.
  PartialPosConsume,

  /// Sometimes check payloads.
  Payloads,

  /// Test w/ multiple threads.
  Threads,
}
/// Helper class extracted from BasePostingsFormatTestCase to exercise a postings format.
pub struct RandomPostingsTester {
  fields: HashMap<String, BTreeMap<BytesRef<Vec<u8>>, SeedAndOrd>>,
  field_infos: Arc<FieldInfos>,
  current_field_infos: Option<Arc<FieldInfos>>,
  all_terms: Vec<FieldAndTerm>,
  max_doc: i32,
  total_postings: i64,
  total_payload_bytes: i64,
  random: u64,
}

impl RandomPostingsTester {
  pub fn new<R>(random: &mut R) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    let seed = random.next_u64();
    let mut random = random_from_seed(seed);

    let mut fields: HashMap<String, BTreeMap<BytesRef<Vec<u8>>, SeedAndOrd>> = HashMap::new();
    let num_fields = TestUtil::next_int(&mut random, 1, 5) as usize;

    let mut field_info_array: Vec<Arc<FieldInfo>> = Vec::with_capacity(num_fields);
    let mut max_doc = 0;
    let mut total_postings = 0i64;
    let mut total_payload_bytes = 0i64;

    for field_upto in 0..num_fields {
      let field = loop {
        let field = TestUtil::random_simple_string(&mut random);
        if !fields.contains_key(&field) {
          break field;
        }
      };

      field_info_array.push(Arc::new(FieldInfo::new(
        field.clone(),
        field_upto as i32,
        false,
        false,
        true,
        IndexOptions::DocsAndFreqsAndPositionsAndOffsets,
        DocValuesType::None,
        DocValuesSkipIndexType::None,
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
      )));

      let mut postings: BTreeMap<BytesRef<Vec<u8>>, SeedAndOrd> = BTreeMap::new();
      let mut seen_terms: HashSet<String> = HashSet::new();

      let num_terms = if random.random_range(0..10) == 7 {
        at_least(&mut random, 50) as usize
      } else {
        TestUtil::next_int(&mut random, 2, 20) as usize
      };

      while postings.len() < num_terms {
        let term_upto = postings.len();
        let mut term = loop {
          let term = TestUtil::random_simple_string(&mut random);
          if !seen_terms.contains(&term) {
            break term;
          }
        };
        seen_terms.insert(term.clone());

        if is_night_mode() && term_upto == 0 && field_upto == 0 {
          term = format!("big_{term}");
        } else if term_upto == 1 && field_upto == 0 {
          term = format!("medium_{term}");
        } else if random.random_bool(0.5) {
          term = format!("low_{term}");
        } else {
          term = format!("verylow_{term}");
        }

        let term_seed = random.next_u64();
        postings.insert(BytesRef::from(term.clone()), SeedAndOrd::new(term_seed));

        let mut docs_enum = get_seed_postings(&term, term_seed, IndexOptions::Docs, true);
        let mut last_doc = 0;
        let mut doc_count = 0i64;
        loop {
          let doc = docs_enum.next_doc()?;
          if doc == NO_MORE_DOCS {
            break;
          }
          doc_count += 1;
          last_doc = doc;
        }
        max_doc = max_doc.max(last_doc);
        total_postings += doc_count;
        total_payload_bytes += doc_count * docs_enum.payload_size as i64;
      }

      let mut ord = 0i64;
      #[allow(clippy::explicit_counter_loop)]
      for ent in postings.values_mut() {
        ent.ord = ord;
        ord += 1;
      }

      fields.insert(field, postings);
    }

    let field_infos = Arc::new(FieldInfos::new(field_info_array)?);
    max_doc += 1;

    let mut all_terms = Vec::new();
    let mut field_names: Vec<String> = fields.keys().cloned().collect();
    field_names.sort();
    for field in field_names {
      if let Some(field_terms) = fields.get(&field) {
        for (ord, term) in field_terms.keys().enumerate() {
          all_terms.push(FieldAndTerm::new(field.clone(), term, ord as i64));
        }
      }
    }

    Ok(Self {
      fields,
      field_infos,
      current_field_infos: None,
      all_terms,
      max_doc,
      total_postings,
      total_payload_bytes,
      random: seed,
    })
  }

  pub fn build_index<C, D>(
    &mut self,
    codec: &C,
    dir: Arc<D>,
    max_allowed: IndexOptions,
    allow_payloads: bool,
    always_test_max: bool,
  ) -> Result<<C::PostingsFormat as PostingsFormat>::FieldsProducer<D::IndexInput>>
  where
    C: crate::core::codecs::codec::Codec,
    C::PostingsFormat: PostingsFormat,
    D: Directory,
  {
    let segment_info = SegmentInfo::new(
      Arc::clone(&dir),
      Some(LATEST.clone()),
      Some(LATEST.clone()),
      "_0",
      self.max_doc,
      false,
      false,
      HashMap::new(),
      StringHelper::random_id(),
      HashMap::new(),
      None,
    )?;

    let values: Vec<IndexOptions> = IndexOptions::values().collect();
    let max_index_option = values
      .iter()
      .position(|v| *v == max_allowed)
      .ok_or_else(|| {
        LuceneError::illegal_argument(format!("unsupported maxAllowed: {max_allowed}"))
      })?;
    if max_index_option == 0 {
      return Err(LuceneError::illegal_argument(
        "maxAllowed must be at least Docs".to_string(),
      ));
    }

    let mut random = random_from_seed(self.random);
    let mut new_field_info_array: Vec<Arc<FieldInfo>> = Vec::with_capacity(self.fields.len());
    for (field_upto, old_field_info) in self.field_infos.iter().enumerate() {
      let index_options = if always_test_max {
        values[max_index_option]
      } else {
        values[TestUtil::next_int(&mut random, 1, max_index_option as i32) as usize]
      };
      let do_payloads = index_options >= IndexOptions::DocsAndFreqsAndPositions && allow_payloads;

      new_field_info_array.push(Arc::new(FieldInfo::new(
        old_field_info.name.clone(),
        field_upto as i32,
        false,
        false,
        do_payloads,
        index_options,
        DocValuesType::None,
        DocValuesSkipIndexType::None,
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
      )));
    }

    let new_field_infos = Arc::new(FieldInfos::new(new_field_info_array)?);
    self.current_field_infos = Some(Arc::clone(&new_field_infos));

    let bytes = self.total_postings * 8 + self.total_payload_bytes;
    let io_context = IOContext::with_flush(FlushInfo::new(self.max_doc, bytes))?;
    let write_state = SegmentWriteState::new(
      get_default_info_stream(),
      dir.as_ref(),
      Arc::clone(&new_field_infos),
      &io_context,
    );

    let mut seed_fields = SeedFields::new(
      self.fields.clone(),
      Arc::clone(&new_field_infos),
      max_allowed,
      allow_payloads,
    );
    let norms = NormsProducerImpl::new(Arc::clone(&new_field_infos), self.max_doc);

    let mut consumer = codec
      .postings_format()
      .fields_consumer(&write_state, &segment_info)?;
    consumer.write(&mut seed_fields, Some(&norms))?;
    consumer.close()?;

    let read_state = SegmentReadState::new(dir.as_ref(), Arc::clone(&new_field_infos), &io_context);
    codec
      .postings_format()
      .fields_producer(&read_state, &segment_info)
  }
}
#[derive(Clone)]
struct SeedAndOrd {
  seed: u64,
  ord: i64,
}

impl SeedAndOrd {
  fn new(seed: u64) -> Self {
    Self { seed, ord: 0 }
  }
}

pub struct SeedFields {
  fields: HashMap<String, BTreeMap<BytesRef<Vec<u8>>, SeedAndOrd>>,
  field_infos: Arc<FieldInfos>,
  max_allowed: IndexOptions,
  allow_payloads: bool,
  keys: Vec<String>,
}

impl SeedFields {
  fn new(
    fields: HashMap<String, BTreeMap<BytesRef<Vec<u8>>, SeedAndOrd>>,
    field_infos: Arc<FieldInfos>,
    max_allowed: IndexOptions,
    allow_payloads: bool,
  ) -> Self {
    let mut keys = fields.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    Self {
      fields,
      field_infos,
      max_allowed,
      allow_payloads,
      keys,
    }
  }
}

impl Fields for SeedFields {
  type FieldIter<'a>
    = VecIter<'a, String>
  where
    Self: 'a;

  fn iterator(&self) -> Result<Self::FieldIter<'_>> {
    Ok(self.keys.iter_ext())
  }

  type Terms = SeedTerms;

  fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
    match self.fields.get(field) {
      Some(terms) => {
        let field_info = self.field_infos.field_info_by_name(field).ok_or_else(|| {
          LuceneError::illegal_state(format!("missing FieldInfo for field {field}"))
        })?;
        Ok(Some(SeedTerms::new(
          terms.clone(),
          field_info,
          self.max_allowed,
          self.allow_payloads,
        )))
      },
      None => Ok(None),
    }
  }

  fn size(&self) -> Result<i32> {
    Ok(self.fields.len() as i32)
  }
}

pub struct SeedTerms {
  terms: BTreeMap<BytesRef<Vec<u8>>, SeedAndOrd>,
  field_info: Arc<FieldInfo>,
  max_allowed: IndexOptions,
  allow_payloads: bool,
}

impl SeedTerms {
  fn new(
    terms: BTreeMap<BytesRef<Vec<u8>>, SeedAndOrd>,
    field_info: Arc<FieldInfo>,
    max_allowed: IndexOptions,
    allow_payloads: bool,
  ) -> Self {
    Self {
      terms,
      field_info,
      max_allowed,
      allow_payloads,
    }
  }
}

impl Terms for SeedTerms {
  type TermsEnum = SeedTermsEnum;
  type IntersectIter = FilteredTermsEnum<Self::TermsEnum, AutomatonTermsEnum>;

  fn iterator(&self) -> Result<Self::TermsEnum> {
    let mut terms_enum =
      SeedTermsEnum::new(self.terms.clone(), self.max_allowed, self.allow_payloads);
    terms_enum.reset();
    Ok(terms_enum)
  }

  fn intersect(
    &self,
    compiled: &CompiledAutomaton,
    start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<Self::IntersectIter> {
    self.default_intersect(compiled, start_term)
  }

  fn size(&self) -> Result<i64> {
    Ok(self.terms.len() as i64)
  }

  fn get_sum_total_term_freq(&self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_sum_doc_freq(&self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_doc_count(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn has_freqs(&self) -> bool {
    *self.field_info.get_index_options() >= IndexOptions::DocsAndFreqs
  }

  fn has_offsets(&self) -> bool {
    *self.field_info.get_index_options() >= IndexOptions::DocsAndFreqsAndPositionsAndOffsets
  }

  fn has_positions(&self) -> bool {
    *self.field_info.get_index_options() >= IndexOptions::DocsAndFreqsAndPositions
  }

  fn has_payloads(&self) -> bool {
    self.allow_payloads && self.field_info.has_payloads()
  }
}

pub struct SeedPostings {
  // Used only to generate docIDs; this way if you pull w/
  // or w/o positions you get the same docID sequence:
  doc_random: StdRng,
  random: StdRng,
  pub doc_freq: i32,
  max_doc_spacing: i32,
  payload_size: i32,
  fixed_payloads: bool,
  payload: BytesRef<Vec<u8>>,
  do_positions: bool,
  allow_payloads: bool,

  doc_id: i32,
  freq: i32,
  pub upto: i32,

  pos: i32,
  offset: i32,
  start_offset: i32,
  end_offset: i32,
  pos_spacing: i32,
  pos_upto: i32,
}

impl SeedPostings {
  pub fn new(
    seed: u64,
    min_doc_freq: i32,
    max_doc_freq: i32,
    options: IndexOptions,
    allow_payloads: bool,
  ) -> Self {
    let mut random = random_from_seed(seed);
    let doc_random_seed = random.next_u64();
    let doc_random = random_from_seed(doc_random_seed);
    let doc_freq = TestUtil::next_int(&mut random, min_doc_freq, max_doc_freq);
    let max_doc_spacing = TestUtil::next_int(&mut random, 1, 100);

    let payload_size = if random.random_range(0..10) == 7 {
      1 + random.random_range(0..3)
    } else {
      1 + random.random_range(0..1)
    };

    let fixed_payloads = random.random_bool(0.5);
    let payload_bytes = vec![0u8; payload_size as usize];
    let payload = BytesRef::from_bytes(payload_bytes);
    let do_positions = IndexOptions::DocsAndFreqsAndPositions
      .cmp(&options)
      .to_int()
      <= 0;

    Self {
      doc_random,
      random,
      doc_freq,
      max_doc_spacing,
      payload_size,
      fixed_payloads,
      payload,
      do_positions,
      allow_payloads,
      doc_id: -1,
      freq: 0,
      upto: 0,
      pos: 0,
      offset: 0,
      start_offset: 0,
      end_offset: 0,
      pos_spacing: 0,
      pos_upto: 0,
    }
  }
}

impl SeedPostings {
  fn _next_doc(&mut self) -> Result<i32> {
    if self.doc_id == -1 {
      self.doc_id = 0;
    }

    while self.pos_upto < self.freq {
      self.next_position()?;
    }

    if self.upto < self.doc_freq {
      if self.upto == 0 && self.doc_random.random_bool(0.5) {
      } else if self.max_doc_spacing == 1 {
        self.doc_id += 1;
      } else {
        self.doc_id += TestUtil::next_int(&mut self.doc_random, 1, self.max_doc_spacing);
      }

      if self.random.random_range(0..200) == 17 {
        self.freq = TestUtil::next_int(&mut self.random, 1, 1000);
      } else if self.random.random_range(0..10) == 17 {
        self.freq = TestUtil::next_int(&mut self.random, 1, 20);
      } else {
        self.freq = TestUtil::next_int(&mut self.random, 1, 4);
      }

      self.pos = 0;
      self.offset = 0;
      self.pos_upto = 0;
      self.pos_spacing = TestUtil::next_int(&mut self.random, 1, 100);

      self.upto += 1;
      Ok(self.doc_id)
    } else {
      self.doc_id = NO_MORE_DOCS;
      Ok(self.doc_id)
    }
  }
}

impl DocIdSetIterator for SeedPostings {
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    self._next_doc()?;
    Ok(self.doc_id)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.slow_advance(target)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.doc_freq as i64)
  }
}

impl PostingsEnum for SeedPostings {
  fn freq(&mut self) -> Result<i32> {
    Ok(self.freq)
  }

  fn next_position(&mut self) -> Result<i32> {
    if !self.do_positions {
      self.pos_upto = self.freq;
      return Ok(-1);
    }

    debug_assert!(self.pos_upto < self.freq);

    if self.pos_upto == 0 && self.random.random_bool(0.5) {
    } else if self.pos_spacing == 1 {
      self.pos += 1;
    } else {
      self.pos += TestUtil::next_int(&mut self.random, 1, self.pos_spacing);
    }

    if self.payload_size != 0 {
      if self.fixed_payloads {
        self.payload.length = self.payload_size as usize;
        self.random.fill_bytes(&mut self.payload.bytes);
      } else {
        let this_payload_size = self.random.random_range(0..self.payload_size);
        if this_payload_size != 0 {
          self.payload.length = self.payload_size as usize;
          self.random.fill_bytes(&mut self.payload.bytes);
        } else {
          self.payload.length = 0;
        }
      }
    } else {
      self.payload.length = 0;
    }

    if !self.allow_payloads {
      self.payload.length = 0;
    }

    self.start_offset = self.offset + self.random.random_range(0..5);
    self.end_offset = self.start_offset + self.random.random_range(0..10);
    self.offset = self.end_offset;

    self.pos_upto += 1;
    Ok(self.pos)
  }

  fn start_offset(&self) -> Result<i32> {
    Ok(self.start_offset)
  }

  fn end_offset(&self) -> Result<i32> {
    Ok(self.end_offset)
  }

  fn get_payload(&self) -> Result<std::option::Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    if self.payload.length == 0 {
      Ok(None)
    } else {
      Ok(Some(Cow::Borrowed(&self.payload)))
    }
  }
}
/// Holds one field, term and ord.
pub struct FieldAndTerm {
  field: String,
  term: BytesRef<Vec<u8>>,
  ord: i64,
}

impl FieldAndTerm {
  pub fn new(field: String, term: &BytesRef<Vec<u8>>, ord: i64) -> Self {
    Self {
      field,
      term: BytesRef::deep_copy_of(term),
      ord,
    }
  }
}

pub struct SeedTermsEnum {
  terms: Vec<(BytesRef<Vec<u8>>, SeedAndOrd)>,
  max_allowed: IndexOptions,
  allow_payloads: bool,
  next_index: usize,
  current_index: Option<usize>,
}
impl SeedTermsEnum {
  fn new(
    terms: BTreeMap<BytesRef<Vec<u8>>, SeedAndOrd>,
    max_allowed: IndexOptions,
    allow_payloads: bool,
  ) -> Self {
    Self {
      terms: terms.into_iter().collect(),
      max_allowed,
      allow_payloads,
      next_index: 0,
      current_index: None,
    }
  }

  fn reset(&mut self) {
    self.next_index = 0;
    self.current_index = None;
  }

  fn current_entry(&self) -> Result<&(BytesRef<Vec<u8>>, SeedAndOrd)> {
    let idx = self
      .current_index
      .ok_or_else(|| LuceneError::illegal_state("this terms enum is unpositioned"))?;
    Ok(&self.terms[idx])
  }

  fn seek_to_index(&mut self, index: usize) {
    self.current_index = Some(index);
    self.next_index = index + 1;
  }
}

impl BytesRefIterator for SeedTermsEnum {
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    if self.next_index >= self.terms.len() {
      self.current_index = None;
      return Ok(None);
    }
    self.current_index = Some(self.next_index);
    self.next_index += 1;
    Ok(Some(Cow::Borrowed(
      &self.terms[self.current_index.unwrap()].0,
    )))
  }
}

impl TermsEnum for SeedTermsEnum {
  type AttributeSource = DummyAttributeSource;

  fn attributes(&self) -> Result<Self::AttributeSource> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
    Ok(self.seek_ceil(term)? == SeekStatus::Found)
  }

  fn prepare_seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    Err(LuceneError::unsupported_operation(format!(
      "prepare_seek_exact({text})"
    )))
  }

  fn get_prepare_seek_exact_status(&mut self, target: &BytesRef<Vec<u8>>) -> Result<bool> {
    Err(LuceneError::unsupported_operation(format!(
      "get_prepare_seek_exact_status({target})"
    )))
  }

  fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    match self.terms.binary_search_by(|(t, _)| t.cmp(term)) {
      Ok(index) => {
        self.seek_to_index(index);
        Ok(SeekStatus::Found)
      },
      Err(index) => {
        if index >= self.terms.len() {
          self.current_index = None;
          self.next_index = self.terms.len();
          Ok(SeekStatus::End)
        } else {
          self.seek_to_index(index);
          Ok(SeekStatus::NotFound)
        }
      },
    }
  }

  fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
    let index = self
      .terms
      .iter()
      .position(|(_, seed_and_ord)| seed_and_ord.ord == ord)
      .ok_or_else(|| LuceneError::illegal_argument(format!("ord= {ord} does not exist")))?;
    self.seek_to_index(index);
    Ok(())
  }

  fn seek_exact_with_state(
    &mut self,
    term: &BytesRef<Vec<u8>>,
    state: &TermStateEnum,
  ) -> Result<()> {
    let _ = state;
    if !self.seek_exact(term)? {
      return Err(LuceneError::illegal_argument(format!(
        "term= {term} does not exist"
      )));
    }
    Ok(())
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    Ok(Cow::Borrowed(&self.current_entry()?.0))
  }

  fn ord(&self) -> Result<i64> {
    Ok(self.current_entry()?.1.ord)
  }

  fn doc_freq(&mut self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }

  type PostingsEnum = SeedPostings;

  fn postings_with_flags(
    &mut self,
    _reuse: Option<Self::PostingsEnum>,
    flags: i32,
  ) -> Result<Self::PostingsEnum> {
    if feature_requested(flags, POSITIONS) {
      if self
        .max_allowed
        .cmp(&IndexOptions::DocsAndFreqsAndPositions)
        .is_lt()
      {
        return Err(LuceneError::unsupported_operation(""));
      }
      if feature_requested(flags, OFFSETS)
        && self
          .max_allowed
          .cmp(&IndexOptions::DocsAndFreqsAndPositionsAndOffsets)
          .is_lt()
      {
        return Err(LuceneError::unsupported_operation(""));
      }
      if feature_requested(flags, PAYLOADS) && !self.allow_payloads {
        return Err(LuceneError::unsupported_operation(""));
      }
    }
    if feature_requested(flags, FREQS) && self.max_allowed.cmp(&IndexOptions::DocsAndFreqs).is_lt()
    {
      return Err(LuceneError::unsupported_operation(""));
    }

    let (term, seed_and_ord) = self.current_entry()?;
    Ok(get_seed_postings(
      &term.utf8_to_string()?,
      seed_and_ord.seed,
      self.max_allowed,
      self.allow_payloads,
    ))
  }

  type ImpactsEnum = DummyImpactsEnum;

  fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn term_state(&mut self) -> Result<TermStateEnum> {
    Ok(crate::core::index::base_terms_enum::BaseTermsEnumTermStateImpl.into())
  }
}

pub fn get_seed_postings(
  term: &str,
  seed: u64,
  options: IndexOptions,
  allow_payloads: bool,
) -> SeedPostings {
  let random_multiplier = random_multiplier();
  let (min_doc_freq, max_doc_freq) = if term.starts_with("big_") {
    (random_multiplier * 50000, random_multiplier * 70000)
  } else if term.starts_with("medium_") {
    (random_multiplier * 3000, random_multiplier * 6000)
  } else if term.starts_with("low_") {
    (random_multiplier, random_multiplier * 40)
  } else {
    (1, 3)
  };

  SeedPostings::new(seed, min_doc_freq, max_doc_freq, options, allow_payloads)
}

struct NormsProducerImpl {
  new_field_infos: Arc<FieldInfos>,
  max_doc: i32,
}
impl NormsProducerImpl {
  fn new(field_infos: Arc<FieldInfos>, max_doc: i32) -> Self {
    Self {
      new_field_infos: field_infos,
      max_doc,
    }
  }
}
impl NormsProducer for NormsProducerImpl {
  type NumericDocValues = NumericDocValuesImpl;

  fn get_norms(&self, field: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
    let field_info = self
      .new_field_infos
      .field_info_by_number(field.number)?
      .unwrap();
    assert!(field_info.has_norms());
    let field_infos = self.new_field_infos.clone();
    let max_doc = self.max_doc;
    Ok(NumericDocValuesImpl::new(field_infos, max_doc))
  }

  fn check_integrity(&self) -> Result<()> {
    Ok(())
  }
}
struct NumericDocValuesImpl {
  field_infos: Arc<FieldInfos>,
  max_doc: i32,
  doc: i32,
}
impl NumericDocValuesImpl {
  fn new(field_infos: Arc<FieldInfos>, max_doc: i32) -> Self {
    Self {
      field_infos,
      max_doc,
      doc: -1,
    }
  }
}

impl DocValuesIterator for NumericDocValuesImpl {
  fn advance_exact(&mut self, target: i32) -> Result<bool> {
    self.doc = target;
    Ok(true)
  }
}

impl DocIdSetIterator for NumericDocValuesImpl {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.doc += 1;
    if self.doc == self.max_doc {
      self.doc = NO_MORE_DOCS;
    }
    Ok(self.doc)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.doc = if target >= self.max_doc {
      NO_MORE_DOCS
    } else {
      target
    };
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.max_doc as i64)
  }
}

impl NumericDocValues for NumericDocValuesImpl {
  fn long_value(&mut self) -> Result<i64> {
    Ok(DocToNorm.apply_as_long(self.doc))
  }
}

trait IntToLongFunction {
  fn apply_as_long(self, i: i32) -> i64;
}
struct DocToNorm;
impl IntToLongFunction for DocToNorm {
  fn apply_as_long(self, doc: i32) -> i64 {
    ((1 + doc) & 0x0f) as i64
  }
}
struct FixedDocToNorm;
impl IntToLongFunction for FixedDocToNorm {
  fn apply_as_long(self, _doc: i32) -> i64 {
    1
  }
}
enum IntToLongFunctionEnum {
  DocToNorm(DocToNorm),
  FixedDocToNorm(FixedDocToNorm),
}
impl IntToLongFunctionEnum {
  fn apply_as_long(self, i: i32) -> i64 {
    match self {
      IntToLongFunctionEnum::DocToNorm(f) => f.apply_as_long(i),
      IntToLongFunctionEnum::FixedDocToNorm(f) => f.apply_as_long(i),
    }
  }
}

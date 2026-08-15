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
use crate::core::codecs::fields_producer::{FieldsProducer, FieldsProducerEnum2};
use crate::core::codecs::lucene101::lucene101_postings_format::Lucene101PostingsFormat;
use crate::core::codecs::postings_format::PostingsFormat;
use crate::core::index::BytesRef;
use crate::core::index::fields::Fields;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::Identity;
use crate::core::index::ord_term_state::OrdTermState;
use crate::core::index::postings_enum::{ALL, POSITIONS, PostingsEnum, feature_requested};
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::slow_impacts_enum::SlowImpactsEnum;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::store::directory::Directory;
use crate::core::store::io_context::Context;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::HasIdentity;
use crate::core::util::accountable::Accountable;
use crate::core::util::automation::compiled_automaton::{
  AutomatonEnum, AutomatonType, CompiledAutomaton,
};
use crate::core::util::automation::transition::Transition;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::close::CloseableRef;
use crate::core::util::dummy::dummy_attribute_source::DummyAttributeSource;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::iterator::{IteratorExt, VecIter, VecIteratorExt};
use crate::core::util::ram_usage_estimator::size_of_vec;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::mem::size_of_val;
use std::sync::{Arc, OnceLock};

// TODO:
//   - build depth-N prefix hash?
//   - or: longer dense skip lists than just next byte?

/// Wraps [`Lucene101PostingsFormat`] for on-disk storage, but then at read time loads and stores
/// all terms and postings directly in RAM as byte and integer arrays.
///
/// **WARNING**: This is exceptionally RAM intensive: it makes no effort to compress the postings
/// data, storing terms as separate byte arrays and postings as separate integer arrays, but as a
/// result it gives a substantial increase in search performance.
///
/// This postings format supports [`TermsEnum::ord`] and [`TermsEnum::seek_exact_with_ord`].
///
/// Because this holds all term bytes as a single byte array, a single segment cannot have more
/// than 2.1GB worth of term bytes.
///
/// # Experimental
pub struct DirectPostingsFormat {
  min_skip_count: i32,
  low_freq_cutoff: i32,
  identity: Identity,
}

const DEFAULT_MIN_SKIP_COUNT: i32 = 8;
const DEFAULT_LOW_FREQ_CUTOFF: i32 = 32;

// TODO: allow passing/wrapping arbitrary postings format?

impl Default for DirectPostingsFormat {
  fn default() -> Self {
    Self::new()
  }
}

impl DirectPostingsFormat {
  pub fn new() -> Self {
    Self::with_params(DEFAULT_MIN_SKIP_COUNT, DEFAULT_LOW_FREQ_CUTOFF)
  }

  /// `min_skip_count` is how many terms in a row must have the same prefix before a skip pointer
  /// is added. Terms with `doc_freq <= low_freq_cutoff` use one integer array to hold all docs,
  /// freqs, positions and offsets; higher-frequency terms use separate arrays.
  pub fn with_params(min_skip_count: i32, low_freq_cutoff: i32) -> Self {
    Self {
      min_skip_count,
      low_freq_cutoff,
      identity: Identity::new(),
    }
  }
}

impl HasIdentity for DirectPostingsFormat {
  fn identity(&self) -> &Identity {
    &self.identity
  }
}

impl PostingsFormat for DirectPostingsFormat {
  fn get_name(&self) -> &str {
    "Direct"
  }

  type FieldsConsumer<O: IndexOutput> =
    <Lucene101PostingsFormat as PostingsFormat>::FieldsConsumer<O>;

  fn fields_consumer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::FieldsConsumer<D1::IndexOutput>>
  where
    D1: Directory,
  {
    Lucene101PostingsFormat::new().fields_consumer(state, segment_info)
  }

  type FieldsProducer<I: IndexInput> = FieldsProducerEnum2<
    <Lucene101PostingsFormat as PostingsFormat>::FieldsProducer<I>,
    DirectFields,
  >;

  fn fields_producer<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::FieldsProducer<D1::IndexInput>>
  where
    D1: Directory,
  {
    let postings = Lucene101PostingsFormat::new().fields_producer(state, segment_info)?;
    if state.context.get_context() != &Context::Merge {
      let load_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        postings.check_integrity()?;
        DirectFields::new(state, &postings, self.min_skip_count, self.low_freq_cutoff)
      }));
      let close_result = postings.close();
      close_result?;
      unwrap_caught_result!(load_result).map(FieldsProducerEnum2::B)
    } else {
      // Don't load postings for merge:
      Ok(FieldsProducerEnum2::A(postings))
    }
  }

  fn for_name(name: &str) -> Result<Arc<Self>> {
    static FORMAT: OnceLock<Arc<DirectPostingsFormat>> = OnceLock::new();
    match name {
      "Direct" => Ok(Arc::clone(
        FORMAT.get_or_init(|| Arc::new(DirectPostingsFormat::new())),
      )),
      _ => Err(LuceneError::illegal_argument(format!(
        "Could not load postings format named \"{name}\""
      ))),
    }
  }
}

pub struct DirectFields {
  fields: BTreeMap<String, DirectField>,
  field_names: Vec<String>,
}

impl DirectFields {
  fn new<D, F>(
    state: &SegmentReadState<D>,
    fields: &F,
    min_skip_count: i32,
    low_freq_cutoff: i32,
  ) -> Result<Self>
  where
    D: Directory,
    F: Fields,
  {
    let mut direct_fields = BTreeMap::new();
    let mut iterator = fields.iterator()?;
    while let Some(field) = iterator.next()? {
      let terms = fields
        .terms(field)?
        .ok_or_else(|| LuceneError::illegal_state(format!("missing terms for field {field}")))?;
      direct_fields.insert(
        field.clone(),
        DirectField::new(state, field, &terms, min_skip_count, low_freq_cutoff)?,
      );
    }
    let field_names = direct_fields.keys().cloned().collect();
    Ok(Self {
      fields: direct_fields,
      field_names,
    })
  }
}

impl Fields for DirectFields {
  type FieldIter<'a>
    = VecIter<'a, String>
  where
    Self: 'a;

  type Terms = DirectField;

  fn iterator(&self) -> Result<Self::FieldIter<'_>> {
    Ok(self.field_names.iter_ext())
  }

  fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
    Ok(self.fields.get(field).cloned())
  }

  fn size(&self) -> Result<i32> {
    Ok(self.fields.len() as i32)
  }
}

impl CloseableRef for DirectFields {}

impl FieldsProducer for DirectFields {
  fn check_integrity(&self) -> Result<()> {
    // If we read entirely into RAM, we already validated.
    Ok(())
  }
}

impl Display for DirectFields {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "DirectFields(fields={})", self.fields.len())
  }
}

#[derive(Clone)]
pub struct DirectField {
  data: Arc<DirectFieldData>,
}

struct DirectFieldData {
  term_bytes: Vec<u8>,
  term_offsets: Vec<i32>,
  skips: Vec<i32>,
  skip_offsets: Vec<i32>,
  terms: Vec<Arc<TermAndSkip>>,
  has_freq: bool,
  has_pos: bool,
  has_offsets: bool,
  has_payloads: bool,
  sum_total_term_freq: i64,
  doc_count: i32,
  sum_doc_freq: i64,
  skip_count: i32,
  count: i32,
  same_counts: Vec<i32>,
  min_skip_count: i32,
}

enum TermAndSkip {
  LowFreq(LowFreqTerm),
  HighFreq(HighFreqTerm),
}

struct LowFreqTerm {
  skips: Vec<i32>,
  postings: Vec<i32>,
  payloads: Option<Vec<u8>>,
  doc_freq: i32,
  total_term_freq: i32,
}

struct HighFreqTerm {
  skips: Vec<i32>,
  total_term_freq: i64,
  doc_ids: Vec<i32>,
  freqs: Option<Vec<i32>>,
  positions: Option<Vec<Vec<i32>>>,
  payloads: Option<Vec<Vec<Option<Vec<u8>>>>>,
}

impl Accountable for LowFreqTerm {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(
      size_of_vec(&self.skips)
        + size_of_vec(&self.postings)
        + self.payloads.as_ref().map_or(0, size_of_vec),
    )
  }
}

impl Accountable for HighFreqTerm {
  fn ram_bytes_used(&self) -> Result<i64> {
    let mut size = size_of_vec(&self.skips) + size_of_vec(&self.doc_ids);
    if let Some(freqs) = &self.freqs {
      size += size_of_vec(freqs);
    }
    if let Some(positions) = &self.positions {
      size += size_of_vec(positions);
      for position in positions {
        size += size_of_vec(position);
      }
    }
    if let Some(payloads) = &self.payloads {
      size += size_of_vec(payloads);
      for payload in payloads {
        size += size_of_vec(payload);
        for bytes in payload.iter().flatten() {
          size += size_of_vec(bytes);
        }
      }
    }
    Ok(size)
  }
}

impl Accountable for TermAndSkip {
  fn ram_bytes_used(&self) -> Result<i64> {
    match self {
      Self::LowFreq(term) => term.ram_bytes_used(),
      Self::HighFreq(term) => term.ram_bytes_used(),
    }
  }
}

struct IntArrayWriter {
  ints: Vec<i32>,
}

impl IntArrayWriter {
  fn new() -> Self {
    Self {
      ints: Vec::with_capacity(10),
    }
  }

  fn add(&mut self, value: i32) {
    self.ints.push(value);
  }

  fn get(&mut self) -> Vec<i32> {
    let result = self.ints.clone();
    self.ints.clear();
    result
  }
}

impl DirectField {
  fn new<D, T>(
    state: &SegmentReadState<D>,
    field: &str,
    terms_in: &T,
    min_skip_count: i32,
    low_freq_cutoff: i32,
  ) -> Result<Self>
  where
    D: Directory,
    T: Terms,
  {
    let field_info = state
      .field_infos
      .field_info_by_name(field)?
      .ok_or_else(|| LuceneError::illegal_state(format!("missing FieldInfo for {field}")))?;

    let sum_total_term_freq = terms_in.get_sum_total_term_freq()?;
    let sum_doc_freq = terms_in.get_sum_doc_freq()?;
    let doc_count = terms_in.get_doc_count()?;

    let num_terms = terms_in.size()?;
    if num_terms == -1 {
      return Err(LuceneError::illegal_argument(
        "codec does not provide Terms.size()",
      ));
    }
    let num_terms = usize::try_from(num_terms)
      .map_err(|_| LuceneError::illegal_argument("invalid negative Terms.size()"))?;
    let mut terms = Vec::with_capacity(num_terms);
    let mut term_offsets = vec![0; 1 + num_terms];
    let mut term_bytes = Vec::with_capacity(1024);
    let mut same_counts = vec![0; 10];
    let mut skip_count = 0i32;

    let index_options = *field_info.get_index_options();
    let has_freq = index_options > IndexOptions::Docs;
    let has_pos = index_options > IndexOptions::DocsAndFreqs;
    let has_offsets = index_options > IndexOptions::DocsAndFreqsAndPositions;
    let has_payloads = field_info.has_payloads();

    let mut postings_enum = None;
    let mut docs_and_positions_enum = None;
    let mut terms_enum = terms_in.iterator()?;
    let mut term_offset = 0usize;
    let mut scratch = IntArrayWriter::new();
    let mut payload_output = Vec::new();
    let mut count = 0usize;

    while let Some(term) = terms_enum.next()?.map(Cow::into_owned) {
      let doc_freq = terms_enum.doc_freq()?;
      let total_term_freq = terms_enum.total_term_freq()?;

      term_offsets[count] = term_offset as i32;
      let term_slice = &term.bytes[term.offset..term.offset + term.length];
      term_bytes.extend_from_slice(term_slice);
      term_offset += term.length;
      term_offsets[count + 1] = term_offset as i32;

      if has_pos {
        docs_and_positions_enum =
          Some(terms_enum.postings_with_flags(docs_and_positions_enum.take(), ALL as i32)?);
      } else {
        postings_enum = Some(terms_enum.postings(postings_enum.take())?);
      }
      let postings_enum2 = if has_pos {
        docs_and_positions_enum.as_mut().unwrap()
      } else {
        postings_enum.as_mut().unwrap()
      };

      let entry = if doc_freq <= low_freq_cutoff {
        payload_output.clear();
        while postings_enum2.next_doc()? != NO_MORE_DOCS {
          scratch.add(postings_enum2.doc_id());
          if has_freq {
            let freq = postings_enum2.freq()?;
            scratch.add(freq);
            if has_pos {
              for _ in 0..freq {
                scratch.add(postings_enum2.next_position()?);
                if has_offsets {
                  scratch.add(postings_enum2.start_offset()?);
                  scratch.add(postings_enum2.end_offset()?);
                }
                if has_payloads {
                  if let Some(payload) = postings_enum2.get_payload()? {
                    scratch.add(payload.length as i32);
                    payload_output.extend_from_slice(
                      &payload.bytes[payload.offset..payload.offset + payload.length],
                    );
                  } else {
                    scratch.add(0);
                  }
                }
              }
            }
          }
        }
        let payloads = has_payloads.then(|| payload_output.clone());
        TermAndSkip::LowFreq(LowFreqTerm {
          skips: Vec::new(),
          postings: scratch.get(),
          payloads,
          doc_freq,
          total_term_freq: total_term_freq as i32,
        })
      } else {
        let mut docs = vec![0; doc_freq as usize];
        let mut freqs = has_freq.then(|| vec![0; doc_freq as usize]);
        let mut positions = has_pos.then(|| vec![Vec::new(); doc_freq as usize]);
        let mut payloads = has_payloads.then(|| vec![Vec::new(); doc_freq as usize]);

        let mut upto = 0usize;
        while postings_enum2.next_doc()? != NO_MORE_DOCS {
          docs[upto] = postings_enum2.doc_id();
          if has_freq {
            let freq = postings_enum2.freq()?;
            freqs.as_mut().unwrap()[upto] = freq;
            if has_pos {
              let mult = if has_offsets { 3 } else { 1 };
              let doc_positions = &mut positions.as_mut().unwrap()[upto];
              doc_positions.resize(mult * freq as usize, 0);
              if has_payloads {
                payloads.as_mut().unwrap()[upto] = vec![None; freq as usize];
              }
              let mut pos_upto = 0usize;
              for pos in 0..freq as usize {
                doc_positions[pos_upto] = postings_enum2.next_position()?;
                if has_payloads && let Some(payload) = postings_enum2.get_payload()? {
                  payloads.as_mut().unwrap()[upto][pos] =
                    Some(payload.bytes[payload.offset..payload.offset + payload.length].to_vec());
                }
                pos_upto += 1;
                if has_offsets {
                  doc_positions[pos_upto] = postings_enum2.start_offset()?;
                  pos_upto += 1;
                  doc_positions[pos_upto] = postings_enum2.end_offset()?;
                  pos_upto += 1;
                }
              }
            }
          }
          upto += 1;
        }
        debug_assert_eq!(upto, doc_freq as usize);
        TermAndSkip::HighFreq(HighFreqTerm {
          skips: Vec::new(),
          total_term_freq,
          doc_ids: docs,
          freqs,
          positions,
          payloads,
        })
      };

      terms.push(entry);
      Self::set_skips(
        count,
        &term_bytes,
        &term_offsets,
        &mut same_counts,
        min_skip_count,
        &mut terms,
        &mut skip_count,
      );
      count += 1;
    }

    term_offsets[count] = term_offset as i32;
    Self::finish_skips(
      count,
      &term_offsets,
      &same_counts,
      min_skip_count,
      &mut terms,
      &mut skip_count,
    );

    let mut skips = Vec::with_capacity(skip_count as usize);
    let mut skip_offsets = vec![0; 1 + num_terms];
    let mut skip_offset = 0usize;
    for (i, term) in terms.iter_mut().enumerate() {
      skip_offsets[i] = skip_offset as i32;
      let term_skips = match term {
        TermAndSkip::LowFreq(term) => std::mem::take(&mut term.skips),
        TermAndSkip::HighFreq(term) => std::mem::take(&mut term.skips),
      };
      skips.extend_from_slice(&term_skips);
      skip_offset += term_skips.len();
    }
    skip_offsets[num_terms] = skip_offset as i32;
    debug_assert_eq!(skip_offset, skip_count as usize);
    term_bytes.shrink_to_fit();

    Ok(Self {
      data: Arc::new(DirectFieldData {
        term_bytes,
        term_offsets,
        skips,
        skip_offsets,
        terms: terms.into_iter().map(Arc::new).collect(),
        has_freq,
        has_pos,
        has_offsets,
        has_payloads,
        sum_total_term_freq,
        doc_count,
        sum_doc_freq,
        skip_count,
        count: count as i32,
        same_counts,
        min_skip_count,
      }),
    })
  }

  // Compares in Unicode (UTF-8) order:
  fn compare(data: &DirectFieldData, ord: usize, other: &BytesRef<Vec<u8>>) -> i32 {
    let other_bytes = &other.bytes;
    let mut upto = data.term_offsets[ord] as usize;
    let term_len = data.term_offsets[ord + 1] as usize - upto;
    let mut other_upto = other.offset;
    let stop = upto + term_len.min(other.length);
    while upto < stop {
      let diff = data.term_bytes[upto] as i32 - other_bytes[other_upto] as i32;
      upto += 1;
      other_upto += 1;
      if diff != 0 {
        return diff;
      }
    }
    term_len as i32 - other.length as i32
  }

  fn set_skips(
    term_ord: usize,
    term_bytes: &[u8],
    term_offsets: &[i32],
    same_counts: &mut Vec<i32>,
    min_skip_count: i32,
    terms: &mut [TermAndSkip],
    skip_count: &mut i32,
  ) {
    let term_length = (term_offsets[term_ord + 1] - term_offsets[term_ord]) as usize;
    if same_counts.len() < term_length {
      same_counts.resize(term_length, 0);
    }
    if term_ord > 0 {
      let last_term_length = (term_offsets[term_ord] - term_offsets[term_ord - 1]) as usize;
      let limit = term_length.min(last_term_length);
      let mut last_term_offset = term_offsets[term_ord - 1] as usize;
      let mut term_offset = term_offsets[term_ord] as usize;
      let mut i = 0usize;
      while i < limit {
        if term_bytes[last_term_offset] == term_bytes[term_offset] {
          same_counts[i] += 1;
          last_term_offset += 1;
          term_offset += 1;
          i += 1;
        } else {
          while i < limit {
            if same_counts[i] >= min_skip_count {
              Self::save_skip(term_ord, same_counts[i], terms, skip_count);
            }
            same_counts[i] = 1;
            i += 1;
          }
          break;
        }
      }
      while i < last_term_length {
        if same_counts[i] >= min_skip_count {
          Self::save_skip(term_ord, same_counts[i], terms, skip_count);
        }
        same_counts[i] = 0;
        i += 1;
      }
      for value in same_counts.iter_mut().take(term_length).skip(limit) {
        *value = 1;
      }
    } else {
      for value in same_counts.iter_mut().take(term_length) {
        *value += 1;
      }
    }
  }

  fn finish_skips(
    count: usize,
    term_offsets: &[i32],
    same_counts: &[i32],
    min_skip_count: i32,
    terms: &mut [TermAndSkip],
    skip_count: &mut i32,
  ) {
    debug_assert_eq!(count, terms.len());
    if count == 0 {
      return;
    }
    let last_term_offset = term_offsets[count - 1];
    let last_term_length = (term_offsets[count] - last_term_offset) as usize;
    for (i, same_count) in same_counts.iter().enumerate().take(last_term_length) {
      if *same_count >= min_skip_count {
        let _ = i;
        Self::save_skip(count, *same_count, terms, skip_count);
      }
    }
    for term in terms {
      let skips = match term {
        TermAndSkip::LowFreq(term) => &mut term.skips,
        TermAndSkip::HighFreq(term) => &mut term.skips,
      };
      skips.reverse();
    }
  }

  fn save_skip(ord: usize, back_count: i32, terms: &mut [TermAndSkip], skip_count: &mut i32) {
    let term = &mut terms[ord - back_count as usize];
    *skip_count += 1;
    match term {
      TermAndSkip::LowFreq(term) => term.skips.push(ord as i32),
      TermAndSkip::HighFreq(term) => term.skips.push(ord as i32),
    }
  }
}

impl Accountable for DirectField {
  fn ram_bytes_used(&self) -> Result<i64> {
    let data = self.data.as_ref();
    let mut size = size_of_val(data) as i64
      + size_of_vec(&data.term_bytes)
      + size_of_vec(&data.term_offsets)
      + size_of_vec(&data.skips)
      + size_of_vec(&data.skip_offsets)
      + size_of_vec(&data.same_counts)
      + size_of_vec(&data.terms);
    for term in &data.terms {
      size += size_of_val(term.as_ref()) as i64 + term.ram_bytes_used()?;
    }
    Ok(size)
  }
}

impl Display for DirectField {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "DirectTerms(terms={},postings={},positions={},docs={})",
      self.data.terms.len(),
      self.data.sum_doc_freq,
      self.data.sum_total_term_freq,
      self.data.doc_count
    )
  }
}

impl Terms for DirectField {
  type TermsEnum = DirectTermsEnum;

  fn iterator(&self) -> Result<Self::TermsEnum> {
    Ok(DirectTermsEnum::new(Arc::clone(&self.data)))
  }

  type IntersectIter = DirectIntersectTermsEnum;

  fn intersect(
    &self,
    compiled: &CompiledAutomaton,
    start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<Self::IntersectIter> {
    if compiled.type_ != AutomatonType::Normal {
      return Err(LuceneError::illegal_argument(
        "please use CompiledAutomaton.get_terms_enum instead",
      ));
    }
    DirectIntersectTermsEnum::new(Arc::clone(&self.data), compiled, start_term)
  }

  fn size(&self) -> Result<i64> {
    Ok(self.data.terms.len() as i64)
  }

  fn get_sum_total_term_freq(&self) -> Result<i64> {
    Ok(self.data.sum_total_term_freq)
  }

  fn get_sum_doc_freq(&self) -> Result<i64> {
    Ok(self.data.sum_doc_freq)
  }

  fn get_doc_count(&self) -> Result<i32> {
    Ok(self.data.doc_count)
  }

  fn has_freqs(&self) -> bool {
    self.data.has_freq
  }

  fn has_offsets(&self) -> bool {
    self.data.has_offsets
  }

  fn has_positions(&self) -> bool {
    self.data.has_pos
  }

  fn has_payloads(&self) -> bool {
    self.data.has_payloads
  }
}

pub struct DirectTermsEnum {
  data: Arc<DirectFieldData>,
  scratch: BytesRef<Vec<u8>>,
  term_ord: i32,
}

impl DirectTermsEnum {
  fn new(data: Arc<DirectFieldData>) -> Self {
    Self {
      data,
      scratch: BytesRef::new(),
      term_ord: -1,
    }
  }

  fn set_term(&mut self) -> &BytesRef<Vec<u8>> {
    let start = self.data.term_offsets[self.term_ord as usize] as usize;
    let end = self.data.term_offsets[self.term_ord as usize + 1] as usize;
    self.scratch = BytesRef::from_bytes(self.data.term_bytes[start..end].to_vec());
    &self.scratch
  }

  // If non-negative, exact match; else, -ord-1, where ord is where the term would be inserted.
  fn find_term(&self, term: &BytesRef<Vec<u8>>) -> i32 {
    let mut low = 0i32;
    let mut high = self.data.terms.len() as i32 - 1;
    while low <= high {
      let mid = ((low + high) as u32 >> 1) as i32;
      let cmp = DirectField::compare(&self.data, mid as usize, term);
      if cmp < 0 {
        low = mid + 1;
      } else if cmp > 0 {
        high = mid - 1;
      } else {
        return mid;
      }
    }
    -(low + 1)
  }

  fn current_term(&self) -> Result<&Arc<TermAndSkip>> {
    self
      .data
      .terms
      .get(self.term_ord as usize)
      .ok_or_else(|| LuceneError::illegal_state("terms enum is not positioned"))
  }
}

impl BytesRefIterator for DirectTermsEnum {
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    self.term_ord += 1;
    if self.term_ord < self.data.terms.len() as i32 {
      self.set_term();
      Ok(Some(Cow::Borrowed(&self.scratch)))
    } else {
      Ok(None)
    }
  }
}

impl TermsEnum for DirectTermsEnum {
  type AttributeSource<'a>
    = &'a DummyAttributeSource
  where
    Self: 'a;
  type AttributeSourceMut<'a>
    = &'a mut DummyAttributeSource
  where
    Self: 'a;

  fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
    let ord = self.find_term(term);
    if ord >= 0 {
      self.term_ord = ord;
      self.set_term();
      Ok(true)
    } else {
      Ok(false)
    }
  }

  fn prepare_seek_exact(&mut self, _text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    Ok(Some(()))
  }

  fn get_prepare_seek_exact_status(&mut self, target: &BytesRef<Vec<u8>>) -> Result<bool> {
    self.seek_exact(target)
  }

  fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    let ord = self.find_term(term);
    if ord >= 0 {
      self.term_ord = ord;
      self.set_term();
      Ok(SeekStatus::Found)
    } else if ord == -(self.data.terms.len() as i32) - 1 {
      Ok(SeekStatus::End)
    } else {
      self.term_ord = -ord - 1;
      self.set_term();
      Ok(SeekStatus::NotFound)
    }
  }

  fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
    self.term_ord = ord as i32;
    self.set_term();
    Ok(())
  }

  fn seek_exact_with_state(
    &mut self,
    term: &BytesRef<Vec<u8>>,
    state: &TermStateEnum,
  ) -> Result<()> {
    self.term_ord = state.ord()? as i32;
    self.set_term();
    debug_assert!(self.scratch.bytes_equals(term));
    Ok(())
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    Ok(Cow::Borrowed(&self.scratch))
  }

  fn ord(&self) -> Result<i64> {
    Ok(self.term_ord as i64)
  }

  fn doc_freq(&mut self) -> Result<i32> {
    Ok(match self.current_term()?.as_ref() {
      TermAndSkip::LowFreq(term) => term.doc_freq,
      TermAndSkip::HighFreq(term) => term.doc_ids.len() as i32,
    })
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    Ok(match self.current_term()?.as_ref() {
      TermAndSkip::LowFreq(term) => term.total_term_freq as i64,
      TermAndSkip::HighFreq(term) => term.total_term_freq,
    })
  }

  type PostingsEnum = DirectPostingsEnum;

  fn postings_with_flags(
    &mut self,
    reuse: Option<Self::PostingsEnum>,
    flags: i32,
  ) -> Result<Self::PostingsEnum> {
    DirectPostingsEnum::for_term(
      Arc::clone(self.current_term()?),
      self.data.has_freq,
      self.data.has_pos,
      self.data.has_offsets,
      self.data.has_payloads,
      reuse,
      flags,
    )
  }

  type ImpactsEnum = SlowImpactsEnum<DirectPostingsEnum>;

  fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
    Ok(SlowImpactsEnum::new(self.postings_with_flags(None, flags)?))
  }

  fn term_state(&mut self) -> Result<TermStateEnum> {
    Ok(
      OrdTermState {
        ord: self.term_ord as i64,
      }
      .into(),
    )
  }
}

pub struct DirectIntersectTermsEnum {
  data: Arc<DirectFieldData>,
  automaton: AutomatonEnum,
  common_suffix_ref: Option<Arc<BytesRef<Vec<u8>>>>,
  term_ord: i32,
  scratch: BytesRef<Vec<u8>>,
  states: Vec<DirectIntersectState>,
  state_upto: usize,
}

struct DirectIntersectState {
  change_ord: i32,
  state: i32,
  transition_upto: i32,
  transition_count: i32,
  transition_max: i32,
  transition_min: i32,
  transition: Transition,
}

impl DirectIntersectState {
  fn new() -> Self {
    Self {
      change_ord: 0,
      state: 0,
      transition_upto: -1,
      transition_count: 0,
      transition_max: -1,
      transition_min: 0,
      transition: Transition::default(),
    }
  }
}

impl DirectIntersectTermsEnum {
  fn new(
    data: Arc<DirectFieldData>,
    compiled: &CompiledAutomaton,
    start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<Self> {
    let mut automaton = compiled.get_automaton()?;
    let mut first_state = DirectIntersectState::new();
    first_state.change_ord = data.terms.len() as i32;
    first_state.state = 0;
    first_state.transition_count =
      automaton.init_transition(first_state.state, &mut first_state.transition)?;
    let mut result = Self {
      data,
      automaton,
      common_suffix_ref: compiled.common_suffix_ref.clone(),
      term_ord: -1,
      scratch: BytesRef::new(),
      states: vec![first_state],
      state_upto: 0,
    };
    if let Some(start_term) = start_term {
      result.seek_to_start_term(start_term)?;
    }
    Ok(result)
  }

  fn grow(&mut self) {
    if self.states.len() == self.state_upto + 1 {
      self.states.push(DirectIntersectState::new());
    }
  }

  fn seek_to_start_term(&mut self, start_term: &BytesRef<Vec<u8>>) -> Result<()> {
    let mut skip_upto = 0usize;
    if start_term.length == 0 {
      if !self.data.terms.is_empty() && self.data.term_offsets[1] == 0 {
        self.term_ord = 0;
      }
    } else {
      self.term_ord += 1;

      'next_label: for i in 0..start_term.length {
        let label = start_term.bytes[start_term.offset + i] as i32;
        while label > self.states[i].transition_max {
          self.states[i].transition_upto += 1;
          if self.states[i].transition_upto >= self.states[i].transition_count {
            // All transitions compare less than the required label.
            break;
          }
          let (automaton, states) = (&mut self.automaton, &mut self.states);
          automaton.get_next_transition(&mut states[i].transition)?;
          states[i].transition_min = states[i].transition.min;
          states[i].transition_max = states[i].transition.max;
          debug_assert!((0..=255).contains(&states[i].transition_min));
          debug_assert!((0..=255).contains(&states[i].transition_max));
        }

        // Skip forwards until we find a term matching the label at this position.
        while self.term_ord < self.data.terms.len() as i32 {
          let term_ord = self.term_ord as usize;
          let skip_offset = self.data.skip_offsets[term_ord] as usize;
          let num_skips =
            (self.data.skip_offsets[term_ord + 1] - self.data.skip_offsets[term_ord]) as usize;
          let term_offset = self.data.term_offsets[term_ord] as usize;
          let term_length =
            (self.data.term_offsets[term_ord + 1] - self.data.term_offsets[term_ord]) as usize;

          if self.term_ord == self.states[self.state_upto].change_ord {
            self.state_upto -= 1;
            self.term_ord -= 1;
            return Ok(());
          }

          if term_length == i {
            self.term_ord += 1;
            skip_upto = 0;
          } else if label < self.data.term_bytes[term_offset + i] as i32 {
            self.term_ord -= 1;
            self.state_upto -= skip_upto;
            return Ok(());
          } else if label == self.data.term_bytes[term_offset + i] as i32 {
            if skip_upto < num_skips {
              self.grow();
              let next_state = self
                .automaton
                .step(self.states[self.state_upto].state, label)?;
              debug_assert_ne!(next_state, -1);

              self.state_upto += 1;
              let state = &mut self.states[self.state_upto];
              state.change_ord = self.data.skips[skip_offset + skip_upto];
              skip_upto += 1;
              state.state = next_state;
              state.transition_count = self
                .automaton
                .init_transition(next_state, &mut state.transition)?;
              state.transition_upto = -1;
              state.transition_max = -1;
              continue 'next_label;
            } else {
              // Index exhausted: just scan now (the number of scans required will be less than
              // min_skip_count).
              let start_term_ord = self.term_ord;
              while self.term_ord < self.data.terms.len() as i32
                && DirectField::compare(&self.data, self.term_ord as usize, start_term) <= 0
              {
                debug_assert!(
                  self.term_ord == start_term_ord
                    || self.data.skip_offsets[self.term_ord as usize]
                      == self.data.skip_offsets[self.term_ord as usize + 1]
                );
                self.term_ord += 1;
              }
              debug_assert!(self.term_ord - start_term_ord < self.data.min_skip_count);
              self.term_ord -= 1;
              self.state_upto -= skip_upto;
              return Ok(());
            }
          } else {
            if skip_upto < num_skips {
              self.term_ord = self.data.skips[skip_offset + skip_upto];
            } else {
              self.term_ord += 1;
            }
            skip_upto = 0;
          }
        }

        // startTerm is >= last term so this enum will not return any terms.
        self.term_ord -= 1;
        return Ok(());
      }
    }

    if self.term_ord >= 0 {
      let term_offset = self.data.term_offsets[self.term_ord as usize] as usize;
      let term_len = (self.data.term_offsets[self.term_ord as usize + 1]
        - self.data.term_offsets[self.term_ord as usize]) as usize;
      let same = start_term.length == term_len
        && start_term.bytes[start_term.offset..start_term.offset + start_term.length]
          == self.data.term_bytes[term_offset..term_offset + term_len];
      if !same {
        self.state_upto -= skip_upto;
        self.term_ord -= 1;
      }
    }
    Ok(())
  }

  fn set_term(&mut self) {
    let start = self.data.term_offsets[self.term_ord as usize] as usize;
    let end = self.data.term_offsets[self.term_ord as usize + 1] as usize;
    self.scratch = BytesRef::from_bytes(self.data.term_bytes[start..end].to_vec());
  }

  fn current_term(&self) -> Result<&Arc<TermAndSkip>> {
    self
      .data
      .terms
      .get(self.term_ord as usize)
      .ok_or_else(|| LuceneError::illegal_state("terms enum is not positioned"))
  }
}

impl BytesRefIterator for DirectIntersectTermsEnum {
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    self.term_ord += 1;
    let mut skip_upto = 0usize;

    if self.term_ord == 0 && self.data.term_offsets[1] == 0 {
      // Special-case empty string:
      debug_assert_eq!(self.state_upto, 0);
      if self.automaton.is_accept(self.states[0].state)? {
        self.scratch = BytesRef::new();
        return Ok(Some(Cow::Borrowed(&self.scratch)));
      }
      self.term_ord += 1;
    }

    'next_term: loop {
      if self.term_ord == self.data.terms.len() as i32 {
        return Ok(None);
      }

      if self.term_ord == self.states[self.state_upto].change_ord {
        // Pop:
        self.state_upto -= 1;
        continue;
      }

      let term_ord = self.term_ord as usize;
      let term_offset = self.data.term_offsets[term_ord] as usize;
      let term_length =
        (self.data.term_offsets[term_ord + 1] - self.data.term_offsets[term_ord]) as usize;
      let skip_offset = self.data.skip_offsets[term_ord] as usize;
      let num_skips =
        (self.data.skip_offsets[term_ord + 1] - self.data.skip_offsets[term_ord]) as usize;

      debug_assert!(self.term_ord < self.states[self.state_upto].change_ord);
      debug_assert!(self.state_upto <= term_length);
      let label = self.data.term_bytes[term_offset + self.state_upto] as i32;

      while label > self.states[self.state_upto].transition_max {
        self.states[self.state_upto].transition_upto += 1;
        if self.states[self.state_upto].transition_upto
          == self.states[self.state_upto].transition_count
        {
          // We've exhausted transitions leaving this state; force pop+next/skip now.
          if self.state_upto == 0 {
            self.term_ord = self.data.terms.len() as i32;
            return Ok(None);
          }
          debug_assert!(self.states[self.state_upto].change_ord > self.term_ord);
          self.term_ord = self.states[self.state_upto].change_ord;
          skip_upto = 0;
          self.state_upto -= 1;
          continue 'next_term;
        }
        let state_upto = self.state_upto;
        let (automaton, states) = (&mut self.automaton, &mut self.states);
        automaton.get_next_transition(&mut states[state_upto].transition)?;
        let state = &mut states[state_upto];
        state.transition_min = state.transition.min;
        state.transition_max = state.transition.max;
        debug_assert!((0..=255).contains(&state.transition_min));
        debug_assert!((0..=255).contains(&state.transition_max));
      }

      let target_label = self.states[self.state_upto].transition_min;
      if label < target_label {
        let mut low = self.term_ord + 1;
        let mut high = self.states[self.state_upto].change_ord - 1;
        loop {
          if low > high {
            // Label not found.
            self.term_ord = low;
            skip_upto = 0;
            continue 'next_term;
          }
          let mut mid = ((low + high) as u32 >> 1) as i32;
          let cmp = self.data.term_bytes
            [self.data.term_offsets[mid as usize] as usize + self.state_upto]
            as i32
            - target_label;
          if cmp < 0 {
            low = mid + 1;
          } else if cmp > 0 {
            high = mid - 1;
          } else {
            // Label found; walk backwards to the first occurrence.
            while mid > self.term_ord
              && self.data.term_bytes
                [self.data.term_offsets[mid as usize - 1] as usize + self.state_upto]
                as i32
                == target_label
            {
              mid -= 1;
            }
            self.term_ord = mid;
            skip_upto = 0;
            continue 'next_term;
          }
        }
      }

      let mut next_state = self
        .automaton
        .step(self.states[self.state_upto].state, label)?;
      if next_state == -1 {
        // Skip.
        if skip_upto < num_skips {
          self.term_ord = self.data.skips[skip_offset + skip_upto];
        } else {
          self.term_ord += 1;
        }
        skip_upto = 0;
      } else if skip_upto < num_skips {
        // Push:
        self.grow();
        self.state_upto += 1;
        let state = &mut self.states[self.state_upto];
        state.state = next_state;
        state.change_ord = self.data.skips[skip_offset + skip_upto];
        skip_upto += 1;
        state.transition_count = self
          .automaton
          .init_transition(next_state, &mut state.transition)?;
        state.transition_upto = -1;
        state.transition_max = -1;

        if self.state_upto == term_length {
          if self.automaton.is_accept(next_state)? {
            self.set_term();
            return Ok(Some(Cow::Borrowed(&self.scratch)));
          }
          self.term_ord += 1;
          skip_upto = 0;
        }
      } else {
        // Run the non-indexed tail of this term.
        if let Some(common_suffix_ref) = &self.common_suffix_ref {
          debug_assert_eq!(common_suffix_ref.offset, 0);
          if term_length < common_suffix_ref.length {
            self.term_ord += 1;
            skip_upto = 0;
            continue 'next_term;
          }
          let offset = term_offset + term_length - common_suffix_ref.length;
          if self.data.term_bytes[offset..offset + common_suffix_ref.length]
            != common_suffix_ref.bytes[..common_suffix_ref.length]
          {
            self.term_ord += 1;
            skip_upto = 0;
            continue 'next_term;
          }
        }

        let mut upto = self.state_upto + 1;
        while upto < term_length {
          next_state = self
            .automaton
            .step(next_state, self.data.term_bytes[term_offset + upto] as i32)?;
          if next_state == -1 {
            self.term_ord += 1;
            skip_upto = 0;
            continue 'next_term;
          }
          upto += 1;
        }

        if self.automaton.is_accept(next_state)? {
          self.set_term();
          return Ok(Some(Cow::Borrowed(&self.scratch)));
        }
        self.term_ord += 1;
        skip_upto = 0;
      }
    }
  }
}

impl TermsEnum for DirectIntersectTermsEnum {
  type AttributeSource<'a>
    = &'a DummyAttributeSource
  where
    Self: 'a;
  type AttributeSourceMut<'a>
    = &'a mut DummyAttributeSource
  where
    Self: 'a;

  fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_exact(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<bool> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn prepare_seek_exact(&mut self, _text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    Ok(Some(()))
  }

  fn get_prepare_seek_exact_status(&mut self, target: &BytesRef<Vec<u8>>) -> Result<bool> {
    self.seek_exact(target)
  }

  fn seek_ceil(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_exact_with_state(
    &mut self,
    term: &BytesRef<Vec<u8>>,
    _state: &TermStateEnum,
  ) -> Result<()> {
    if !self.seek_exact(term)? {
      return Err(LuceneError::illegal_argument(format!(
        "term= {term} does not exist"
      )));
    }
    Ok(())
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    Ok(Cow::Borrowed(&self.scratch))
  }

  fn ord(&self) -> Result<i64> {
    Ok(self.term_ord as i64)
  }

  fn doc_freq(&mut self) -> Result<i32> {
    Ok(match self.current_term()?.as_ref() {
      TermAndSkip::LowFreq(term) => term.doc_freq,
      TermAndSkip::HighFreq(term) => term.doc_ids.len() as i32,
    })
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    Ok(match self.current_term()?.as_ref() {
      TermAndSkip::LowFreq(term) => term.total_term_freq as i64,
      TermAndSkip::HighFreq(term) => term.total_term_freq,
    })
  }

  type PostingsEnum = DirectPostingsEnum;

  fn postings_with_flags(
    &mut self,
    reuse: Option<Self::PostingsEnum>,
    flags: i32,
  ) -> Result<Self::PostingsEnum> {
    DirectPostingsEnum::for_term(
      Arc::clone(self.current_term()?),
      self.data.has_freq,
      self.data.has_pos,
      self.data.has_offsets,
      self.data.has_payloads,
      reuse,
      flags,
    )
  }

  type ImpactsEnum = SlowImpactsEnum<DirectPostingsEnum>;

  fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
    Ok(SlowImpactsEnum::new(self.postings_with_flags(None, flags)?))
  }

  fn term_state(&mut self) -> Result<TermStateEnum> {
    Ok(
      OrdTermState {
        ord: self.term_ord as i64,
      }
      .into(),
    )
  }
}

pub enum DirectPostingsEnum {
  LowFreqDocsNoTf(LowFreqDocsEnumNoTf),
  LowFreqDocsNoPos(LowFreqDocsEnumNoPos),
  LowFreqDocs(LowFreqDocsEnum),
  LowFreqPostings(LowFreqPostingsEnum),
  HighFreqDocs(HighFreqDocsEnum),
  HighFreqPostings(HighFreqPostingsEnum),
}

impl DirectPostingsEnum {
  fn for_term(
    term: Arc<TermAndSkip>,
    has_freq: bool,
    has_pos: bool,
    has_offsets: bool,
    has_payloads: bool,
    reuse: Option<Self>,
    flags: i32,
  ) -> Result<Self> {
    if feature_requested(flags, POSITIONS) {
      return Ok(match term.as_ref() {
        TermAndSkip::LowFreq(_) if !has_freq => {
          let mut docs_enum = match reuse {
            Some(Self::LowFreqDocsNoTf(value)) => value,
            _ => LowFreqDocsEnumNoTf::new(),
          };
          docs_enum.reset(term);
          Self::LowFreqDocsNoTf(docs_enum)
        },
        TermAndSkip::LowFreq(_) if !has_pos => {
          let mut docs_enum = match reuse {
            Some(Self::LowFreqDocsNoPos(value)) => value,
            _ => LowFreqDocsEnumNoPos::new(),
          };
          docs_enum.reset(term);
          Self::LowFreqDocsNoPos(docs_enum)
        },
        TermAndSkip::LowFreq(_) => {
          let mut postings_enum = LowFreqPostingsEnum::new(has_offsets, has_payloads);
          postings_enum.reset(term);
          Self::LowFreqPostings(postings_enum)
        },
        TermAndSkip::HighFreq(_) if !has_pos => {
          let mut docs_enum = HighFreqDocsEnum::new();
          docs_enum.reset(term);
          Self::HighFreqDocs(docs_enum)
        },
        TermAndSkip::HighFreq(_) => {
          let mut postings_enum = HighFreqPostingsEnum::new(has_offsets);
          postings_enum.reset(term);
          Self::HighFreqPostings(postings_enum)
        },
      });
    }

    Ok(match term.as_ref() {
      TermAndSkip::LowFreq(_) if has_freq && has_pos => {
        let mut pos_len = if has_offsets { 3 } else { 1 };
        if has_payloads {
          pos_len += 1;
        }
        let mut docs_enum = match reuse {
          Some(Self::LowFreqDocs(value)) if value.can_reuse(pos_len) => value,
          _ => LowFreqDocsEnum::new(pos_len),
        };
        docs_enum.reset(term);
        Self::LowFreqDocs(docs_enum)
      },
      TermAndSkip::LowFreq(_) if has_freq => {
        let mut docs_enum = match reuse {
          Some(Self::LowFreqDocsNoPos(value)) => value,
          _ => LowFreqDocsEnumNoPos::new(),
        };
        docs_enum.reset(term);
        Self::LowFreqDocsNoPos(docs_enum)
      },
      TermAndSkip::LowFreq(_) => {
        let mut docs_enum = match reuse {
          Some(Self::LowFreqDocsNoTf(value)) => value,
          _ => LowFreqDocsEnumNoTf::new(),
        };
        docs_enum.reset(term);
        Self::LowFreqDocsNoTf(docs_enum)
      },
      TermAndSkip::HighFreq(_) => {
        let mut docs_enum = match reuse {
          Some(Self::HighFreqDocs(value)) => value,
          _ => HighFreqDocsEnum::new(),
        };
        docs_enum.reset(term);
        Self::HighFreqDocs(docs_enum)
      },
    })
  }
}

impl DocIdSetIterator for DirectPostingsEnum {
  fn doc_id(&self) -> i32 {
    match self {
      Self::LowFreqDocsNoTf(value) => value.doc_id(),
      Self::LowFreqDocsNoPos(value) => value.doc_id(),
      Self::LowFreqDocs(value) => value.doc_id(),
      Self::LowFreqPostings(value) => value.doc_id(),
      Self::HighFreqDocs(value) => value.doc_id(),
      Self::HighFreqPostings(value) => value.doc_id(),
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    match self {
      Self::LowFreqDocsNoTf(value) => value.next_doc(),
      Self::LowFreqDocsNoPos(value) => value.next_doc(),
      Self::LowFreqDocs(value) => value.next_doc(),
      Self::LowFreqPostings(value) => value.next_doc(),
      Self::HighFreqDocs(value) => value.next_doc(),
      Self::HighFreqPostings(value) => value.next_doc(),
    }
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    match self {
      Self::LowFreqDocsNoTf(value) => value.advance(target),
      Self::LowFreqDocsNoPos(value) => value.advance(target),
      Self::LowFreqDocs(value) => value.advance(target),
      Self::LowFreqPostings(value) => value.advance(target),
      Self::HighFreqDocs(value) => value.advance(target),
      Self::HighFreqPostings(value) => value.advance(target),
    }
  }

  fn cost(&self) -> Result<i64> {
    match self {
      Self::LowFreqDocsNoTf(value) => value.cost(),
      Self::LowFreqDocsNoPos(value) => value.cost(),
      Self::LowFreqDocs(value) => value.cost(),
      Self::LowFreqPostings(value) => value.cost(),
      Self::HighFreqDocs(value) => value.cost(),
      Self::HighFreqPostings(value) => value.cost(),
    }
  }
}

impl PostingsEnum for DirectPostingsEnum {
  fn freq(&mut self) -> Result<i32> {
    match self {
      Self::LowFreqDocsNoTf(value) => value.freq(),
      Self::LowFreqDocsNoPos(value) => value.freq(),
      Self::LowFreqDocs(value) => value.freq(),
      Self::LowFreqPostings(value) => value.freq(),
      Self::HighFreqDocs(value) => value.freq(),
      Self::HighFreqPostings(value) => value.freq(),
    }
  }

  fn next_position(&mut self) -> Result<i32> {
    match self {
      Self::LowFreqDocsNoTf(value) => value.next_position(),
      Self::LowFreqDocsNoPos(value) => value.next_position(),
      Self::LowFreqDocs(value) => value.next_position(),
      Self::LowFreqPostings(value) => value.next_position(),
      Self::HighFreqDocs(value) => value.next_position(),
      Self::HighFreqPostings(value) => value.next_position(),
    }
  }

  fn start_offset(&self) -> Result<i32> {
    match self {
      Self::LowFreqDocsNoTf(value) => value.start_offset(),
      Self::LowFreqDocsNoPos(value) => value.start_offset(),
      Self::LowFreqDocs(value) => value.start_offset(),
      Self::LowFreqPostings(value) => value.start_offset(),
      Self::HighFreqDocs(value) => value.start_offset(),
      Self::HighFreqPostings(value) => value.start_offset(),
    }
  }

  fn end_offset(&self) -> Result<i32> {
    match self {
      Self::LowFreqDocsNoTf(value) => value.end_offset(),
      Self::LowFreqDocsNoPos(value) => value.end_offset(),
      Self::LowFreqDocs(value) => value.end_offset(),
      Self::LowFreqPostings(value) => value.end_offset(),
      Self::HighFreqDocs(value) => value.end_offset(),
      Self::HighFreqPostings(value) => value.end_offset(),
    }
  }

  fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    match self {
      Self::LowFreqDocsNoTf(value) => value.get_payload(),
      Self::LowFreqDocsNoPos(value) => value.get_payload(),
      Self::LowFreqDocs(value) => value.get_payload(),
      Self::LowFreqPostings(value) => value.get_payload(),
      Self::HighFreqDocs(value) => value.get_payload(),
      Self::HighFreqPostings(value) => value.get_payload(),
    }
  }
}

// Docs only:
pub struct LowFreqDocsEnumNoTf {
  term: Option<Arc<TermAndSkip>>,
  upto: i32,
}

impl LowFreqDocsEnumNoTf {
  fn new() -> Self {
    Self {
      term: None,
      upto: -1,
    }
  }

  fn reset(&mut self, term: Arc<TermAndSkip>) {
    self.term = Some(term);
    self.upto = -1;
  }

  fn postings(&self) -> &[i32] {
    match self.term.as_ref().unwrap().as_ref() {
      TermAndSkip::LowFreq(term) => &term.postings,
      _ => unreachable!(),
    }
  }
}

impl DocIdSetIterator for LowFreqDocsEnumNoTf {
  fn doc_id(&self) -> i32 {
    if self.upto < 0 {
      -1
    } else if (self.upto as usize) < self.postings().len() {
      self.postings()[self.upto as usize]
    } else {
      NO_MORE_DOCS
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.upto += 1;
    Ok(self.doc_id())
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.slow_advance(target)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.postings().len() as i64)
  }
}

impl PostingsEnum for LowFreqDocsEnumNoTf {
  fn freq(&mut self) -> Result<i32> {
    Ok(1)
  }

  fn next_position(&mut self) -> Result<i32> {
    Ok(-1)
  }

  fn start_offset(&self) -> Result<i32> {
    Ok(-1)
  }

  fn end_offset(&self) -> Result<i32> {
    Ok(-1)
  }

  fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(None)
  }
}

// Docs + freqs:
pub struct LowFreqDocsEnumNoPos {
  term: Option<Arc<TermAndSkip>>,
  upto: i32,
}

impl LowFreqDocsEnumNoPos {
  fn new() -> Self {
    Self {
      term: None,
      upto: -2,
    }
  }

  fn reset(&mut self, term: Arc<TermAndSkip>) {
    self.term = Some(term);
    self.upto = -2;
  }

  fn postings(&self) -> &[i32] {
    match self.term.as_ref().unwrap().as_ref() {
      TermAndSkip::LowFreq(term) => &term.postings,
      _ => unreachable!(),
    }
  }
}

impl DocIdSetIterator for LowFreqDocsEnumNoPos {
  fn doc_id(&self) -> i32 {
    if self.upto < 0 {
      -1
    } else if (self.upto as usize) < self.postings().len() {
      self.postings()[self.upto as usize]
    } else {
      NO_MORE_DOCS
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.upto += 2;
    Ok(self.doc_id())
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.slow_advance(target)
  }

  fn cost(&self) -> Result<i64> {
    Ok((self.postings().len() / 2) as i64)
  }
}

impl PostingsEnum for LowFreqDocsEnumNoPos {
  fn freq(&mut self) -> Result<i32> {
    Ok(self.postings()[self.upto as usize + 1])
  }

  fn next_position(&mut self) -> Result<i32> {
    Ok(-1)
  }

  fn start_offset(&self) -> Result<i32> {
    Ok(-1)
  }

  fn end_offset(&self) -> Result<i32> {
    Ok(-1)
  }

  fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(None)
  }
}

// Docs + freqs + positions/offsets:
pub struct LowFreqDocsEnum {
  term: Option<Arc<TermAndSkip>>,
  pos_mult: i32,
  upto: i32,
  freq: i32,
}

impl LowFreqDocsEnum {
  fn new(pos_mult: i32) -> Self {
    Self {
      term: None,
      pos_mult,
      upto: -2,
      freq: 0,
    }
  }

  fn can_reuse(&self, pos_mult: i32) -> bool {
    self.pos_mult == pos_mult
  }

  fn reset(&mut self, term: Arc<TermAndSkip>) {
    self.term = Some(term);
    self.upto = -2;
    self.freq = 0;
  }

  fn postings(&self) -> &[i32] {
    match self.term.as_ref().unwrap().as_ref() {
      TermAndSkip::LowFreq(term) => &term.postings,
      _ => unreachable!(),
    }
  }
}

impl DocIdSetIterator for LowFreqDocsEnum {
  fn doc_id(&self) -> i32 {
    if self.upto < 0 {
      -1
    } else if (self.upto as usize) < self.postings().len() {
      self.postings()[self.upto as usize]
    } else {
      NO_MORE_DOCS
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.upto += 2 + self.freq * self.pos_mult;
    if (self.upto as usize) < self.postings().len() {
      self.freq = self.postings()[self.upto as usize + 1];
      debug_assert!(self.freq > 0);
    }
    Ok(self.doc_id())
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.slow_advance(target)
  }

  fn cost(&self) -> Result<i64> {
    Ok((self.postings().len() / 2) as i64)
  }
}

impl PostingsEnum for LowFreqDocsEnum {
  fn freq(&mut self) -> Result<i32> {
    Ok(self.freq)
  }

  fn next_position(&mut self) -> Result<i32> {
    Ok(-1)
  }

  fn start_offset(&self) -> Result<i32> {
    Ok(-1)
  }

  fn end_offset(&self) -> Result<i32> {
    Ok(-1)
  }

  fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(None)
  }
}

pub struct LowFreqPostingsEnum {
  term: Option<Arc<TermAndSkip>>,
  pos_mult: i32,
  has_offsets: bool,
  has_payloads: bool,
  payload: Option<BytesRef<Vec<u8>>>,
  upto: usize,
  doc_id: i32,
  freq: i32,
  skip_positions: i32,
  pos: i32,
  start_offset: i32,
  end_offset: i32,
  payload_offset: usize,
  payload_length: usize,
}

impl LowFreqPostingsEnum {
  fn new(has_offsets: bool, has_payloads: bool) -> Self {
    let pos_mult = if has_offsets {
      if has_payloads { 4 } else { 3 }
    } else if has_payloads {
      2
    } else {
      1
    };
    Self {
      term: None,
      pos_mult,
      has_offsets,
      has_payloads,
      payload: None,
      upto: 0,
      doc_id: -1,
      freq: 0,
      skip_positions: 0,
      pos: -1,
      start_offset: -1,
      end_offset: -1,
      payload_offset: 0,
      payload_length: 0,
    }
  }

  fn reset(&mut self, term: Arc<TermAndSkip>) {
    self.term = Some(term);
    self.upto = 0;
    self.skip_positions = 0;
    self.pos = -1;
    self.start_offset = -1;
    self.end_offset = -1;
    self.doc_id = -1;
    self.payload_offset = 0;
    self.payload_length = 0;
    self.payload = None;
  }

  fn low_term(&self) -> &LowFreqTerm {
    match self.term.as_ref().unwrap().as_ref() {
      TermAndSkip::LowFreq(term) => term,
      _ => unreachable!(),
    }
  }
}

impl DocIdSetIterator for LowFreqPostingsEnum {
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.pos = -1;
    if self.has_payloads {
      for _ in 0..self.skip_positions {
        self.upto += 1;
        if self.has_offsets {
          self.upto += 2;
        }
        let length = self.low_term().postings[self.upto] as usize;
        self.upto += 1;
        self.payload_offset += length;
      }
    } else {
      self.upto += self.pos_mult as usize * self.skip_positions as usize;
    }

    if self.upto < self.low_term().postings.len() {
      self.doc_id = self.low_term().postings[self.upto];
      self.upto += 1;
      self.freq = self.low_term().postings[self.upto];
      self.upto += 1;
      self.skip_positions = self.freq;
      Ok(self.doc_id)
    } else {
      self.doc_id = NO_MORE_DOCS;
      Ok(self.doc_id)
    }
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.slow_advance(target)
  }

  fn cost(&self) -> Result<i64> {
    Ok((self.low_term().postings.len() / 2) as i64)
  }
}

impl PostingsEnum for LowFreqPostingsEnum {
  fn freq(&mut self) -> Result<i32> {
    Ok(self.freq)
  }

  fn next_position(&mut self) -> Result<i32> {
    debug_assert!(self.skip_positions > 0);
    self.skip_positions -= 1;
    self.pos = self.low_term().postings[self.upto];
    self.upto += 1;
    if self.has_offsets {
      self.start_offset = self.low_term().postings[self.upto];
      self.upto += 1;
      self.end_offset = self.low_term().postings[self.upto];
      self.upto += 1;
    }
    if self.has_payloads {
      self.payload_length = self.low_term().postings[self.upto] as usize;
      self.upto += 1;
      if self.payload_length > 0 {
        let payloads = self.low_term().payloads.as_ref().unwrap();
        self.payload = Some(BytesRef::from_bytes(
          payloads[self.payload_offset..self.payload_offset + self.payload_length].to_vec(),
        ));
      } else {
        self.payload = None;
      }
      self.payload_offset += self.payload_length;
    }
    Ok(self.pos)
  }

  fn start_offset(&self) -> Result<i32> {
    Ok(self.start_offset)
  }

  fn end_offset(&self) -> Result<i32> {
    Ok(self.end_offset)
  }

  fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(self.payload.as_ref().map(Cow::Borrowed))
  }
}

// Docs + freqs:
pub struct HighFreqDocsEnum {
  term: Option<Arc<TermAndSkip>>,
  upto: i32,
  doc_id: i32,
}

impl HighFreqDocsEnum {
  fn new() -> Self {
    Self {
      term: None,
      upto: -1,
      doc_id: -1,
    }
  }

  fn reset(&mut self, term: Arc<TermAndSkip>) {
    self.term = Some(term);
    self.doc_id = -1;
    self.upto = -1;
  }

  fn high_term(&self) -> &HighFreqTerm {
    match self.term.as_ref().unwrap().as_ref() {
      TermAndSkip::HighFreq(term) => term,
      _ => unreachable!(),
    }
  }
}

impl DocIdSetIterator for HighFreqDocsEnum {
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.upto += 1;
    if (self.upto as usize) < self.high_term().doc_ids.len() {
      self.doc_id = self.high_term().doc_ids[self.upto as usize];
    } else {
      self.doc_id = NO_MORE_DOCS;
    }
    Ok(self.doc_id)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.upto += 1;
    let length = self.high_term().doc_ids.len() as i32;
    if self.upto == length {
      self.doc_id = NO_MORE_DOCS;
      return Ok(self.doc_id);
    }

    let mut inc = 10;
    let mut next_upto = self.upto + 10;
    let (mut low, mut high);
    loop {
      if next_upto >= length {
        low = next_upto - inc;
        high = length - 1;
        break;
      }
      if target <= self.high_term().doc_ids[next_upto as usize] {
        low = next_upto - inc;
        high = next_upto;
        break;
      }
      inc *= 2;
      next_upto += inc;
    }
    loop {
      if low > high {
        self.upto = low;
        break;
      }
      let mid = ((low + high) as u32 >> 1) as i32;
      let cmp = self.high_term().doc_ids[mid as usize] - target;
      if cmp < 0 {
        low = mid + 1;
      } else if cmp > 0 {
        high = mid - 1;
      } else {
        self.upto = mid;
        break;
      }
    }
    if self.upto == length {
      self.doc_id = NO_MORE_DOCS;
    } else {
      self.doc_id = self.high_term().doc_ids[self.upto as usize];
    }
    Ok(self.doc_id)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.high_term().doc_ids.len() as i64)
  }
}

impl PostingsEnum for HighFreqDocsEnum {
  fn freq(&mut self) -> Result<i32> {
    Ok(
      self
        .high_term()
        .freqs
        .as_ref()
        .map_or(1, |freqs| freqs[self.upto as usize]),
    )
  }

  fn next_position(&mut self) -> Result<i32> {
    Ok(-1)
  }

  fn start_offset(&self) -> Result<i32> {
    Ok(-1)
  }

  fn end_offset(&self) -> Result<i32> {
    Ok(-1)
  }

  fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(None)
  }
}

// TODO: specialize offsets and not.
pub struct HighFreqPostingsEnum {
  term: Option<Arc<TermAndSkip>>,
  has_offsets: bool,
  pos_jump: i32,
  upto: i32,
  doc_id: i32,
  pos_upto: i32,
  payload: Option<BytesRef<Vec<u8>>>,
}

impl HighFreqPostingsEnum {
  fn new(has_offsets: bool) -> Self {
    Self {
      term: None,
      has_offsets,
      pos_jump: if has_offsets { 3 } else { 1 },
      upto: -1,
      doc_id: -1,
      pos_upto: 0,
      payload: None,
    }
  }

  fn reset(&mut self, term: Arc<TermAndSkip>) {
    self.term = Some(term);
    self.upto = -1;
    self.payload = None;
  }

  fn high_term(&self) -> &HighFreqTerm {
    match self.term.as_ref().unwrap().as_ref() {
      TermAndSkip::HighFreq(term) => term,
      _ => unreachable!(),
    }
  }
}

impl DocIdSetIterator for HighFreqPostingsEnum {
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.upto += 1;
    if (self.upto as usize) < self.high_term().doc_ids.len() {
      self.pos_upto = -self.pos_jump;
      self.doc_id = self.high_term().doc_ids[self.upto as usize];
    } else {
      self.doc_id = NO_MORE_DOCS;
    }
    Ok(self.doc_id)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.upto += 1;
    let length = self.high_term().doc_ids.len() as i32;
    if self.upto == length {
      self.doc_id = NO_MORE_DOCS;
      return Ok(self.doc_id);
    }

    let mut inc = 10;
    let mut next_upto = self.upto + 10;
    let (mut low, mut high);
    loop {
      if next_upto >= length {
        low = next_upto - inc;
        high = length - 1;
        break;
      }
      if target <= self.high_term().doc_ids[next_upto as usize] {
        low = next_upto - inc;
        high = next_upto;
        break;
      }
      inc *= 2;
      next_upto += inc;
    }
    loop {
      if low > high {
        self.upto = low;
        break;
      }
      let mid = ((low + high) as u32 >> 1) as i32;
      let cmp = self.high_term().doc_ids[mid as usize] - target;
      if cmp < 0 {
        low = mid + 1;
      } else if cmp > 0 {
        high = mid - 1;
      } else {
        self.upto = mid;
        break;
      }
    }
    if self.upto == length {
      self.doc_id = NO_MORE_DOCS;
    } else {
      self.pos_upto = -self.pos_jump;
      self.doc_id = self.high_term().doc_ids[self.upto as usize];
    }
    Ok(self.doc_id)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.high_term().doc_ids.len() as i64)
  }
}

impl PostingsEnum for HighFreqPostingsEnum {
  fn freq(&mut self) -> Result<i32> {
    Ok(self.high_term().freqs.as_ref().unwrap()[self.upto as usize])
  }

  fn next_position(&mut self) -> Result<i32> {
    self.pos_upto += self.pos_jump;
    let position =
      self.high_term().positions.as_ref().unwrap()[self.upto as usize][self.pos_upto as usize];
    if let Some(payloads) = &self.high_term().payloads {
      self.payload = payloads[self.upto as usize][(self.pos_upto / self.pos_jump) as usize]
        .as_ref()
        .map(|bytes| BytesRef::from_bytes(bytes.clone()));
    } else {
      self.payload = None;
    }
    Ok(position)
  }

  fn start_offset(&self) -> Result<i32> {
    if self.has_offsets {
      Ok(
        self.high_term().positions.as_ref().unwrap()[self.upto as usize]
          [self.pos_upto as usize + 1],
      )
    } else {
      Ok(-1)
    }
  }

  fn end_offset(&self) -> Result<i32> {
    if self.has_offsets {
      Ok(
        self.high_term().positions.as_ref().unwrap()[self.upto as usize]
          [self.pos_upto as usize + 2],
      )
    } else {
      Ok(-1)
    }
  }

  fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(self.payload.as_ref().map(Cow::Borrowed))
  }
}

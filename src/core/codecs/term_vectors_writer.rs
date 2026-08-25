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
use crate::core::codecs::DefaultTermVectorsFormat;
use crate::core::codecs::term_vectors_format::TermVectorsFormat;
use crate::core::codecs::term_vectors_reader::TermVectorsReader;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::fields::Fields;
use crate::core::index::merge_state::{DocMap, MergeState, MergeStateMeta};
use crate::core::index::postings_enum::{OFFSETS, PAYLOADS, PostingsEnum};
use crate::core::index::term_vectors::TermVectors;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::index::{BytesRef, BytesRefBuilder, DocIDMerger, Sub, SubBase, of};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::store::DataInput;
use crate::core::util::accountable::Accountable;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::iterator::IteratorExt;
use std::rc::Rc;

struct TermVectorsMergeSub<DM> {
  reader_index: usize,
  max_doc: i32,
  doc_id: i32,
  doc_map: Rc<DM>,
}

impl<DM> TermVectorsMergeSub<DM> {
  fn new(doc_map: Rc<DM>, reader_index: usize, max_doc: i32) -> Self {
    Self {
      reader_index,
      max_doc,
      doc_id: -1,
      doc_map,
    }
  }
}

impl<DM> SubBase for TermVectorsMergeSub<DM>
where
  DM: DocMap,
{
  fn next_doc(&mut self) -> Result<i32> {
    self.doc_id += 1;
    if self.doc_id == self.max_doc {
      Ok(NO_MORE_DOCS)
    } else {
      Ok(self.doc_id)
    }
  }

  type DocMap = DM;

  fn get_doc_map(&self) -> Result<&Self::DocMap> {
    Ok(&self.doc_map)
  }
}

pub trait TermVectorsWriter: Accountable + Closeable {
  fn start_document(&mut self, num_vector_fields: i32) -> Result<()>;

  fn finish_document(&mut self) -> Result<()> {
    Ok(())
  }
  fn start_field(
    &mut self,
    field_info: &FieldInfo,
    num_terms: usize,
    positions: bool,
    offsets: bool,
    payloads: bool,
  ) -> Result<()>;

  fn finish_field(&mut self) -> Result<()> {
    Ok(())
  }

  fn start_term(&mut self, term: &BytesRef<Vec<u8>>, freq: i32) -> Result<()>;

  fn finish_term(&mut self) -> Result<()> {
    Ok(())
  }

  fn add_position(
    &mut self,
    position: i32,
    start_offset: i32,
    end_offset: i32,
    payload: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<()>;

  /// Called before [`Closeable::close`], passing in the number of documents
  /// that were written. Note that this is intentionally redundant (equivalent
  /// to the number of calls to [`Self::start_document`]), but a codec should
  /// check that this is the case to detect the JRE bug described in
  /// LUCENE-1282.
  fn finish(&mut self, num_docs: i32) -> Result<()>;

  /// Called by [`IndexWriter`](crate::core::index::index_writer::IndexWriter) when writing new segments.
  ///
  /// This is an expert API that allows the codec to consume positions and
  /// offsets directly from the indexer.
  ///
  /// The default implementation calls [`Self::add_position`], but
  /// implementations can override this if they want to efficiently write all
  /// the positions, then all the offsets, for example.
  ///
  /// NOTE: This API is extremely expert and subject to change or removal!!!
  fn add_prox(
    &mut self,
    num_prox: usize,
    positions: Option<&mut impl DataInput>,
    offsets: Option<&mut impl DataInput>,
  ) -> Result<()> {
    TermVectorsWriterDefaults::add_prox(self, num_prox, positions, offsets)
  }
  /// Merges in the term vectors from the readers in `merge_state`. The default
  /// implementation skips over deleted documents, and uses
  /// [`Self::start_document`], [`Self::start_field`], [`Self::start_term`],
  /// [`Self::add_position`], and [`Self::finish`], returning the number of
  /// documents that were written. Implementations can override this method for
  /// more sophisticated merging (bulk-byte copying, etc).
  fn merge<D, CR>(&mut self, merge_state: &mut MergeState<D, CR>) -> Result<i32>
  where
    CR: CodecReader,
    Self: Sized,
  {
    TermVectorsWriterDefaults::merge(self, merge_state)
  }

  /// Safe (but, slowish) default method to write every vector field in the document.
  fn add_all_doc_vectors<F, DM>(
    &mut self,
    vectors: Option<&F>,
    merge_state: &MergeStateMeta<DM>,
  ) -> Result<()>
  where
    F: Fields,
    DM: DocMap,
  {
    if vectors.is_none() {
      self.start_document(0)?;
      self.finish_document()?;
      return Ok(());
    }

    let vectors = vectors.unwrap();

    let mut num_fields = vectors.size()?;
    if num_fields == -1 {
      // count manually
      num_fields = 0;
      let mut it = vectors.iterator()?;
      while it.has_next()? {
        it.next()?;
        num_fields += 1;
      }
    }

    self.start_document(num_fields)?;

    let mut last_field_name: Option<String> = None;

    let mut field_count = 0;

    let mut fields_iter = vectors.iterator()?;
    while fields_iter.has_next()? {
      let field_name = fields_iter.next()?.unwrap();
      field_count += 1;

      let field_info = merge_state
        .merge_field_infos
        .field_info_by_name(field_name)?
        .ok_or_else(|| LuceneError::illegal_state("missing FieldInfo"))?;

      if let Some(ref last) = last_field_name {
        debug_assert!(
          field_name > last,
          "lastFieldName={} fieldName={}",
          last,
          field_name
        );
      }
      last_field_name = Some(field_name.clone());

      let Some(terms) = vectors.terms(field_name)? else {
        // Fields iterator should not lie
        continue;
      };

      let has_positions = terms.has_positions();
      let has_offsets = terms.has_offsets();
      let has_payloads = terms.has_payloads();
      debug_assert!(!has_payloads || has_positions);

      let mut num_terms = terms.size()? as i32;
      if num_terms == -1 {
        num_terms = 0;
        let mut terms_enum = terms.iterator()?;
        // count manually. It is stupid, but needed, as Terms.size() is not a mandatory statistics
        // function
        while terms_enum.next()?.is_some() {
          num_terms += 1;
        }
      }

      self.start_field(
        field_info.as_ref(),
        num_terms as usize,
        has_positions,
        has_offsets,
        has_payloads,
      )?;

      let mut terms_enum = terms.iterator()?;
      let mut term_count = 0;

      while let Some(_term) = terms_enum.next()? {
        term_count += 1;

        let freq = terms_enum.total_term_freq()? as i32;
        self.start_term(terms_enum.term()?.as_ref(), freq)?;

        if has_positions || has_offsets {
          let mut docs_and_positions_enum =
            terms_enum.postings_with_flags(None, (OFFSETS | PAYLOADS) as i32)?;

          let doc_id = docs_and_positions_enum.next_doc()?;
          debug_assert!(doc_id != NO_MORE_DOCS);
          debug_assert!(docs_and_positions_enum.freq()? == freq);

          for _ in 0..freq {
            let pos = docs_and_positions_enum.next_position()?;
            let start_offset = docs_and_positions_enum.start_offset()?;
            let end_offset = docs_and_positions_enum.end_offset()?;
            let payload = docs_and_positions_enum.get_payload()?;

            debug_assert!(!has_positions || pos >= 0);
            self.add_position(pos, start_offset, end_offset, payload.as_deref())?;
          }
        }

        self.finish_term()?;
      }

      debug_assert!(term_count == num_terms);
      self.finish_field()?;
    }

    debug_assert!(field_count == num_fields);
    self.finish_document()?;

    Ok(())
  }
}

pub struct TermVectorsWriterDefaults;

impl TermVectorsWriterDefaults {
  pub fn add_prox<W>(
    writer: &mut W,
    num_prox: usize,
    mut positions: Option<&mut impl DataInput>,
    mut offsets: Option<&mut impl DataInput>,
  ) -> Result<()>
  where
    W: TermVectorsWriter + ?Sized,
  {
    let mut position = 0;
    let mut last_offset = 0;
    let mut payload: Option<BytesRefBuilder<Vec<u8>>> = None;

    for _ in 0..num_prox {
      let this_payload = if let Some(pos_input) = positions.as_mut() {
        let code = pos_input.read_vint()?;
        position += (code as u32 >> 1) as i32;

        if code & 1 != 0 {
          let payload_len = pos_input.read_vint()? as usize;

          if payload.is_none() {
            payload = Some(BytesRefBuilder::new());
          }
          let builder = payload.as_mut().unwrap();
          builder.grow_no_copy(payload_len)?;
          pos_input.read_bytes(&mut builder.bytes_ref.bytes, 0, payload_len)?;
          builder.set_length(payload_len);
          Some(builder.get_bytes_ref())
        } else {
          None
        }
      } else {
        position = -1;
        None
      };

      let (start_offset, end_offset) = if let Some(off_input) = offsets.as_mut() {
        let start = last_offset + off_input.read_vint()?;
        let end = start + off_input.read_vint()?;
        last_offset = end;
        (start, end)
      } else {
        (-1, -1)
      };

      writer.add_position(position, start_offset, end_offset, this_payload)?;
    }

    Ok(())
  }

  pub fn merge<W, D, CR>(writer: &mut W, merge_state: &mut MergeState<D, CR>) -> Result<i32>
  where
    W: TermVectorsWriter,
    CR: CodecReader,
  {
    let mut subs = Vec::with_capacity(merge_state.term_vectors_readers.len());
    for i in 0..merge_state.term_vectors_readers.len() {
      if let Some(reader) = &merge_state.term_vectors_readers[i] {
        reader.check_integrity()?;
      }
      subs.push(Sub::new(TermVectorsMergeSub::new(
        merge_state.doc_maps[i].clone(),
        i,
        merge_state.max_docs[i],
      )));
    }

    let mut doc_id_merger = of(subs, merge_state.needs_index_sort)?;
    let merge_state_meta = merge_state.get_meta();
    let mut doc_count = 0;
    while let Some(sub_index) = doc_id_merger.next()? {
      let sub = &doc_id_merger.get_subs()[sub_index].sub;

      // NOTE: it's very important to first assign to vectors then pass it to
      // termVectorsWriter.addAllDocVectors; see LUCENE-1282
      let vectors = match merge_state.term_vectors_readers[sub.reader_index].as_mut() {
        Some(reader) => reader.get(sub.doc_id)?,
        None => None,
      };
      writer.add_all_doc_vectors(vectors.as_ref(), &merge_state_meta)?;
      doc_count += 1;
    }
    writer.finish(doc_count)?;
    Ok(doc_count)
  }
}

pub type DefaultTermVectorsWriter<D> =
  <DefaultTermVectorsFormat as TermVectorsFormat>::TermVectorsWriter<D>;

pub enum TermVectorsWriterEnum2<A, B> {
  A(A),
  B(B),
}

impl<A, B> Closeable for TermVectorsWriterEnum2<A, B>
where
  A: Closeable,
  B: Closeable,
{
  fn close(&mut self) -> Result<()> {
    match self {
      Self::A(inner) => inner.close(),
      Self::B(inner) => inner.close(),
    }
  }
}

impl<A, B> Accountable for TermVectorsWriterEnum2<A, B>
where
  A: Accountable,
  B: Accountable,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    match self {
      Self::A(inner) => inner.ram_bytes_used(),
      Self::B(inner) => inner.ram_bytes_used(),
    }
  }
}

impl<A, B> TermVectorsWriter for TermVectorsWriterEnum2<A, B>
where
  A: TermVectorsWriter,
  B: TermVectorsWriter,
{
  fn start_document(&mut self, num_vector_fields: i32) -> Result<()> {
    match self {
      Self::A(inner) => inner.start_document(num_vector_fields),
      Self::B(inner) => inner.start_document(num_vector_fields),
    }
  }

  fn finish_document(&mut self) -> Result<()> {
    match self {
      Self::A(inner) => inner.finish_document(),
      Self::B(inner) => inner.finish_document(),
    }
  }

  fn start_field(
    &mut self,
    field_info: &FieldInfo,
    num_terms: usize,
    positions: bool,
    offsets: bool,
    payloads: bool,
  ) -> Result<()> {
    match self {
      Self::A(inner) => inner.start_field(field_info, num_terms, positions, offsets, payloads),
      Self::B(inner) => inner.start_field(field_info, num_terms, positions, offsets, payloads),
    }
  }

  fn finish_field(&mut self) -> Result<()> {
    match self {
      Self::A(inner) => inner.finish_field(),
      Self::B(inner) => inner.finish_field(),
    }
  }

  fn start_term(&mut self, term: &BytesRef<Vec<u8>>, freq: i32) -> Result<()> {
    match self {
      Self::A(inner) => inner.start_term(term, freq),
      Self::B(inner) => inner.start_term(term, freq),
    }
  }

  fn finish_term(&mut self) -> Result<()> {
    match self {
      Self::A(inner) => inner.finish_term(),
      Self::B(inner) => inner.finish_term(),
    }
  }

  fn add_position(
    &mut self,
    position: i32,
    start_offset: i32,
    end_offset: i32,
    payload: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<()> {
    match self {
      Self::A(inner) => inner.add_position(position, start_offset, end_offset, payload),
      Self::B(inner) => inner.add_position(position, start_offset, end_offset, payload),
    }
  }

  fn finish(&mut self, num_docs: i32) -> Result<()> {
    match self {
      Self::A(inner) => inner.finish(num_docs),
      Self::B(inner) => inner.finish(num_docs),
    }
  }

  fn add_prox(
    &mut self,
    num_prox: usize,
    positions: Option<&mut impl DataInput>,
    offsets: Option<&mut impl DataInput>,
  ) -> Result<()> {
    match self {
      Self::A(inner) => inner.add_prox(num_prox, positions, offsets),
      Self::B(inner) => inner.add_prox(num_prox, positions, offsets),
    }
  }

  fn merge<D, CR>(&mut self, merge_state: &mut MergeState<D, CR>) -> Result<i32>
  where
    CR: CodecReader,
  {
    match self {
      Self::A(inner) => inner.merge(merge_state),
      Self::B(inner) => inner.merge(merge_state),
    }
  }
}

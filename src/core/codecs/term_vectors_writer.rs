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
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::fields::Fields;
use crate::core::index::merge_state::{DocMap, MergeState, MergeStateMeta};
use crate::core::index::postings_enum::{OFFSETS, PAYLOADS, PostingsEnum};
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::store::DataInput;
use crate::core::store::directory::Directory;
use crate::core::util::accountable::Accountable;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::iterator::IteratorExt;

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

  fn finish<D>(&mut self, num_docs: i32, dir: &D) -> Result<()>
  where
    D: Directory;
  fn finish_add_prox(&mut self, num_prox: usize) -> Result<()>;
  fn add_positions(&mut self, num_prox: usize, positions: &mut impl DataInput) -> Result<()>;
  fn add_offsets(&mut self, num_prox: usize, offsets: &mut impl DataInput) -> Result<()>;

  fn default_add_prox(
    &mut self,
    num_prox: usize,
    mut positions: Option<&mut impl DataInput>,
    mut offsets: Option<&mut impl DataInput>,
  ) -> Result<()> {
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

      self.add_position(position, start_offset, end_offset, this_payload)?;
    }

    Ok(())
  }
  fn merge<D, D1, CR>(&mut self, merge_state: &mut MergeState<D, CR>, dir: &D1) -> Result<i32>
  where
    D: Directory,
    D1: Directory,
    CR: CodecReader;

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
        .field_info_by_name(field_name)
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
pub type DefaultTermVectorsWriter<D> =
  <DefaultTermVectorsFormat as TermVectorsFormat>::TermVectorsWriter<D>;

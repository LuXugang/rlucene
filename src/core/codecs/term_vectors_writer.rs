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
use crate::core::index::merge_state::{MergeState, MergeStateMeta};
use crate::core::index::postings_enum::{OFFSETS, PAYLOADS, PostingsEnum};
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::store::DataInput;
use crate::core::store::directory::Directory;
use crate::core::util::accountable::Accountable;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::iterator::IteratorExt;

pub trait TermVectorsWriter: Accountable {
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
        positions: &mut Option<&mut impl DataInput>,
        offsets: &mut Option<&mut impl DataInput>,
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
                    builder.grow_no_copy(payload_len);
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
    fn add_all_doc_vectors<F, CR>(
        &mut self,
        vectors: Option<&F>,
        merge_state: &MergeStateMeta<CR>,
    ) -> Result<()>
    where
        F: Fields,
        CR: CodecReader,
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
pub type DefaultTermVectorsWriter<O> =
    <DefaultTermVectorsFormat as TermVectorsFormat>::TermVectorsWriter<O>;

#[cfg(test)]
mod tests {
    use crate::core::document::document::Document;
    use crate::core::document::field::{Field, Store};
    use crate::core::document::field_type::FieldType;
    use crate::core::document::stored_field::stored_field_type;
    use crate::core::document::string_field::string_field_type;
    use crate::core::document::text_field::text_field_type;
    use crate::core::index::BytesRef;
    use crate::core::index::directory_reader::directory_reader_util;
    use crate::core::index::fields::Fields;
    use crate::core::index::index_reader::IndexReader;
    use crate::core::index::index_writer::IndexWriter;
    use crate::core::index::index_writer_config::DISABLE_AUTO_FLUSH;
    use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
    use crate::core::index::log_merge_policy::LogMergePolicy;
    use crate::core::index::postings_enum::{ALL, PostingsEnum};
    use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
    use crate::core::index::stored_fields::StoredFields;
    use crate::core::index::term_vectors::TermVectors;
    use crate::core::index::terms::Terms;
    use crate::core::index::terms_enum::TermsEnum;
    use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::core::util::bytes_ref_iterator::BytesRefIterator;
    use crate::core::util::error::lucene_error::{LuceneError, Result};
    use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
    use crate::test::core::index::random_index_writer::RandomIndexWriter;
    use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
        new_directory_shared, new_field, new_index_writer_config,
        new_index_writer_config_with_analyzer, new_string_field, new_text_field, random,
    };
    use rand::Rng;
    use std::collections::HashMap;

    #[allow(dead_code)]
    struct TestTermVectorsWriter;

    #[test]
    fn test_double_offset_counting() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        let mock = MockAnalyzer::new(&mut random);
        let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
        let w = IndexWriter::new(dir.clone(), iwc)?;

        let mut field_types = HashMap::new();
        let mut doc = Document::new();

        let mut custom_type = FieldType::from_ref(&*string_field_type::TYPE_NOT_STORED)?;
        custom_type.set_store_term_vectors(true)?;
        custom_type.set_store_term_vector_positions(true)?;
        custom_type.set_store_term_vector_offsets(true)?;

        let f = new_field(&mut random, "field", "abcd", &custom_type, &mut field_types)?;
        doc.add(f.clone());
        doc.add(f.clone());

        let f2 = new_field(&mut random, "field", "", &custom_type, &mut field_types)?;
        doc.add(f2);

        doc.add(f);

        w.add_document(doc)?;
        w.close()?;

        let reader = directory_reader_util::open(dir.clone())?;

        let mut tv_reader = reader.term_vectors()?;
        let field0 = tv_reader.get(0)?;
        let tv = field0.as_ref().unwrap();
        let terms = tv.terms("field")?;
        let terms = terms.as_ref().unwrap();
        let mut terms_enum = terms.iterator()?;

        assert!(terms_enum.next()?.is_some());
        assert_eq!("", terms_enum.term()?.utf8_to_string()?);

        assert_eq!(1, terms_enum.total_term_freq()?);

        let mut dp_enum = terms_enum.postings_with_flags(None, ALL as i32)?;
        assert_ne!(dp_enum.next_doc()?, NO_MORE_DOCS);

        dp_enum.next_position()?;
        assert_eq!(8, dp_enum.start_offset()?);
        assert_eq!(8, dp_enum.end_offset()?);

        assert_eq!(NO_MORE_DOCS, dp_enum.next_doc()?);

        let next = terms_enum.next()?;
        assert!(next.is_some());
        assert_eq!(&BytesRef::from_string("abcd"), next.unwrap().as_ref());

        let mut dp_enum = terms_enum.postings_with_flags(Some(dp_enum), ALL as i32)?;
        assert_eq!(3, terms_enum.total_term_freq()?);

        assert_ne!(dp_enum.next_doc()?, NO_MORE_DOCS);

        dp_enum.next_position()?;
        assert_eq!(0, dp_enum.start_offset()?);
        assert_eq!(4, dp_enum.end_offset()?);

        dp_enum.next_position()?;
        assert_eq!(4, dp_enum.start_offset()?);
        assert_eq!(8, dp_enum.end_offset()?);

        dp_enum.next_position()?;
        assert_eq!(8, dp_enum.start_offset()?);
        assert_eq!(12, dp_enum.end_offset()?);

        assert_eq!(NO_MORE_DOCS, dp_enum.next_doc()?);

        assert!(terms_enum.next()?.is_none());

        Ok(())
    }

    #[test]
    fn test_double_offset_counting2() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        let mock = MockAnalyzer::new(&mut random);
        let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
        let w = IndexWriter::new(dir.clone(), iwc)?;

        let mut field_types = HashMap::new();
        let mut doc = Document::new();

        let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        custom_type.set_store_term_vectors(true)?;
        custom_type.set_store_term_vector_positions(true)?;
        custom_type.set_store_term_vector_offsets(true)?;

        let f = new_field(&mut random, "field", "abcd", &custom_type, &mut field_types)?;
        doc.add(f.clone());
        doc.add(f);

        w.add_document(doc)?;
        w.close()?;

        let reader = directory_reader_util::open(dir.clone())?;

        let mut tv_reader = reader.term_vectors()?;
        let field = tv_reader.get(0)?;
        let tv = field.as_ref().unwrap();
        let terms = tv.terms("field")?;
        let terms = terms.as_ref().unwrap();
        let mut terms_enum = terms.iterator()?;

        assert!(terms_enum.next()?.is_some());

        let mut dp_enum = terms_enum.postings_with_flags(None, ALL as i32)?;

        assert_eq!(2, terms_enum.total_term_freq()?);

        assert_ne!(dp_enum.next_doc()?, NO_MORE_DOCS);

        dp_enum.next_position()?;
        assert_eq!(0, dp_enum.start_offset()?);
        assert_eq!(4, dp_enum.end_offset()?);

        dp_enum.next_position()?;
        assert_eq!(5, dp_enum.start_offset()?);
        assert_eq!(9, dp_enum.end_offset()?);

        assert_eq!(NO_MORE_DOCS, dp_enum.next_doc()?);

        Ok(())
    }

    #[test]
    fn test_end_offset_position_char_analyzer() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;

        let mock = MockAnalyzer::new(&mut random);
        let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
        let w = IndexWriter::new(dir.clone(), iwc)?;

        let mut field_types = HashMap::new();
        let mut doc = Document::new();

        let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        custom_type.set_store_term_vectors(true)?;
        custom_type.set_store_term_vector_positions(true)?;
        custom_type.set_store_term_vector_offsets(true)?;

        let f = new_field(
            &mut random,
            "field",
            "abcd   ",
            &custom_type,
            &mut field_types,
        )?;
        doc.add(f.clone());
        doc.add(f);

        w.add_document(doc)?;
        w.close()?;

        let reader = directory_reader_util::open(dir.clone())?;

        let mut tv_reader = reader.term_vectors()?;
        let field = tv_reader.get(0)?;
        let tv = field.as_ref().unwrap();
        let terms = tv.terms("field")?;
        let terms = terms.as_ref().unwrap();
        let mut terms_enum = terms.iterator()?;

        assert!(terms_enum.next()?.is_some());

        let mut dp_enum = terms_enum.postings_with_flags(None, ALL as i32)?;

        assert_eq!(2, terms_enum.total_term_freq()?);

        assert_ne!(dp_enum.next_doc()?, NO_MORE_DOCS);

        dp_enum.next_position()?;
        assert_eq!(0, dp_enum.start_offset()?);
        assert_eq!(4, dp_enum.end_offset()?);

        dp_enum.next_position()?;
        assert_eq!(8, dp_enum.start_offset()?);
        assert_eq!(12, dp_enum.end_offset()?);

        assert_eq!(NO_MORE_DOCS, dp_enum.next_doc()?);

        Ok(())
    }
    #[test]
    fn test_end_offset_position_with_caching_token_filter() -> Result<()> {
        // TODO CachingTokenFilter未实现
        Ok(())
    }
    #[test]
    fn test_end_offset_position_stop_filter() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        let mock = MockAnalyzer::new(&mut random);
        let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
        let w = IndexWriter::new(dir.clone(), iwc)?;

        let mut field_types = HashMap::new();
        let mut doc = Document::new();

        let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        custom_type.set_store_term_vectors(true)?;
        custom_type.set_store_term_vector_positions(true)?;
        custom_type.set_store_term_vector_offsets(true)?;

        let f = new_field(
            &mut random,
            "field",
            "abcd the",
            &custom_type,
            &mut field_types,
        )?;
        doc.add(f.clone());
        doc.add(f);

        w.add_document(doc)?;
        w.close()?;

        let reader = directory_reader_util::open(dir.clone())?;

        let mut tv_reader = reader.term_vectors()?;
        let field = tv_reader.get(0)?;
        let tv = field.as_ref().unwrap();
        let terms = tv.terms("field")?;
        let terms = terms.as_ref().unwrap();
        let mut terms_enum = terms.iterator()?;

        assert!(terms_enum.next()?.is_some());

        let mut dp_enum = terms_enum.postings_with_flags(None, ALL as i32)?;

        assert_eq!(2, terms_enum.total_term_freq()?);

        assert_ne!(dp_enum.next_doc()?, NO_MORE_DOCS);

        dp_enum.next_position()?;
        assert_eq!(0, dp_enum.start_offset()?);
        assert_eq!(4, dp_enum.end_offset()?);

        dp_enum.next_position()?;
        assert_eq!(9, dp_enum.start_offset()?);
        assert_eq!(13, dp_enum.end_offset()?);

        assert_eq!(NO_MORE_DOCS, dp_enum.next_doc()?);

        Ok(())
    }
    #[test]
    fn test_end_offset_position_standard() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        let mock = MockAnalyzer::new(&mut random);
        let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
        let w = IndexWriter::new(dir.clone(), iwc)?;

        let mut field_types = HashMap::new();
        let mut doc = Document::new();

        let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        custom_type.set_store_term_vectors(true)?;
        custom_type.set_store_term_vector_positions(true)?;
        custom_type.set_store_term_vector_offsets(true)?;

        let f = new_field(
            &mut random,
            "field",
            "abcd the  ",
            &custom_type,
            &mut field_types,
        )?;
        let f2 = new_field(
            &mut random,
            "field",
            "crunch man",
            &custom_type,
            &mut field_types,
        )?;
        doc.add(f);
        doc.add(f2);

        w.add_document(doc)?;
        w.close()?;

        let reader = directory_reader_util::open(dir.clone())?;

        let mut tv_reader = reader.term_vectors()?;
        let field = tv_reader.get(0)?;
        let tv = field.as_ref().unwrap();
        let terms = tv.terms("field")?;
        let terms = terms.as_ref().unwrap();

        let mut terms_enum = terms.iterator()?;

        assert!(terms_enum.next()?.is_some());
        let mut dp_enum = terms_enum.postings_with_flags(None, ALL as i32)?;

        assert_ne!(dp_enum.next_doc()?, NO_MORE_DOCS);
        dp_enum.next_position()?;
        assert_eq!(0, dp_enum.start_offset()?);
        assert_eq!(4, dp_enum.end_offset()?);

        assert!(terms_enum.next()?.is_some());
        dp_enum = terms_enum.postings_with_flags(Some(dp_enum), ALL as i32)?;
        assert_ne!(dp_enum.next_doc()?, NO_MORE_DOCS);
        dp_enum.next_position()?;
        assert_eq!(11, dp_enum.start_offset()?);
        assert_eq!(17, dp_enum.end_offset()?);

        assert!(terms_enum.next()?.is_some());
        dp_enum = terms_enum.postings_with_flags(Some(dp_enum), ALL as i32)?;
        assert_ne!(dp_enum.next_doc()?, NO_MORE_DOCS);
        dp_enum.next_position()?;
        assert_eq!(18, dp_enum.start_offset()?);
        assert_eq!(21, dp_enum.end_offset()?);

        Ok(())
    }
    #[test]
    fn test_end_offset_position_standard_empty_field() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        let mock = MockAnalyzer::new(&mut random);
        let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
        let w = IndexWriter::new(dir.clone(), iwc)?;

        let mut field_types = HashMap::new();
        let mut doc = Document::new();

        let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        custom_type.set_store_term_vectors(true)?;
        custom_type.set_store_term_vector_positions(true)?;
        custom_type.set_store_term_vector_offsets(true)?;

        let f = new_field(&mut random, "field", "", &custom_type, &mut field_types)?;
        let f2 = new_field(
            &mut random,
            "field",
            "crunch man",
            &custom_type,
            &mut field_types,
        )?;
        doc.add(f);
        doc.add(f2);

        w.add_document(doc)?;
        w.close()?;

        let reader = directory_reader_util::open(dir.clone())?;

        let mut tv_reader = reader.term_vectors()?;
        let field = tv_reader.get(0)?;
        let tv = field.as_ref().unwrap();
        let terms = tv.terms("field")?;
        let terms = terms.as_ref().unwrap();

        let mut terms_enum = terms.iterator()?;

        assert!(terms_enum.next()?.is_some());
        let mut dp_enum = terms_enum.postings_with_flags(None, ALL as i32)?;

        assert_eq!(1, terms_enum.total_term_freq()? as i32);

        assert_ne!(dp_enum.next_doc()?, NO_MORE_DOCS);
        dp_enum.next_position()?;
        assert_eq!(1, dp_enum.start_offset()?);
        assert_eq!(7, dp_enum.end_offset()?);

        assert!(terms_enum.next()?.is_some());
        dp_enum = terms_enum.postings_with_flags(Some(dp_enum), ALL as i32)?;

        assert_ne!(dp_enum.next_doc()?, NO_MORE_DOCS);
        dp_enum.next_position()?;
        assert_eq!(8, dp_enum.start_offset()?);
        assert_eq!(11, dp_enum.end_offset()?);

        Ok(())
    }
    #[test]
    fn test_end_offset_position_standard_empty_field2() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;

        let mock = MockAnalyzer::new(&mut random);
        let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
        let w = IndexWriter::new(dir.clone(), iwc)?;

        let mut field_types = HashMap::new();
        let mut doc = Document::new();

        let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        custom_type.set_store_term_vectors(true)?;
        custom_type.set_store_term_vector_positions(true)?;
        custom_type.set_store_term_vector_offsets(true)?;

        let f = new_field(&mut random, "field", "abcd", &custom_type, &mut field_types)?;
        doc.add(f);

        doc.add(new_field(
            &mut random,
            "field",
            "",
            &custom_type,
            &mut field_types,
        )?);

        let f2 = new_field(
            &mut random,
            "field",
            "crunch",
            &custom_type,
            &mut field_types,
        )?;
        doc.add(f2);

        w.add_document(doc)?;
        w.close()?;

        let reader = directory_reader_util::open(dir.clone())?;

        let mut tv_reader = reader.term_vectors()?;
        let field = tv_reader.get(0)?;
        let tv = field.as_ref().unwrap();
        let terms = tv.terms("field")?;
        let terms = terms.as_ref().unwrap();

        let mut terms_enum = terms.iterator()?;

        assert!(terms_enum.next()?.is_some());

        let mut dp_enum = terms_enum.postings_with_flags(None, ALL as i32)?;

        assert_eq!(1, terms_enum.total_term_freq()? as i32);

        assert_ne!(dp_enum.next_doc()?, NO_MORE_DOCS);

        dp_enum.next_position()?;
        assert_eq!(0, dp_enum.start_offset()?);
        assert_eq!(4, dp_enum.end_offset()?);

        assert!(terms_enum.next()?.is_some());

        dp_enum = terms_enum.postings_with_flags(Some(dp_enum), ALL as i32)?;

        assert_ne!(dp_enum.next_doc()?, NO_MORE_DOCS);

        dp_enum.next_position()?;
        assert_eq!(6, dp_enum.start_offset()?);
        assert_eq!(12, dp_enum.end_offset()?);

        Ok(())
    }
    #[test]
    fn test_term_vector_corruption() -> Result<()> {
        // TODO add_indexes未实现
        Ok(())
    }
    #[test]
    fn test_term_vector_corruption2() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        let mut field_types = HashMap::new();

        for _ in 0..2 {
            let mock = MockAnalyzer::new(&mut random);
            let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
            iwc.set_max_buffered_docs(2);
            iwc.set_ram_buffer_size_mb(DISABLE_AUTO_FLUSH as f64);
            iwc.set_merge_scheduler(SerialMergeScheduler);
            iwc.set_merge_policy(LogMergePolicy::log_doc());

            let writer = IndexWriter::new(dir.clone(), iwc)?;

            let mut document = Document::new();

            let mut custom_type = FieldType::new();
            custom_type.set_stored(true)?;

            let stored_field = new_field(
                &mut random,
                "stored",
                "stored",
                &custom_type,
                &mut field_types,
            )?;
            document.add(stored_field.clone());

            writer.add_document(document.clone())?;
            writer.add_document(document)?;

            let mut document = Document::new();
            document.add(stored_field);

            let mut custom_type2 = FieldType::from_ref(&*string_field_type::TYPE_NOT_STORED)?;
            custom_type2.set_store_term_vectors(true)?;
            custom_type2.set_store_term_vector_positions(true)?;
            custom_type2.set_store_term_vector_offsets(true)?;

            let term_vector_field = new_field(
                &mut random,
                "termVector",
                "termVector",
                &custom_type2,
                &mut field_types,
            )?;
            document.add(term_vector_field);

            writer.add_document(document)?;
            writer.force_merge(1)?;
            writer.close()?;

            let reader = directory_reader_util::open(dir.clone())?;
            let mut tv_reader = reader.term_vectors()?;

            assert!(tv_reader.get(0)?.is_none());
            assert!(tv_reader.get(1)?.is_none());
            assert!(tv_reader.get(2)?.is_some());
        }

        Ok(())
    }
    #[test]
    fn test_term_vector_corruption3() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        let mock = MockAnalyzer::new(&mut random);
        let mut iwc1 = new_index_writer_config_with_analyzer(&mut random, mock);
        iwc1.set_max_buffered_docs(2);
        iwc1.set_ram_buffer_size_mb(DISABLE_AUTO_FLUSH as f64);
        iwc1.set_merge_scheduler(SerialMergeScheduler);
        iwc1.set_merge_policy(LogMergePolicy::log_doc());
        let mut document = Document::new();
        let mut field_types = HashMap::new();
        {
            let writer = IndexWriter::new(dir.clone(), iwc1)?;

            let mut custom_type = FieldType::new();
            custom_type.set_stored(true)?;
            let stored_field = new_field(
                &mut random,
                "stored",
                "stored",
                &custom_type,
                &mut field_types,
            )?;
            document.add(stored_field.clone());

            let mut custom_type2 = FieldType::from_ref(&*string_field_type::TYPE_NOT_STORED)?;
            custom_type2.set_store_term_vectors(true)?;
            custom_type2.set_store_term_vector_positions(true)?;
            custom_type2.set_store_term_vector_offsets(true)?;
            let term_vector_field = new_field(
                &mut random,
                "termVector",
                "termVector",
                &custom_type2,
                &mut field_types,
            )?;
            document.add(term_vector_field.clone());

            for _ in 0..10 {
                writer.add_document(document.clone())?;
            }
            writer.close()?;
        }

        let mock = MockAnalyzer::new(&mut random);
        let mut iwc2 = new_index_writer_config_with_analyzer(&mut random, mock);
        iwc2.set_max_buffered_docs(2);
        iwc2.set_ram_buffer_size_mb(DISABLE_AUTO_FLUSH as f64);
        iwc2.set_merge_scheduler(SerialMergeScheduler);
        iwc2.set_merge_policy(LogMergePolicy::log_doc());

        let writer = IndexWriter::new(dir.clone(), iwc2)?;

        for _ in 0..6 {
            writer.add_document(document.clone())?;
        }
        writer.force_merge(1)?;
        writer.close()?;

        let reader = directory_reader_util::open(dir.clone())?;

        let mut stored_fields = reader.stored_fields()?;
        let mut term_vectors = reader.term_vectors()?;

        for i in 0..10 {
            term_vectors.get(i)?;
            stored_fields.document(i)?;
        }

        Ok(())
    }
    #[test]
    fn test_no_term_vector_after_term_vector() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;

        let iwc = new_index_writer_config(&mut random);
        let iw = IndexWriter::new(dir.clone(), iwc)?;

        let mut field_types = HashMap::new();
        let mut custom_type2 = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        custom_type2.set_store_term_vectors(true)?;
        custom_type2.set_store_term_vector_positions(true)?;
        custom_type2.set_store_term_vector_offsets(true)?;

        let mut document = Document::new();
        document.add(new_field(
            &mut random,
            "tvtest",
            "a b c",
            &custom_type2,
            &mut field_types,
        )?);
        iw.add_document(document)?;

        let mut document = Document::new();
        document.add(new_text_field(
            &mut random,
            "tvtest",
            "x y z",
            Store::No,
            &mut field_types,
        )?);
        iw.add_document(document)?;

        iw.commit()?;

        let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        custom_type.set_store_term_vectors(true)?;

        let mut document = Document::new();
        document.add(new_field(
            &mut random,
            "tvtest",
            "a b c",
            &custom_type,
            &mut field_types,
        )?);
        iw.add_document(document)?;

        iw.commit()?;

        iw.force_merge(1)?;
        iw.close()?;

        Ok(())
    }
    #[test]
    fn test_no_term_vector_after_term_vector_merge() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        let mock = MockAnalyzer::new(&mut random);
        let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
        let iw = IndexWriter::new(dir.clone(), iwc)?;
        let mut field_types = HashMap::new();
        let mut document = Document::new();
        let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        custom_type.set_store_term_vectors(true)?;
        document.add(new_field(
            &mut random,
            "tvtest",
            "a b c",
            &custom_type,
            &mut field_types,
        )?);
        iw.add_document(document)?;
        iw.commit()?;

        let mut document = Document::new();
        document.add(new_text_field(
            &mut random,
            "tvtest",
            "x y z",
            Store::No,
            &mut field_types,
        )?);
        iw.add_document(document)?;
        // Make first segment
        iw.commit()?;

        iw.force_merge(1)?;

        let mut custom_type2 = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        custom_type2.set_store_term_vectors(true)?;

        let mut document = Document::new();
        document.add(new_field(
            &mut random,
            "tvtest",
            "a b c",
            &custom_type2,
            &mut field_types,
        )?);
        iw.add_document(document)?;
        // Make 2nd segment
        iw.commit()?;
        iw.force_merge(1)?;
        iw.close()?;
        Ok(())
    }
    #[test]
    fn test_inconsistent_term_vector_options() -> Result<()> {
        let mut random = random();
        let mut a;
        let mut b;

        // no vectors + vectors
        a = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        b = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        b.set_store_term_vectors(true)?;
        do_test_mixup(&mut random, a, b)?;

        // vectors + vectors with pos
        a = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        a.set_store_term_vectors(true)?;
        b = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        b.set_store_term_vectors(true)?;
        b.set_store_term_vector_positions(true)?;
        do_test_mixup(&mut random, a, b)?;

        // vectors + vectors with off
        a = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        a.set_store_term_vectors(true)?;
        b = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        b.set_store_term_vectors(true)?;
        b.set_store_term_vector_offsets(true)?;
        do_test_mixup(&mut random, a, b)?;

        // vectors with pos + vectors with pos + off
        a = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        a.set_store_term_vectors(true)?;
        a.set_store_term_vector_positions(true)?;
        b = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        b.set_store_term_vectors(true)?;
        b.set_store_term_vector_positions(true)?;
        b.set_store_term_vector_offsets(true)?;
        do_test_mixup(&mut random, a, b)?;

        // vectors with pos + vectors with pos + pay
        a = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        a.set_store_term_vectors(true)?;
        a.set_store_term_vector_positions(true)?;
        b = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        b.set_store_term_vectors(true)?;
        b.set_store_term_vector_positions(true)?;
        b.set_store_term_vector_payloads(true)?;
        do_test_mixup(&mut random, a, b)?;
        Ok(())
    }

    fn do_test_mixup<R: Rng + ?Sized>(
        random: &mut R,
        ft1: FieldType,
        ft2: FieldType,
    ) -> Result<()> {
        let dir = new_directory_shared(random)?;
        let iw = RandomIndexWriter::new(random, dir.clone());

        let mut field_types = HashMap::new();
        for i in 0..3 {
            let mut doc = Document::new();
            doc.add(new_string_field(
                random,
                "id",
                i.to_string(),
                Store::No,
                &mut field_types,
            )?);
            iw.add_document(doc)?;
        }

        let mut doc = Document::new();
        doc.add(Field::new("field", "value1", ft1.clone()));
        doc.add(Field::new("field", "value1", ft2.clone()));

        // ensure broken doc hits exception
        let err = iw.add_document(doc).unwrap_err();
        match err {
            LuceneError::IllegalArgument(msg) => {
                assert!(
                    msg.to_string().starts_with("all instances of a given field name must have the same term vectors settings")
                        || msg.to_string().starts_with("Inconsistency of field data structures across documents for field [field]")
                );
            },
            _ => unreachable!("unexpected error type: {:?}", err),
        }
        let ir = iw.get_reader()?;
        assert_eq!(3, ir.num_docs()?);
        iw.close()?;
        Ok(())
    }
    #[test]
    fn test_no_abort_on_bad_tv_settings() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        // Don't use RandomIndexWriter because we want to be sure both docs go to 1 seg:
        let iwc = new_index_writer_config(&mut random);
        let iw = IndexWriter::new(dir.clone(), iwc)?;
        let mut doc = Document::new();
        iw.add_document(doc.clone())?;

        let mut ft = FieldType::from_ref(&*stored_field_type::TYPE)?;
        ft.set_store_term_vectors(true)?;
        ft.freeze();

        doc.add(Field::from_string("field", "value", ft)?);

        let err = iw.add_document(doc.clone()).unwrap_err();
        match err {
            LuceneError::IllegalArgument(_) => {},
            _ => unreachable!("unexpected error: {:?}", err),
        }

        let reader = directory_reader_util::open_from_writer(&iw)?;
        // Make sure the exc didn't lose our first document:
        assert_eq!(1, reader.num_docs()?);

        iw.close()?;
        Ok(())
    }
}

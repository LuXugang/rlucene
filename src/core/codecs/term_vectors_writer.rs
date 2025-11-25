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
use crate::core::codecs::compressing::lucene90_compressing_term_vectors_writer::Lucene90CompressingTermVectorsWriter;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::store::DataInput;
use crate::core::store::directory::Directory;
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::Result;

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

    fn add_prox(
        &mut self,
        num_prox: usize,
        positions: &mut Option<&mut impl DataInput>,
        offsets: &mut Option<&mut impl DataInput>,
    ) -> Result<()>;

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
                    pos_input.read_bytes(&mut builder.bytes_ref.bytes, 0, payload_len as i32)?;
                    builder.set_length(payload_len);
                    Some(builder.get_bytes_ref())
                } else {
                    None
                }
            } else {
                position = -1;
                None
            };

            // --- offsets ---
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
}

pub enum TermVectorsWriterEnum<D>
where
    D: Directory,
{
    Lucene90(Lucene90CompressingTermVectorsWriter<D>),
}

impl<D> Accountable for TermVectorsWriterEnum<D>
where
    D: Directory,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        match self {
            TermVectorsWriterEnum::Lucene90(writer) => writer.ram_bytes_used(),
        }
    }
}

impl<D> TermVectorsWriter for TermVectorsWriterEnum<D>
where
    D: Directory,
{
    fn start_document(&mut self, num_vector_fields: i32) -> Result<()> {
        match self {
            TermVectorsWriterEnum::Lucene90(writer) => writer.start_document(num_vector_fields),
        }
    }

    fn finish_document(&mut self) -> Result<()> {
        match self {
            TermVectorsWriterEnum::Lucene90(writer) => writer.finish_document(),
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
            TermVectorsWriterEnum::Lucene90(writer) => {
                writer.start_field(field_info, num_terms, positions, offsets, payloads)
            },
        }
    }

    fn finish_field(&mut self) -> Result<()> {
        match self {
            TermVectorsWriterEnum::Lucene90(writer) => writer.finish_field(),
        }
    }

    fn start_term(&mut self, term: &BytesRef<Vec<u8>>, freq: i32) -> Result<()> {
        match self {
            TermVectorsWriterEnum::Lucene90(writer) => writer.start_term(term, freq),
        }
    }

    fn finish_term(&mut self) -> Result<()> {
        match self {
            TermVectorsWriterEnum::Lucene90(writer) => writer.finish_term(),
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
            TermVectorsWriterEnum::Lucene90(writer) => {
                writer.add_position(position, start_offset, end_offset, payload)
            },
        }
    }

    fn finish<D1>(&mut self, num_docs: i32, dir: &D1) -> Result<()>
    where
        D1: Directory,
    {
        match self {
            TermVectorsWriterEnum::Lucene90(writer) => writer.finish(num_docs, dir),
        }
    }

    fn add_prox(
        &mut self,
        num_prox: usize,
        positions: &mut Option<&mut impl DataInput>,
        offsets: &mut Option<&mut impl DataInput>,
    ) -> Result<()> {
        match self {
            TermVectorsWriterEnum::Lucene90(writer) => {
                writer.add_prox(num_prox, positions, offsets)
            },
        }
    }

    fn default_add_prox(
        &mut self,
        num_prox: usize,
        positions: &mut Option<&mut impl DataInput>,
        offsets: &mut Option<&mut impl DataInput>,
    ) -> Result<()> {
        match self {
            TermVectorsWriterEnum::Lucene90(writer) => {
                writer.default_add_prox(num_prox, positions, offsets)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::document::document::Document;
    use std::sync::Arc;

    use crate::core::document::field_type::FieldType;
    use crate::core::document::string_field::string_field_type;
    use crate::core::document::text_field::text_field_type;
    use crate::core::index::BytesRef;
    use crate::core::index::directory_reader::directory_reader_util;
    use crate::core::index::fields::Fields;
    use crate::core::index::index_reader::IndexReader;
    use crate::core::index::index_writer::IndexWriter;
    use crate::core::index::postings_enum::{ALL, PostingsEnum};
    use crate::core::index::term_vectors::TermVectors;
    use crate::core::index::terms::Terms;
    use crate::core::index::terms_enum::TermsEnum;
    use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::core::util::bytes_ref_iterator::BytesRefIterator;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        new_directory, new_field, new_index_writer_config, random,
    };

    #[allow(dead_code)]
    struct TestTermVectorsWriter;
    #[test]
    fn test_double_offset_counting() -> Result<()> {
        let mut random = random();

        let dir = Arc::new(new_directory(&mut random)?);
        // TODO: 未实现MockAnalyzer
        let iwc = new_index_writer_config(&mut random);
        let w = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();

        let mut custom_type = FieldType::from_ref(&string_field_type::TYPE_NOT_STORED.clone())?;
        custom_type.set_store_term_vectors(true)?;
        custom_type.set_store_term_vector_positions(true)?;
        custom_type.set_store_term_vector_offsets(true)?;

        let f = new_field("field", "abcd", &custom_type)?;
        doc.add(f.clone());
        doc.add(f.clone());

        let f2 = new_field("field", "", &custom_type)?;
        doc.add(f2);

        doc.add(f);

        w.add_document(doc)?;
        w.close()?;

        let reader = Arc::new(directory_reader_util::open(dir.clone())?);

        let mut tv_reader = reader.term_vectors()?;
        let field0 = tv_reader.get(0)?;
        let tv = field0.as_ref().unwrap();
        let terms = tv.terms("field")?;
        let terms = terms.as_ref().unwrap();
        let mut terms_enum = terms.iterator()?;

        // First token = ""
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
        let dir = Arc::new(new_directory(&mut random)?);
        // TODO: 未实现MockAnalyzer
        let iwc = new_index_writer_config(&mut random);
        let w = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();

        let mut custom_type = FieldType::from_ref(&text_field_type::TYPE_NOT_STORED.clone())?;
        custom_type.set_store_term_vectors(true)?;
        custom_type.set_store_term_vector_positions(true)?;
        custom_type.set_store_term_vector_offsets(true)?;

        let f = new_field("field", "abcd", &custom_type)?;
        doc.add(f.clone());
        doc.add(f);

        w.add_document(doc)?;
        w.close()?;

        let reader = Arc::new(directory_reader_util::open(dir.clone())?);

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

        let dir = Arc::new(new_directory(&mut random)?);

        // TODO: 未实现MockAnalyzer
        let iwc = new_index_writer_config(&mut random);
        let w = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();

        let mut custom_type = FieldType::from_ref(&text_field_type::TYPE_NOT_STORED.clone())?;
        custom_type.set_store_term_vectors(true)?;
        custom_type.set_store_term_vector_positions(true)?;
        custom_type.set_store_term_vector_offsets(true)?;

        let f = new_field("field", "abcd   ", &custom_type)?;
        doc.add(f.clone());
        doc.add(f);

        w.add_document(doc)?;
        w.close()?;

        let reader = Arc::new(directory_reader_util::open(dir.clone())?);

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
}

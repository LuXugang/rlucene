/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use crate::codecs::compressing::lucene90_compressing_term_vectors_writer::Lucene90CompressingTermVectorsWriter;
use crate::index::field_info::FieldInfo;
use crate::index::{BytesRef, BytesRefBuilder};
use crate::store::directory::Directory;
use crate::store::DataInput;
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::Result;

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

    fn finish(&mut self, num_docs: i32) -> Result<()>;

    fn add_prox(
        &mut self,
        num_prox: usize,
        positions: &mut Option<impl DataInput>,
        offsets: &mut Option<impl DataInput>,
    ) -> Result<()>;

    fn default_add_prox(
        &mut self,
        num_prox: usize,
        positions: &mut Option<impl DataInput>,
        offsets: &mut Option<impl DataInput>,
    ) -> Result<()> {
        let mut position = 0;
        let mut last_offset = 0;
        let mut payload: Option<BytesRefBuilder<Vec<u8>>> = None;

        for _ in 0..num_prox {
            let (start_offset, end_offset);
            let this_payload;

            if let Some(pos_input) = positions.as_mut() {
                let code = pos_input.read_vint()?;
                position += (code as u32 >> 1) as i32;

                if code & 1 != 0 {
                    // This position has a payload
                    let payload_len = pos_input.read_vint()? as usize;

                    if payload.is_none() {
                        payload = Some(BytesRefBuilder::new());
                    }
                    let builder = payload.as_mut().unwrap();
                    builder.grow_no_copy(payload_len);
                    pos_input.read_bytes(&mut builder.bytes_ref.bytes, 0, payload_len as i32)?;
                    builder.set_length(payload_len);
                    this_payload = Some(builder.get_bytes_ref());
                } else {
                    this_payload = None;
                }
            } else {
                position = -1;
                this_payload = None;
            }

            if let Some(off_input) = offsets.as_mut() {
                start_offset = last_offset + off_input.read_vint()?;
                end_offset = start_offset + off_input.read_vint()?;
                last_offset = end_offset;
            } else {
                start_offset = -1;
                end_offset = -1;
            }

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

    fn finish(&mut self, num_docs: i32) -> Result<()> {
        match self {
            TermVectorsWriterEnum::Lucene90(writer) => writer.finish(num_docs),
        }
    }

    fn add_prox(
        &mut self,
        num_prox: usize,
        positions: &mut Option<impl DataInput>,
        offsets: &mut Option<impl DataInput>,
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
        positions: &mut Option<impl DataInput>,
        offsets: &mut Option<impl DataInput>,
    ) -> Result<()> {
        match self {
            TermVectorsWriterEnum::Lucene90(writer) => {
                writer.default_add_prox(num_prox, positions, offsets)
            },
        }
    }
}

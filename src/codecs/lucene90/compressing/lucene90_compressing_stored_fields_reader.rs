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
use crate::codecs::compressing::lucene90_compressing_stored_fields_writer::Lucene90CompressingStoredFieldsWriter;
use crate::codecs::compressing::stored_fields_ints::StoredFieldsInts;
use crate::codecs::compression::compression_mode::DecompressorEnum;
use crate::codecs::compression::decompressor::Decompressor;
use crate::index::BytesRef;
use crate::store::dummy::dummy_index_input::DummyIndexInput;
use crate::store::{ByteArrayDataInput, DataInput, IndexInput};
use crate::util::array_util::ArrayUtil;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{CommonUtil, SliceCopyOps};
use byteorder::ReadBytesExt;
use std::cell::RefCell;
use std::cmp::min;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

pub struct Lucene90CompressingStoredFieldsReader<I>
where
    I: IndexInput,
{
    fields_stream: Rc<RefCell<I>>,
}
impl Lucene90CompressingStoredFieldsReader<DummyIndexInput> {
    /// Reads a float in a variable-length format. Reads between one and five bytes.
    /// Small integral values typically take fewer bytes.
    pub fn read_zfloat(input: &mut impl DataInput) -> Result<f32> {
        let b = input.read_byte()? as i32;
        if b == 0xFF {
            // negative value
            let bits = input.read_int()? as u32;
            Ok(f32::from_bits(bits))
        } else if (b & 0x80) != 0 {
            // small integer [-1..125]
            Ok(((b & 0x7F) - 1) as f32)
        } else {
            // positive float
            let high = b  << 24;
            let mid = (input.read_short()? as u16 as i32) << 8;
            let low = input.read_byte()? as i32;
            let bits = high | mid | low;
            Ok(f32::from_bits(bits as u32))
        }
    }
    /// Reads a double in a variable-length format. Reads between one and nine bytes.
    /// Small integral values typically take fewer bytes.
    pub fn read_zdouble(input: &mut impl DataInput) -> Result<f64> {
        let b = input.read_byte()? as i32;
        if b == 0xFF {
            // negative value (full i64 bits)
            let bits = input.read_long()? as u64;
            Ok(f64::from_bits(bits))
        } else if b == 0xFE {
            // float encoded as f32
            let bits = input.read_int()? as u32;
            Ok(f32::from_bits(bits) as f64)
        } else if (b & 0x80) != 0 {
            // small integer [-1..124]
            Ok(((b & 0x7F) - 1) as f64)
        } else {
            // positive double
            let high = (b as u64) << 56;
            let mid1 = (input.read_int()? as u32 as u64) << 24;
            let mid2 = (input.read_short()? as u16 as u64) << 8;
            let low = input.read_byte()? as u64;
            let bits = high | mid1 | mid2 | low;
            Ok(f64::from_bits(bits))
        }
    }
    /// Reads a long in a variable-length format. Reads between one and nine bytes.
    /// Small values typically take fewer bytes.
    pub fn read_tlong(input: &mut impl DataInput) -> Result<i64> {
        let header = input.read_byte()? as i32;

        let mut bits = (header & 0x1F) as i64;
        if (header & 0x20) != 0 {
            // continuation bit is set
            bits |= input.read_vlong()? << 5;
        }

        let mut l = BitUtil::zig_zag_decode_i64(bits as u64);

        match header & Lucene90CompressingStoredFieldsWriter::DAY_ENCODING {
            Lucene90CompressingStoredFieldsWriter::SECOND_ENCODING => {
                l *= Lucene90CompressingStoredFieldsWriter::SECOND
            }
            Lucene90CompressingStoredFieldsWriter::HOUR_ENCODING => {
                l *= Lucene90CompressingStoredFieldsWriter::HOUR
            }
            Lucene90CompressingStoredFieldsWriter::DAY_ENCODING => {
                l *= Lucene90CompressingStoredFieldsWriter::DAY
            }
            0 => {}
            _ => {
                debug_assert!(false, "should not be here");
                return Err(LuceneError::unreachable("invalid tlong encoding"));
            }
        }

        Ok(l)
    }
}

impl<I> Lucene90CompressingStoredFieldsReader<I>
where
    I: IndexInput,
{
    // -0 isn't compressed.
    const NEGATIVE_ZERO_FLOAT: u32 = (-0f32).to_bits();
    const NEGATIVE_ZERO_DOUBLE: u64 = (-0f64).to_bits();

    // for compression of timestamps
    const SECOND: i64 = 1_000;
    const HOUR: i64 = 60 * 60 * Self::SECOND;
    const DAY: i64 = 24 * Self::HOUR;

    const SECOND_ENCODING: u8 = 0x40;
    const HOUR_ENCODING: u8 = 0x80;
    const DAY_ENCODING: u8 = 0xC0;
}

/// Keeps state about the current block of documents.
struct BlockState<I>
where
    I: IndexInput,
{
    doc_base: i32,
    chunk_docs: i32,
    /// Whether the block has been sliced, this happens for large documents.
    sliced: bool,
    offsets: Vec<i64>,
    num_stored_fields: Vec<i64>,
    start_pointer: i64,
    spare: Option<BytesRef>,
    bytes: Option<BytesRef>,
    merging: bool,
    fields_stream: Rc<RefCell<I>>,
    decompressor: DecompressorEnum,
    chunk_size: i32,
}
impl<I> BlockState<I>
where
    I: IndexInput,
{
    /// Creates a new `BlockState` with default values.
    fn new(
        merging: bool,
        fields_stream: Rc<RefCell<I>>,
        decompressor: DecompressorEnum,
        chunk_size: i32,
    ) -> Self {
        let (spare, bytes) = if merging {
            (Some(BytesRef::new()), Some(BytesRef::new()))
        } else {
            (None, None)
        };

        BlockState {
            doc_base: 0,
            chunk_docs: 0,
            sliced: false,
            offsets: Vec::new(),
            num_stored_fields: Vec::new(),
            start_pointer: 0,
            spare,
            bytes,
            merging,
            fields_stream,
            decompressor,
            chunk_size,
        }
    }

    fn contains(&self, doc_id: i32) -> bool {
        doc_id >= self.doc_base && doc_id < self.doc_base + self.chunk_docs
    }
    /// Reset this block so that it stores state for the block that contains the given doc id.
    fn reset(&mut self, doc_id: i32, num_docs: i32) -> Result<()> {
        let result: Result<()> = (|| {
            self.do_reset(doc_id, num_docs)?;
            Ok(())
        })();

        if result.is_err() {
            // if the read failed, set chunkDocs to 0 so that it does not
            // contain any docs anymore and is not reused. This should help
            // get consistent exceptions when trying to get several
            // documents which are in the same corrupted block since it will
            // force the header to be decoded again
            self.chunk_docs = 0;
        }
        Ok(())
    }

    fn do_reset(&mut self, doc_id: i32, num_docs: i32) -> Result<()> {
        let mut stream = self.fields_stream.borrow_mut();

        self.doc_base = stream.read_vint()?;
        let token = stream.read_vint()?;
        self.chunk_docs = ((token as u32) >> 2) as i32;

        if !self.contains(doc_id) || self.doc_base + self.chunk_docs > num_docs {
            return Err(LuceneError::corrupt_index(format!(
                "Corrupted: docID={}, docBase={}, chunkDocs={}, numDocs={} (resource={})",
                doc_id, self.doc_base, self.chunk_docs, num_docs, stream
            )));
        }

        self.sliced = (token & 1) != 0;

        ArrayUtil::grow_no_copy(&mut self.offsets, self.chunk_docs + 1)?;
        ArrayUtil::grow_no_copy(&mut self.num_stored_fields, self.chunk_docs)?;

        if self.chunk_docs == 1 {
            self.num_stored_fields[0] = stream.read_vint()? as i64;
            self.offsets[1] = stream.read_vint()? as i64;
        } else {
            // Number of stored fields per document
            StoredFieldsInts::read_ints(
                &mut *stream,
                self.chunk_docs,
                &mut self.num_stored_fields,
                0,
            )?;
            // The stream encodes the length of each document and we decode
            // it into a list of monotonically increasing offsets
            StoredFieldsInts::read_ints(&mut *stream, self.chunk_docs, &mut self.offsets, 1)?;

            for i in 0..self.chunk_docs as usize {
                self.offsets[i + 1] += self.offsets[i];
            }
            // Additional validation: only the empty document has a serialized length of 0
            for i in 0..self.chunk_docs as usize {
                let len = self.offsets[i + 1] - self.offsets[i];
                let stored_fields = self.num_stored_fields[i];
                if (len == 0) != (stored_fields == 0) {
                    return Err(LuceneError::corrupt_index(format!(
                        "length={}, numStoredFields={} (resource={})",
                        len, stored_fields, stream
                    )));
                }
            }
        }

        self.start_pointer = stream.get_file_pointer();

        if self.merging {
            let total_length =
                i32::try_from(self.offsets[self.chunk_docs as usize]).map_err(|_| {
                    LuceneError::integer_overflow(format!(
                        "too large: {}",
                        self.offsets[self.chunk_docs as usize]
                    ))
                })?;
            // decompress eagerly
            if self.sliced {
                if let (Some(spare), Some(bytes)) = (&mut self.spare, &mut self.bytes) {
                    bytes.offset = 0;
                    bytes.length = 0;

                    let mut decompressed = 0;
                    while decompressed < total_length {
                        let to_decompress = min(total_length - decompressed, self.chunk_size);
                        self.decompressor.decompress(
                            &mut *stream,
                            to_decompress,
                            0,
                            to_decompress,
                            spare,
                        )?;

                        let new_len = bytes.length + spare.length;
                        ArrayUtil::grow_with_len(&mut bytes.bytes, new_len)?;
                        bytes.bytes.copy_from(
                            &spare.bytes
                                [spare.offset as usize..(spare.offset + spare.length) as usize],
                            bytes.length as usize,
                        );
                        bytes.length = new_len;
                        decompressed += to_decompress;
                    }
                }
            } else if let Some(bytes) = &mut self.bytes {
                self.decompressor
                    .decompress(&mut *stream, total_length, 0, total_length, bytes)?;
                if bytes.length != total_length {
                    return Err(LuceneError::corrupt_index(format!(
                        "Corrupted: expected chunk size = {}, got {} (resource={})",
                        total_length, bytes.length, stream
                    )));
                }
            }
        }
        Ok(())
    }
    /// Get the serialized representation of the given docID.
    /// This docID has to be contained in the current block.
    pub fn document(&mut self, doc_id: i32) -> Result<SerializedDocument<I>> {
        if !self.contains(doc_id) {
            return Err(LuceneError::illegal_argument(""));
        }

        let index = (doc_id - self.doc_base) as usize;
        let offset = i32::try_from(self.offsets[index]).map_err(|_| {
            LuceneError::integer_overflow(format!("offset too large: {}", self.offsets[index]))
        })?;
        let length =
            i32::try_from(self.offsets[index + 1] - self.offsets[index]).map_err(|_| {
                LuceneError::integer_overflow(format!(
                    "length too large: {}",
                    self.offsets[index + 1] - self.offsets[index]
                ))
            })?;
        let total_length = i32::try_from(self.offsets[self.chunk_docs as usize]).map_err(|_| {
            LuceneError::integer_overflow(format!(
                "totalLength too large: {}",
                self.offsets[self.chunk_docs as usize]
            ))
        })?;
        let num_stored_fields = i32::try_from(self.num_stored_fields[index]).map_err(|_| {
            LuceneError::integer_overflow(format!(
                "numStoredFields too large: {}",
                self.num_stored_fields[index]
            ))
        })?;

        let mut bytes = if self.merging {
            match self.bytes {
                Some(ref mut bytes) => CommonUtil::take_and_reset(bytes, |bytes| {
                    let vec = vec![0; bytes.bytes.len()];
                    BytesRef::from_vec(vec, 0, 0)
                }),
                None => {
                    return Err(LuceneError::illegal_state(
                        "bytes is None, but merging is true",
                    ))
                }
            }
        } else {
            BytesRef::new()
        };

        let document_input = if length == 0 {
            DataInputEnum::ByteArray(ByteArrayDataInput::new())
        } else if self.merging {
            DataInputEnum::ByteArray(ByteArrayDataInput::with_range(
                std::mem::take(&mut bytes.bytes),
                bytes.offset + offset,
                length,
            ))
        } else {
            let mut stream = self.fields_stream.borrow_mut();
            stream.seek(self.start_pointer)?;

            if self.sliced {
                self.decompressor.decompress(
                    &mut *stream,
                    self.chunk_size,
                    offset,
                    min(length, self.chunk_size - offset),
                    &mut bytes,
                )?;
                DataInputEnum::Impl(DataInputImpl::new(
                    &mut self.decompressor,
                    self.chunk_size,
                    Rc::clone(&self.fields_stream),
                    bytes,
                    length,
                ))
            } else {
                self.decompressor.decompress(
                    &mut *stream,
                    total_length,
                    offset,
                    length,
                    &mut bytes,
                )?;
                debug_assert_eq!(bytes.length, length);
                DataInputEnum::ByteArray(ByteArrayDataInput::with_range(
                    std::mem::take(&mut bytes.bytes),
                    bytes.offset,
                    bytes.length,
                ))
            }
        };

        Ok(SerializedDocument::new(
            document_input,
            length,
            num_stored_fields,
        ))
    }
}

/// A serialized document. You need to decode its input to get an actual `Document`.
pub struct SerializedDocument<'a, I>
where
    I: IndexInput,
{
    /// The serialized data input.
    input: DataInputEnum<'a, I>,

    /// The number of bytes on which the document is encoded.
    length: i32,

    /// The number of stored fields in the document.
    num_stored_fields: i32,
}

impl<'a, I> SerializedDocument<'a, I>
where
    I: IndexInput,
{
    pub fn new(input: DataInputEnum<'a, I>, length: i32, num_stored_fields: i32) -> Self {
        SerializedDocument {
            input,
            length,
            num_stored_fields,
        }
    }
}

struct DataInputImpl<'a, I>
where
    I: IndexInput,
{
    decompressed: i32,
    length: i32,
    decompressor: &'a mut DecompressorEnum,
    chunk_size: i32,
    fields_stream: Rc<RefCell<I>>,
    bytes: BytesRef,
}
impl<'a, I> DataInputImpl<'a, I>
where
    I: IndexInput,
{
    fn new(
        decompressor: &'a mut DecompressorEnum,
        chunk_size: i32,
        fields_stream: Rc<RefCell<I>>,
        bytes: BytesRef,
        length: i32,
    ) -> Self {
        let decompressed = bytes.length;
        DataInputImpl {
            decompressed,
            length,
            decompressor,
            chunk_size,
            fields_stream,
            bytes,
        }
    }
    fn fill_buffer(&mut self) -> Result<()> {
        debug_assert!(self.decompressed <= self.length);

        if self.decompressed == self.length {
            return Err(LuceneError::eof(""));
        }

        let to_decompress = std::cmp::min(self.length - self.decompressed, self.chunk_size);
        self.decompressor.decompress(
            &mut *self.fields_stream.borrow_mut(),
            to_decompress,
            0,
            to_decompress,
            &mut self.bytes,
        )?;
        self.decompressed += to_decompress;
        Ok(())
    }
}

impl<I> Display for DataInputImpl<'_, I>
where
    I: IndexInput,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DataInputImpl in Lucene90CompressingStoredFieldsReader")
    }
}

impl<I> DataInput for DataInputImpl<'_, I>
where
    I: IndexInput,
{
    fn read_byte(&mut self) -> Result<u8> {
        if self.bytes.length == 0 {
            self.fill_buffer()?;
        }
        self.bytes.length -= 1;
        let b = self.bytes.bytes[self.bytes.offset as usize];
        self.bytes.offset += 1;
        Ok(b)
    }

    fn read_bytes(&mut self, b: &mut [u8], mut offset: i32, mut len: i32) -> Result<()> {
        while len > self.bytes.length {
            b.copy_from(
                &self.bytes.bytes
                    [self.bytes.offset as usize..(self.bytes.offset + self.bytes.length) as usize],
                offset as usize,
            );
            len -= self.bytes.length;
            offset += self.bytes.length;
            self.fill_buffer()?;
        }
        b.copy_from(
            &self.bytes.bytes[self.bytes.offset as usize..(self.bytes.offset + len) as usize],
            len as usize,
        );
        self.bytes.offset += len;
        self.bytes.length -= len;
        Ok(())
    }

    fn skip_bytes(&mut self, mut num_bytes: i64) -> Result<()> {
        if num_bytes < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "num_bytes must be >= 0, got {}",
                num_bytes
            )));
        }

        while num_bytes > self.bytes.length as i64 {
            num_bytes -= self.bytes.length as i64;
            self.fill_buffer()?;
        }
        let num_bytes = i32::try_from(num_bytes)
            .map_err(|_| LuceneError::integer_overflow(format!("too large: {}", num_bytes)))?;
        self.bytes.offset += num_bytes;
        self.bytes.length -= num_bytes;
        Ok(())
    }
}
enum DataInputEnum<'a, I>
where
    I: IndexInput,
{
    ByteArray(ByteArrayDataInput),
    Impl(DataInputImpl<'a, I>),
}

impl<I> Display for DataInputEnum<'_, I>
where
    I: IndexInput,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DataInputEnum::ByteArray(data_input) => write!(f, "{}", data_input),
            DataInputEnum::Impl(data_input) => write!(f, "{}", data_input),
        }
    }
}

impl<I> DataInput for DataInputEnum<'_, I>
where
    I: IndexInput,
{
    fn read_byte(&mut self) -> Result<u8> {
        match self {
            DataInputEnum::ByteArray(data_input) => data_input.read_byte(),
            DataInputEnum::Impl(data_input) => data_input.read_byte(),
        }
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<()> {
        match self {
            DataInputEnum::ByteArray(data_input) => data_input.read_bytes(b, offset, len),
            DataInputEnum::Impl(data_input) => data_input.read_bytes(b, offset, len),
        }
    }

    fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
        match self {
            DataInputEnum::ByteArray(data_input) => data_input.skip_bytes(num_bytes),
            DataInputEnum::Impl(data_input) => data_input.skip_bytes(num_bytes),
        }
    }
}

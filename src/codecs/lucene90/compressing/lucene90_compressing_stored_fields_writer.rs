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
use crate::codecs::compressing::lucene90_compressing_stored_fields_reader::Lucene90CompressingStoredFieldsReader;
use crate::codecs::compressing::stored_fields_ints::StoredFieldsInts;
use crate::codecs::compression::compression_mode::{
    CompressionModeBase, CompressionModeEnum, CompressorEnum,
};
use crate::codecs::compression::compressor::Compressor;
use crate::codecs::compression::matching_readers::MatchingReaders;
use crate::codecs::lucene90::fields_index::FieldsIndex;
use crate::codecs::lucene90::fields_index_writer::FieldsIndexWriter;
use crate::codecs::stored_fields_reader::{StoredFieldsReader, StoredFieldsReaderEnum};
use crate::codecs::stored_fields_writer::{MergeVisitor, StoredFieldsWriter};
use crate::codecs::CodecUtil;
use crate::index::field_info::FieldInfo;
use crate::index::merge_state::{DocMapEnum, MergeState};
use crate::index::segment_info::SegmentInfo;
use crate::index::stored_fields::StoredFields;
use crate::index::{doc_id_merger_util, BytesRef, DocIDMerger, IndexFileNames, Sub, SubBase};
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::store::directory::Directory;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{
    ByteBuffersDataOutput, DataInput, DataOutput, IOContext, IndexInput, IndexOutput,
};
use crate::util::accountable::Accountable;
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::packed::PackedInts;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::env;
use std::mem::discriminant;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
/// [`StoredFieldsWriter`] implementation for [`Lucene90CompressingStoredFieldsFormat`](crate::codecs::lucene90::compressing::lucene90_compressing_stored_fields_format::Lucene90CompressingStoredFieldsFormat).
pub(crate) static TYPE_BITS: Lazy<i32> =
    Lazy::new(|| PackedInts::bits_required(lucene90_csfw_util::NUMERIC_DOUBLE as i64).unwrap());

pub(crate) static TYPE_MASK: Lazy<i64> = Lazy::new(|| PackedInts::max_value(*TYPE_BITS));
pub struct Lucene90CompressingStoredFieldsWriter<D>
where
    D: Directory,
{
    segment: String,
    index_writer: FieldsIndexWriter<D>,
    meta_stream: D::IndexOutputType,
    fields_stream: D::IndexOutputType,
    compressor: CompressorEnum,
    compression_mode: CompressionModeEnum,
    chunk_size: i32,
    max_docs_per_chunk: i32,
    buffered_docs: ByteBuffersDataOutput,
    num_stored_fields: Vec<i32>,
    end_offsets: Vec<i32>,
    doc_base: i32,
    num_buffered_docs: i32,
    num_chunks: i64,
    num_dirty_chunks: i64,
    num_dirty_docs: i64,
    num_stored_fields_in_doc: i32,
}
pub mod lucene90_csfw_util {
    use crate::store::DataOutput;
    use crate::util::bit_util::BitUtil;

    /// Extension of stored fields file
    pub(crate) const FIELDS_EXTENSION: &str = "fdt";
    /// Extension of stored fields index
    pub(crate) const INDEX_EXTENSION: &str = "fdx";
    /// Extension of stored fields meta
    pub(crate) const META_EXTENSION: &str = "fdm";
    /// Codec name for the index
    pub(crate) const INDEX_CODEC_NAME: &str = "Lucene90FieldsIndex";
    pub(crate) const STRING: i32 = 0x00;
    pub(crate) const BYTE_ARR: i32 = 0x01;
    pub(crate) const NUMERIC_INT: i32 = 0x02;
    pub(crate) const NUMERIC_FLOAT: i32 = 0x03;
    pub(crate) const NUMERIC_LONG: i32 = 0x04;
    pub(crate) const NUMERIC_DOUBLE: i32 = 0x05;

    pub(crate) const VERSION_START: i32 = 1;
    pub(crate) const VERSION_CURRENT: i32 = VERSION_START;
    pub(crate) const META_VERSION_START: i32 = 0;

    // -0 isn't compressed.
    pub(crate) const NEGATIVE_ZERO_FLOAT: u32 = (-0f32).to_bits();
    pub(crate) const NEGATIVE_ZERO_DOUBLE: u64 = (-0f64).to_bits();

    // for compression of timestamps
    pub(crate) const SECOND: i64 = 1_000;
    pub(crate) const HOUR: i64 = 60 * 60 * SECOND;
    pub(crate) const DAY: i64 = 24 * HOUR;

    pub(crate) const SECOND_ENCODING: i32 = 0x40;
    pub(crate) const HOUR_ENCODING: i32 = 0x80;
    pub(crate) const DAY_ENCODING: i32 = 0xC0;

    /// Writes a float in a variable-length format. Writes between one and five bytes.
    /// Small integral values typically take fewer bytes.
    ///
    /// ZFloat --> Header, Bytes*?
    ///
    /// - Header --> [`DataOutput::write_byte`](crate::store::data_output::DataOutput::write_byte) (Uint8). When it is equal to `0xFF` then the value
    ///   is negative and stored in the next 4 bytes. Otherwise, if the first bit is set, then the
    ///   other bits in the header encode the value plus one and no other bytes are read.
    ///   Otherwise, the value is a positive float value whose first byte is the header, and 3
    ///   bytes need to be read to complete it.
    /// - Bytes --> Potential additional bytes to read depending on the header.
    pub(crate) fn write_zfloat(
        out: &mut impl DataOutput,
        f: f32,
    ) -> crate::util::error::lucene_error::Result<()> {
        let int_val = f as i32;
        let float_bits = f.to_bits();

        if f == int_val as f32
            && (-1..=0x7D).contains(&int_val)
            && float_bits != NEGATIVE_ZERO_FLOAT
        {
            // small integer [-1..125]: single byte
            out.write_byte((0x80 | (1 + int_val)) as u8)?;
        } else if (float_bits >> 31) == 0 {
            // other positive floats: 4 bytes
            out.write_byte((float_bits >> 24) as u8)?;
            out.write_short((float_bits >> 8) as i16)?;
            out.write_byte(float_bits as u8)?;
        } else {
            // negative float or special: 5 bytes
            out.write_byte(0xFFu8)?;
            out.write_int(float_bits as i32)?;
        }
        Ok(())
    }
    /// Writes a float in a variable-length format. Writes between one and five bytes.
    /// Small integral values typically take fewer bytes.
    ///
    /// ZFloat --> Header, Bytes*?
    ///
    /// - Header --> [`DataOutput::write_byte`](crate::store::data_output::DataOutput::write_byte) (Uint8). When it is equal to `0xFF` then the value
    ///   is negative and stored in the next 8 bytes. When it is equal to `0xFE` then the value is
    ///   stored as a float in the next 4 bytes. Otherwise if the first bit is set then the other
    ///   bits in the header encode the value plus one and no other bytes are read. Otherwise, the
    ///   value is a positive float value whose first byte is the header, and 7 bytes need to be
    ///   read to complete it.
    /// - Bytes --> Potential additional bytes to read depending on the header.
    pub(crate) fn write_zdouble(
        out: &mut impl DataOutput,
        d: f64,
    ) -> crate::util::error::lucene_error::Result<()> {
        let int_val = d as i32;
        let double_bits = d.to_bits(); // u64

        if d == int_val as f64
            && (-1..=0x7C).contains(&int_val)
            && double_bits != NEGATIVE_ZERO_DOUBLE
        {
            // small integer value [-1..124]: single byte
            out.write_byte((0x80 | (int_val + 1)) as u8)?;
        } else if d == (d as f32) as f64 {
            // d has an accurate float representation: 5 bytes
            out.write_byte(0xFE)?;
            out.write_int((d as f32).to_bits() as i32)?;
        } else if (double_bits >> 63) == 0 {
            // other positive doubles: 8 bytes
            out.write_byte((double_bits >> 56) as u8)?;
            out.write_int((double_bits >> 24) as u32 as i32)?; // lower 32 bits as i32
            out.write_short((double_bits >> 8) as i16)?;
            out.write_byte(double_bits as u8)?;
        } else {
            // other negative doubles: 9 bytes
            out.write_byte(0xFF)?;
            out.write_long(double_bits as i64)?;
        }
        Ok(())
    }
    /// Writes a long in a variable-length format. Writes between one and ten bytes.
    /// Small values or values representing timestamps with day, hour or second precision
    /// typically require fewer bytes.
    ///
    /// ZLong --> Header, Bytes*?
    ///
    /// - Header --> The first two bits indicate the compression scheme:
    ///   - 00 - uncompressed
    ///   - 01 - multiple of 1000 (second)
    ///   - 10 - multiple of 3600000 (hour)
    ///   - 11 - multiple of 86400000 (day)
    ///
    ///   Then the next bit is a continuation bit, indicating whether more bytes need to be read,
    ///   and the last 5 bits are the lower bits of the encoded value. In order to reconstruct the
    ///   value, you need to combine the 5 lower bits of the header with a vLong in the next bytes
    ///   (if the continuation bit is set to 1). Then [`BitUtil::zig_zag_decode`](BitUtil::zig_zag_decode_i64) it and finally
    ///   multiply by the multiple corresponding to the compression scheme.
    ///
    /// - Bytes --> Potential additional bytes to read depending on the header.
    // T for "timestamp"
    pub(crate) fn write_tlong(
        out: &mut impl DataOutput,
        mut l: i64,
    ) -> crate::util::error::lucene_error::Result<()> {
        let mut header;

        if l % SECOND != 0 {
            header = 0;
        } else if l % DAY == 0 {
            // timestamp with day precision
            header = DAY_ENCODING;
            l /= DAY;
        } else if l % HOUR == 0 {
            // timestamp with hour precision, or day precision with a timezone
            header = HOUR_ENCODING;
            l /= HOUR;
        } else {
            // timestamp with second precision
            header = SECOND_ENCODING;
            l /= SECOND;
        }

        let zigzag_l = BitUtil::zig_zag_encode_i64(l);
        header |= (zigzag_l & 0x1F) as i32; // last 5 bits

        let upper_bits = ((zigzag_l as u64) >> 5) as i64;
        if upper_bits != 0 {
            header |= 0x20;
        }

        out.write_byte(header as u8)?;

        if upper_bits != 0 {
            out.write_vlong(upper_bits)?;
        }
        Ok(())
    }
}
impl<D> Lucene90CompressingStoredFieldsWriter<D>
where
    D: Directory,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        directory: Arc<Mutex<D>>,
        si: Rc<SegmentInfo<D>>,
        segment_suffix: &str,
        context: &IOContext,
        format_name: &str,
        compression_mode: CompressionModeEnum,
        chunk_size: i32,
        max_docs_per_chunk: i32,
        block_shift: i32,
    ) -> Result<Self> {
        let segment = si.name.clone();
        let compressor = compression_mode.new_compressor();
        let buffered_docs = ByteBuffersDataOutput::with_resettable_instance();

        let meta_file = IndexFileNames::segment_file_name(
            &segment,
            segment_suffix,
            lucene90_csfw_util::META_EXTENSION,
        );
        let mut meta_stream;
        let mut fields_stream;
        {
            let mut dir = directory
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire  lock.".to_string()))?;
            meta_stream = dir.create_output(&meta_file, context)?;

            let fields_file = IndexFileNames::segment_file_name(
                &segment,
                segment_suffix,
                lucene90_csfw_util::FIELDS_EXTENSION,
            );
            fields_stream = dir.create_output(&fields_file, context)?;
        }

        let index_writer = FieldsIndexWriter::new(
            directory,
            &segment,
            segment_suffix,
            lucene90_csfw_util::INDEX_EXTENSION,
            lucene90_csfw_util::INDEX_CODEC_NAME,
            si.get_id().clone(),
            block_shift,
            context.clone(),
        )?;

        CodecUtil::write_index_header(
            &mut meta_stream,
            &format!("{}Meta", lucene90_csfw_util::INDEX_CODEC_NAME),
            lucene90_csfw_util::VERSION_CURRENT,
            &si.get_id(),
            segment_suffix,
        )?;

        debug_assert_eq!(
            CodecUtil::index_header_length(
                &format!("{}Meta", lucene90_csfw_util::INDEX_CODEC_NAME),
                segment_suffix
            ) as i64,
            meta_stream.get_file_pointer()
        );

        CodecUtil::write_index_header(
            &mut fields_stream,
            format_name,
            lucene90_csfw_util::VERSION_CURRENT,
            &si.get_id(),
            segment_suffix,
        )?;

        debug_assert_eq!(
            CodecUtil::index_header_length(format_name, segment_suffix) as i64,
            fields_stream.get_file_pointer()
        );

        meta_stream.write_vint(chunk_size)?;

        Ok(Self {
            segment,
            compression_mode,
            compressor,
            chunk_size,
            max_docs_per_chunk,
            buffered_docs,
            num_stored_fields: vec![0; 16],
            end_offsets: vec![0; 16],
            doc_base: 0,
            num_buffered_docs: 0,
            num_chunks: 0,
            num_dirty_chunks: 0,
            num_dirty_docs: 0,
            meta_stream,
            fields_stream,
            index_writer,
            num_stored_fields_in_doc: 0,
        })
    }

    fn save_ints(values: &[i32], length: i32, out: &mut impl DataOutput) -> Result<()> {
        if length == 1 {
            out.write_vint(values[0])?;
        } else {
            StoredFieldsInts::write_ints(values, 0, length, out)?;
        }
        Ok(())
    }
    fn write_header(&mut self, sliced: bool, dirty_chunk: bool) -> Result<()> {
        let sliced_bit = if sliced { 1 } else { 0 };
        let dirty_bit = if dirty_chunk { 2 } else { 0 };

        self.fields_stream.write_vint(self.doc_base)?;
        self.fields_stream
            .write_vint((self.num_buffered_docs << 2) | dirty_bit | sliced_bit)?;
        // save numStoredFields
        Self::save_ints(
            &self.num_stored_fields,
            self.num_buffered_docs,
            &mut self.fields_stream,
        )?;
        // save lengths
        Self::save_ints(
            &self.end_offsets,
            self.num_buffered_docs,
            &mut self.fields_stream,
        )?;
        Ok(())
    }
    fn trigger_flush(&self) -> bool {
        self.buffered_docs.size() as i32 >= self.chunk_size
            || self.num_buffered_docs >= self.max_docs_per_chunk
    }
    fn flush(&mut self, force: bool) -> Result<()> {
        debug_assert!(self.trigger_flush() != force);

        self.num_chunks += 1;
        if force {
            self.num_dirty_chunks += 1;
            self.num_dirty_docs += self.num_buffered_docs as i64;
        }

        self.index_writer.write_index(
            self.num_buffered_docs,
            self.fields_stream.get_file_pointer(),
        )?;

        // convert end offsets into lengths
        for i in (1..self.num_buffered_docs as usize).rev() {
            self.end_offsets[i] -= self.end_offsets[i - 1];
            debug_assert!(self.end_offsets[i] >= 0);
        }

        let sliced = self.buffered_docs.size() >= 2 * self.chunk_size as i64;
        let dirty_chunk = force;

        self.write_header(sliced, dirty_chunk)?;
        let mut byte_buffers = self.buffered_docs.get_data_input();
        // compress stored fields to fieldsStream.
        if sliced {
            let capacity = byte_buffers.length() as i32;
            let mut compressed = 0;
            while compressed < capacity {
                let len = std::cmp::min(self.chunk_size, capacity - compressed);
                let mut bbdi = byte_buffers.slice(compressed as i64, len as i64)?;
                self.compressor
                    .compress(&mut bbdi, &mut self.fields_stream)?;
                compressed += len;
            }
        } else {
            self.compressor
                .compress(&mut byte_buffers, &mut self.fields_stream)?;
        }

        // reset
        self.doc_base += self.num_buffered_docs;
        self.num_buffered_docs = 0;
        self.buffered_docs.reset();

        Ok(())
    }

    fn copy_one_doc<I>(
        &mut self,
        reader: &mut Lucene90CompressingStoredFieldsReader<I>,
        doc_id: i32,
    ) -> Result<()>
    where
        I: IndexInput,
    {
        debug_assert_eq!(reader.get_version(), lucene90_csfw_util::VERSION_CURRENT);

        let mut doc = reader.serialized_document(doc_id)?;

        self.start_document()?;

        self.buffered_docs
            .copy_bytes(&mut doc.input, doc.length as i64)?;

        self.num_stored_fields_in_doc = doc.num_stored_fields;
        self.finish_document()?;

        Ok(())
    }
    fn copy_chunks<I: IndexInput>(
        &mut self,
        merge_state: &mut MergeState<I>,
        sub: &CompressingStoredFieldsMergeSub,
        from_doc_id: i32,
        to_doc_id: i32,
    ) -> Result<()> {
        let StoredFieldsReaderEnum::Lucene90(reader) =
            &mut merge_state.stored_fields_readers[sub.reader_index];

        debug_assert_eq!(reader.get_version(), lucene90_csfw_util::VERSION_CURRENT);
        debug_assert_eq!(reader.get_chunk_size(), self.chunk_size);
        debug_assert!(
            discriminant(reader.get_compression_mode()) == discriminant(&self.compression_mode)
        );
        debug_assert!(!self.too_dirty(reader)?);
        debug_assert!(merge_state.live_docs[sub.reader_index].is_none());

        let mut doc_id = from_doc_id;
        let max_pointer = reader.get_max_pointer();

        // copy docs that belong to the previous chunk
        while doc_id < to_doc_id && reader.is_loaded(doc_id)? {
            self.copy_one_doc(reader, doc_id)?;
            doc_id += 1;
        }
        let stream = reader.get_fields_stream();
        let mut raw_docs = stream.borrow_mut();
        let index = reader.get_index_reader();

        if doc_id >= to_doc_id {
            return Ok(());
        }

        let mut from_pointer = index.get_start_pointer(doc_id)?;
        let to_pointer = if to_doc_id == sub.max_doc {
            max_pointer
        } else {
            index.get_start_pointer(to_doc_id)?
        };

        if from_pointer < to_pointer {
            if self.num_buffered_docs > 0 {
                self.flush(true)?;
            }

            raw_docs.seek(from_pointer)?;

            while from_pointer < to_pointer {
                let base = raw_docs.read_vint()?;
                let code = raw_docs.read_vint()?;
                let buffered_docs = ((code as u32) >> 2) as i32;

                if base != doc_id {
                    return Err(LuceneError::corrupt_index(format!(
                        "invalid state: base={} != docID={} (resource {})",
                        base, doc_id, raw_docs
                    )));
                }
                // write a new index entry and new header for this chunk.
                self.index_writer
                    .write_index(buffered_docs, self.fields_stream.get_file_pointer())?;
                self.fields_stream.write_vint(self.doc_base)?;
                self.fields_stream.write_vint(code)?;
                doc_id += buffered_docs;
                self.doc_base += buffered_docs;
                if doc_id > to_doc_id {
                    return Err(LuceneError::corrupt_index(format!(
                        "invalid state: base={}, count={}, toDocID={} (resource {})",
                        base, buffered_docs, to_doc_id, raw_docs
                    )));
                }
                // copy bytes until the next chunk boundary (or end of chunk data).
                // using the stored fields index for this isn't the most efficient, but fast enough
                // and is a source of redundancy for detecting bad things.
                let end_chunk_pointer = if doc_id == sub.max_doc {
                    max_pointer
                } else {
                    index.get_start_pointer(doc_id)?
                };

                let num_bytes = end_chunk_pointer - raw_docs.get_file_pointer();
                self.fields_stream.copy_bytes(&mut *raw_docs, num_bytes)?;

                self.num_chunks += 1;

                if code & 2 != 0 {
                    debug_assert!(buffered_docs < self.max_docs_per_chunk);
                    self.num_dirty_chunks += 1;
                    self.num_dirty_docs += buffered_docs as i64;
                }
                from_pointer = end_chunk_pointer;
            }
        }

        // copy leftover docs that don't form a complete chunk
        debug_assert!(!reader.is_loaded(doc_id)?);
        while doc_id < to_doc_id {
            self.copy_one_doc(reader, doc_id)?;
            doc_id += 1;
        }
        Ok(())
    }
    /// Returns `true` if we should recompress this reader, even though we could bulk merge compressed data.
    ///
    /// The last chunk written for a segment is typically incomplete, so without recompressing,
    /// in some worst-case situations (e.g. frequent reopen with tiny flushes), over time the compression
    /// ratio can degrade. This is a safety switch.
    fn too_dirty<I>(&self, candidate: &Lucene90CompressingStoredFieldsReader<I>) -> Result<bool>
    where
        I: IndexInput,
    {
        // A segment is considered dirty only if it has enough dirty docs to make a full block
        // AND more than 1% blocks are dirty.
        Ok(
            candidate.get_num_dirty_docs()? > self.max_docs_per_chunk as i64
                && candidate.get_num_dirty_chunks()? * 100 > candidate.get_num_chunks()?,
        )
    }
    fn get_merge_strategy<I>(
        &self,
        merge_state: &MergeState<I>,
        matching_readers: &MatchingReaders,
        reader_index: usize,
    ) -> Result<MergeStrategy>
    where
        I: IndexInput,
    {
        let candidate = &merge_state.stored_fields_readers[reader_index];
        let (is_lucene90_compressing_stored_fields_reader, same_version) = match &candidate {
            StoredFieldsReaderEnum::Lucene90(reader) => {
                let version = reader.get_version();
                (false, version != lucene90_csfw_util::VERSION_CURRENT)
            }
            _ => (true, false),
        };
        if !matching_readers.matching_readers[reader_index]
            || is_lucene90_compressing_stored_fields_reader
            || same_version
        {
            return Ok(MergeStrategy::Visitor);
        }

        if let StoredFieldsReaderEnum::Lucene90(reader) = &candidate {
            return if *BULK_MERGE_ENABLED
                && discriminant(reader.get_compression_mode())
                    == discriminant(&self.compression_mode)
                && reader.get_chunk_size() == self.chunk_size
                && merge_state.live_docs[reader_index].is_none()
                && !self.too_dirty(reader)?
            {
                Ok(MergeStrategy::Bulk)
            } else {
                Ok(MergeStrategy::Doc)
            };
        }
        Err(LuceneError::illegal_state("should not be here"))
    }
}
pub static BULK_MERGE_ENABLED_SYSPROP: &str =
    "lucene90.compressing.stored.fields.writer.enableBulkMerge";

pub static BULK_MERGE_ENABLED: Lazy<bool> = Lazy::new(|| {
    env::var(BULK_MERGE_ENABLED_SYSPROP)
        .ok()
        .map(|v| v.parse::<bool>().unwrap_or(true))
        .unwrap_or(true)
});
impl<D> StoredFieldsWriter for Lucene90CompressingStoredFieldsWriter<D>
where
    D: Directory,
{
    fn start_document(&mut self) -> Result<()> {
        Ok(())
    }
    fn finish_document(&mut self) -> Result<()> {
        if self.num_buffered_docs as usize == self.num_stored_fields.len() {
            let new_len = ArrayUtil::oversize(self.num_buffered_docs + 1, 4);
            ArrayUtil::grow_exact(&mut self.num_stored_fields, new_len)?;
            ArrayUtil::grow_exact(&mut self.end_offsets, new_len)?;
        }

        self.num_stored_fields[self.num_buffered_docs as usize] = self.num_stored_fields_in_doc;
        self.num_stored_fields_in_doc = 0;

        let offset = i32::try_from(self.buffered_docs.size()).map_err(|_| {
            LuceneError::illegal_state(format!(
                "bufferedDocs size {} too large for i32",
                self.buffered_docs.size()
            ))
        })?;
        self.end_offsets[self.num_buffered_docs as usize] = offset;

        self.num_buffered_docs += 1;

        if self.trigger_flush() {
            self.flush(false)?;
        }

        Ok(())
    }

    fn write_field_i32(&mut self, info: &FieldInfo, value: i32) -> Result<()> {
        self.num_stored_fields_in_doc += 1;
        let info_and_bits =
            ((info.number as i64) << *TYPE_BITS) | lucene90_csfw_util::NUMERIC_INT as i64;
        self.buffered_docs.write_vlong(info_and_bits)?;
        self.buffered_docs.write_zint(value)?;
        Ok(())
    }

    fn write_field_i64(&mut self, info: &FieldInfo, value: i64) -> Result<()> {
        self.num_stored_fields_in_doc += 1;
        let info_and_bits =
            ((info.number as i64) << *TYPE_BITS) | lucene90_csfw_util::NUMERIC_LONG as i64;
        self.buffered_docs.write_vlong(info_and_bits)?;
        lucene90_csfw_util::write_tlong(&mut self.buffered_docs, value)?;
        Ok(())
    }

    fn write_field_f32(&mut self, info: &FieldInfo, value: f32) -> Result<()> {
        self.num_stored_fields_in_doc += 1;
        let info_and_bits =
            ((info.number as i64) << *TYPE_BITS) | lucene90_csfw_util::NUMERIC_FLOAT as i64;
        self.buffered_docs.write_vlong(info_and_bits)?;
        lucene90_csfw_util::write_zfloat(&mut self.buffered_docs, value)?;
        Ok(())
    }

    fn write_field_f64(&mut self, info: &FieldInfo, value: f64) -> Result<()> {
        self.num_stored_fields_in_doc += 1;
        let info_and_bits =
            ((info.number as i64) << *TYPE_BITS) | lucene90_csfw_util::NUMERIC_DOUBLE as i64;
        self.buffered_docs.write_vlong(info_and_bits)?;
        lucene90_csfw_util::write_zdouble(&mut self.buffered_docs, value)?;
        Ok(())
    }
    fn write_field_with_input(
        &mut self,
        info: &FieldInfo,
        value: &mut impl DataInput,
        length: i32,
    ) -> Result<()> {
        self.num_stored_fields_in_doc += 1;
        let info_and_bits =
            ((info.number as i64) << *TYPE_BITS) | lucene90_csfw_util::BYTE_ARR as i64;
        self.buffered_docs.write_vlong(info_and_bits)?;
        self.buffered_docs.write_vint(length)?;
        self.buffered_docs.copy_bytes(value, length as i64)?;
        Ok(())
    }

    fn write_field_bytes(&mut self, info: &FieldInfo, value: &BytesRef) -> Result<()> {
        self.num_stored_fields_in_doc += 1;
        let info_and_bits =
            ((info.number as i64) << *TYPE_BITS) | lucene90_csfw_util::BYTE_ARR as i64;
        self.buffered_docs.write_vlong(info_and_bits)?;
        self.buffered_docs.write_vint(value.length)?;
        self.buffered_docs
            .write_bytes_range(&value.bytes, value.offset, value.length)?;
        Ok(())
    }

    fn write_field_str(&mut self, info: &FieldInfo, value: &str) -> Result<()> {
        self.num_stored_fields_in_doc += 1;
        let info_and_bits =
            ((info.number as i64) << *TYPE_BITS) | lucene90_csfw_util::STRING as i64;
        self.buffered_docs.write_vlong(info_and_bits)?;
        self.buffered_docs.write_string(value)?;
        Ok(())
    }
    fn finish(&mut self, num_docs: i32) -> Result<()> {
        if self.num_buffered_docs > 0 {
            self.flush(true)?;
        } else {
            debug_assert_eq!(self.buffered_docs.size(), 0);
        }

        if self.doc_base != num_docs {
            return Err(LuceneError::illegal_state(format!(
                "Wrote {} docs, finish called with numDocs={}",
                self.doc_base, num_docs
            )));
        }

        self.index_writer.finish(
            num_docs,
            self.fields_stream.get_file_pointer(),
            &mut self.meta_stream,
        )?;

        self.meta_stream.write_vlong(self.num_chunks)?;
        self.meta_stream.write_vlong(self.num_dirty_chunks)?;
        self.meta_stream.write_vlong(self.num_dirty_docs)?;

        CodecUtil::write_footer(&mut self.meta_stream)?;
        CodecUtil::write_footer(&mut self.fields_stream)?;

        debug_assert_eq!(self.buffered_docs.size(), 0);

        Ok(())
    }

    fn merge<I>(&mut self, merge_state: &mut MergeState<I>) -> Result<i32>
    where
        I: IndexInput,
        Self: Sized,
    {
        let matching_readers = MatchingReaders::new(merge_state)?;
        let mut visitors: Vec<Option<MergeVisitor>> =
            vec![None; merge_state.stored_fields_readers.len()];
        let mut subs = Vec::with_capacity(merge_state.stored_fields_readers.len());

        for i in 0..merge_state.stored_fields_readers.len() {
            merge_state.stored_fields_readers[i].check_integrity()?;
            let strategy = self.get_merge_strategy(merge_state, &matching_readers, i)?;
            if strategy == MergeStrategy::Visitor {
                visitors[i] = Some(MergeVisitor::new(merge_state, i)?);
            }
            subs.push(Rc::new(RefCell::new(Sub::new(
                CompressingStoredFieldsMergeSub::new(merge_state, strategy, i),
            ))));
        }

        let mut doc_id_merger = doc_id_merger_util::of(subs, merge_state.needs_index_sort)?;
        let mut doc_count = 0;

        while let Some(sub_rc) = doc_id_merger.next()? {
            let sub = sub_rc.borrow_mut();
            debug_assert_eq!(sub.mapped_doc_id, doc_count);
            let reader = &mut merge_state.stored_fields_readers[sub.sub.reader_index];

            match sub.sub.merge_strategy {
                MergeStrategy::Bulk => {
                    let from_doc = sub.sub.doc_id;
                    let mut to_doc_id = from_doc;
                    let current = sub_rc.clone();

                    while let Some(next_sub_rc) = doc_id_merger.next()? {
                        if !Rc::ptr_eq(&current, &next_sub_rc) {
                            break;
                        }
                        to_doc_id += 1;
                        debug_assert!(next_sub_rc.borrow().sub.doc_id == to_doc_id)
                    }
                    to_doc_id += 1; // exclusive bound
                    self.copy_chunks(merge_state, &current.borrow().sub, from_doc, to_doc_id)?;
                    doc_count += to_doc_id - from_doc;
                }
                MergeStrategy::Doc => {
                    let lucene_90 = match reader {
                        StoredFieldsReaderEnum::Lucene90(reader) => reader,
                        _ => {
                            return Err(LuceneError::illegal_state(
                                "Invalid reader type".to_string(),
                            ))
                        }
                    };
                    self.copy_one_doc(lucene_90, sub.sub.doc_id)?;
                    doc_count += 1;
                }
                MergeStrategy::Visitor => {
                    debug_assert!(visitors[sub.sub.reader_index].is_some());
                    self.start_document()?;
                    match visitors[sub.sub.reader_index] {
                        Some(ref mut visitor) => {
                            reader.document_with_visitor(sub.sub.doc_id, visitor, self)?;
                            self.finish_document()?;
                            doc_count += 1;
                        }
                        None => {
                            return Err(LuceneError::illegal_state(
                                "Visitor must exist for VISITOR strategy".to_string(),
                            ))
                        }
                    }
                }
            }
        }

        self.finish(doc_count)?;
        Ok(doc_count)
    }
}

impl<D> Accountable for Lucene90CompressingStoredFieldsWriter<D>
where
    D: Directory,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        // TODO
        Ok(0)
    }
}
/// Merge strategy used during stored field merging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MergeStrategy {
    /// Copy chunk by chunk in a compressed format.
    Bulk,

    /// Copy document by document in a decompressed format.
    Doc,

    /// Copy field by field of decompressed documents.
    Visitor,
}
struct CompressingStoredFieldsMergeSub {
    pub reader_index: usize,
    pub max_doc: i32,
    pub merge_strategy: MergeStrategy,
    pub doc_id: i32,
    pub doc_map: Rc<DocMapEnum>,
}

impl CompressingStoredFieldsMergeSub {
    fn new<I: IndexInput>(
        merge_state: &MergeState<I>,
        merge_strategy: MergeStrategy,
        reader_index: usize,
    ) -> Self {
        Self {
            reader_index,
            merge_strategy,
            max_doc: merge_state.max_docs[reader_index],
            doc_id: -1,
            doc_map: Rc::clone(&merge_state.doc_maps[reader_index]),
        }
    }
}

impl SubBase for CompressingStoredFieldsMergeSub {
    fn next_doc(&mut self) -> Result<i32> {
        self.doc_id += 1;
        if self.doc_id == self.max_doc {
            Ok(NO_MORE_DOCS)
        } else {
            Ok(self.doc_id)
        }
    }

    fn get_doc_map(&self) -> Result<&Rc<DocMapEnum>> {
        Ok(&self.doc_map)
    }
}
// used for padding
impl Default for CompressingStoredFieldsMergeSub {
    fn default() -> Self {
        Self {
            reader_index: 0,
            max_doc: 0,
            merge_strategy: MergeStrategy::Doc,
            doc_id: -1,
            doc_map: Rc::new(DocMapEnum::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::codecs::compressing::lucene90_compressing_stored_fields_reader::lucene90_csfr_util;
    use crate::codecs::compressing::lucene90_compressing_stored_fields_writer::lucene90_csfw_util;
    use crate::store::{ByteArrayDataInput, ByteArrayDataOutput};
    use crate::test::util::lucene_test_case::random;
    use crate::util::error::lucene_error::Result;
    use rand::Rng;
    #[allow(dead_code)] // for quick search
    struct TestCompressingStoredFieldsFormat;
    #[test]
    fn test_zfloat() -> Result<()> {
        let buffer_out = vec![0u8; 5]; // we never need more than 5 bytes
        let mut output = ByteArrayDataOutput::with_bytes(buffer_out);

        // round-trip small integer values
        for i in i16::MIN..i16::MAX {
            let f = i as f32;
            lucene90_csfw_util::write_zfloat(&mut output, f)?;
            let mut input =
                ByteArrayDataInput::with_range(output.bytes.clone(), 0, output.get_position());
            let g = lucene90_csfr_util::read_zfloat(&mut input)?;

            assert!(input.eof());
            assert_eq!(f.to_bits(), g.to_bits());
            // check that compression actually works
            if (-1..=123).contains(&i) {
                assert_eq!(output.get_position(), 1);
            }
            output.reset()?;
        }

        // round-trip special values
        let special = [
            -0.0f32,
            0.0f32,
            f32::NEG_INFINITY,
            f32::INFINITY,
            f32::MIN,
            f32::MAX,
            f32::NAN,
        ];

        for &f in &special {
            lucene90_csfw_util::write_zfloat(&mut output, f)?;
            let mut input =
                ByteArrayDataInput::with_range(output.bytes.clone(), 0, output.get_position());
            let g = lucene90_csfr_util::read_zfloat(&mut input)?;
            assert!(input.eof());
            assert_eq!(f.to_bits(), g.to_bits());
            output.reset()?;
        }

        // round-trip random values
        let mut rng = random();
        for _ in 0..100_000 {
            let f: f32 = rng.random::<f32>() * (rng.random_range(0..100) as f32 - 50.0);
            lucene90_csfw_util::write_zfloat(&mut output, f)?;

            let len = output.get_position();
            assert!(
                len <= if (f.to_bits() >> 31) == 1 { 5 } else { 4 },
                "length={}, f={}",
                len,
                f
            );

            let mut input =
                ByteArrayDataInput::with_range(output.bytes.clone(), 0, output.get_position());
            let g = lucene90_csfr_util::read_zfloat(&mut input)?;
            assert!(input.eof());
            assert_eq!(f.to_bits(), g.to_bits());

            output.reset()?;
        }

        Ok(())
    }
    #[test]
    fn test_zdouble() -> Result<()> {
        let buffer_out = vec![0u8; 9]; // we never need more than 9 bytes
        let mut output = ByteArrayDataOutput::with_bytes(buffer_out);

        // round-trip small integer values
        for i in i16::MIN..i16::MAX {
            let x = i as f64;
            lucene90_csfw_util::write_zdouble(&mut output, x)?;
            let mut input =
                ByteArrayDataInput::with_range(output.bytes.clone(), 0, output.get_position());
            let y = lucene90_csfr_util::read_zdouble(&mut input)?;
            assert!(input.eof());
            assert_eq!(x.to_bits(), y.to_bits());

            // check that compression actually works
            if (-1..=124).contains(&i) {
                assert_eq!(output.get_position(), 1); // single byte compression
            }

            output.reset()?;
        }

        // round-trip special values
        let special = [
            -0.0f64,
            0.0f64,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::MIN,
            f64::MAX,
            f64::NAN,
        ];

        for &x in &special {
            lucene90_csfw_util::write_zdouble(&mut output, x)?;
            let mut input =
                ByteArrayDataInput::with_range(output.bytes.clone(), 0, output.get_position());
            let y = lucene90_csfr_util::read_zdouble(&mut input)?;
            assert!(input.eof());
            assert_eq!(x.to_bits(), y.to_bits());
            output.reset()?;
        }

        // round-trip random double values
        let mut random = random();
        for _ in 0..100_000 {
            let x = random.random::<f64>() * (random.random_range(0..100) as f64 - 50.0);
            lucene90_csfw_util::write_zdouble(&mut output, x)?;
            let len = output.get_position();
            assert!(
                len <= if x < 0.0 { 9 } else { 8 },
                "length={}, d={}",
                len,
                x
            );

            let mut input =
                ByteArrayDataInput::with_range(output.bytes.clone(), 0, output.get_position());
            let y = lucene90_csfr_util::read_zdouble(&mut input)?;
            assert!(input.eof());
            assert_eq!(x.to_bits(), y.to_bits());

            output.reset()?;
        }

        // round-trip float values cast to double
        for _ in 0..100_000 {
            let x = random.random::<f32>() * (random.random_range(0..100) as f32 - 50.0);
            let x = x as f64;
            lucene90_csfw_util::write_zdouble(&mut output, x)?;
            let len = output.get_position();
            assert!(len <= 5, "length={}, d={}", len, x);
            let mut input =
                ByteArrayDataInput::with_range(output.bytes.clone(), 0, output.get_position());
            let y = lucene90_csfr_util::read_zdouble(&mut input)?;
            assert!(input.eof());
            assert_eq!(x.to_bits(), y.to_bits());

            output.reset()?;
        }

        Ok(())
    }
    #[test]
    fn test_tlong() -> Result<()> {
        let buffer_out = vec![0u8; 10]; // we never need more than 10 bytes
        let mut output = ByteArrayDataOutput::with_bytes(buffer_out);

        // round-trip small integer values
        for i in i16::MIN..i16::MAX {
            for &mul in &[
                lucene90_csfw_util::SECOND,
                lucene90_csfw_util::HOUR,
                lucene90_csfw_util::DAY,
            ] {
                let l1 = i as i64 * mul;
                lucene90_csfw_util::write_tlong(&mut output, l1)?;
                let mut input =
                    ByteArrayDataInput::with_range(output.bytes.clone(), 0, output.get_position());
                let l2 = lucene90_csfr_util::read_tlong(&mut input)?;
                assert!(input.eof());
                assert_eq!(l1, l2);

                // check that compression actually works
                if (-16..=15).contains(&i) {
                    assert_eq!(output.get_position(), 1); // single byte compression
                }

                output.reset()?;
            }
        }

        // round-trip random values
        let mut rng = random();
        for _ in 0..100_000 {
            let num_bits = rng.random_range(0..=64);
            let mask = if num_bits == 64 {
                i64::MAX
            } else {
                ((1u64 << num_bits) - 1) as i64
            };
            let mut l1 = rng.random::<i64>() & mask;
            match rng.random_range(0..4) {
                0 => l1 *= lucene90_csfw_util::SECOND,
                1 => l1 *= lucene90_csfw_util::HOUR,
                2 => l1 *= lucene90_csfw_util::DAY,
                _ => {} // uncompressed
            }

            lucene90_csfw_util::write_tlong(&mut output, l1)?;
            let mut input =
                ByteArrayDataInput::with_range(output.bytes.clone(), 0, output.get_position());
            let l2 = lucene90_csfr_util::read_tlong(&mut input)?;
            assert!(input.eof());
            assert_eq!(l1, l2);
            output.reset()?;
        }

        Ok(())
    }
}

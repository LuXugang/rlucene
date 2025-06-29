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
use std::cell::RefCell;
use std::rc::Rc;

use crate::codecs::lucene90::fields_index::FieldsIndex;
use crate::codecs::lucene90::fields_index_writer::fields_index_writer_const;
use crate::codecs::CodecUtil;
use crate::index::IndexFileNames;
use crate::store::directory::Directory;
use crate::store::{IOContext, IndexInput, ReadAdvice};
use crate::util::error::lucene_error::Result;
use crate::util::long_values::LongValues;
use crate::util::packed::direct_monotonic_reader::direct_monotonic::Meta;
use crate::util::packed::direct_monotonic_reader::{
    direct_monotonic_reader_util, DirectMonotonicReader,
};
use crate::util::StringHelper;

pub(crate) struct FieldsIndexReader<I>
where
    I: IndexInput,
{
    max_doc: i32,
    block_shift: i32,
    num_chunks: i32,
    docs_meta: Meta,
    start_pointers_meta: Meta,
    index_input: I,
    docs_start_pointer: i64,
    docs_end_pointer: i64,
    start_pointers_start_pointer: i64,
    start_pointers_end_pointer: i64,
    docs: DirectMonotonicReader<I::RandomAccessSlice>,
    start_pointers: DirectMonotonicReader<I::RandomAccessSlice>,
    max_pointer: i64,
}
impl<I> FieldsIndexReader<I>
where
    I: IndexInput,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new<D>(
        dir: &mut D,
        name: String,
        suffix: &str,
        extension: &str,
        codec_name: &str,
        id: &[u8; StringHelper::ID_LENGTH],
        meta_in: &mut impl IndexInput,
        context: &IOContext,
    ) -> Result<Self>
    where
        D: Directory<IndexInputType = I>,
    {
        let max_doc = meta_in.read_int()?;
        let block_shift = meta_in.read_int()?;
        let num_chunks = meta_in.read_int()?;
        let docs_start_pointer = meta_in.read_long()?;
        let docs_meta =
            direct_monotonic_reader_util::load_meta(meta_in, num_chunks as i64, block_shift)?;
        let docs_end_pointer = meta_in.read_long()?;
        let start_pointers_start_pointer = meta_in.read_long()?;
        let start_pointers_meta =
            direct_monotonic_reader_util::load_meta(meta_in, num_chunks as i64, block_shift)?;
        let start_pointers_end_pointer = meta_in.read_long()?;
        let max_pointer = meta_in.read_long()?;

        let mut index_input = dir.open_input(
            &IndexFileNames::segment_file_name(&name, suffix, extension),
            &context.with_read_advice(ReadAdvice::RandomPreload)?,
        )?;

        CodecUtil::check_index_header(
            &mut index_input,
            &format!("{}Idx", codec_name),
            fields_index_writer_const::VERSION_START,
            fields_index_writer_const::VERSION_CURRENT,
            id,
            suffix,
        )?;
        CodecUtil::retrieve_checksum(&mut index_input)?;

        let docs_slice = index_input
            .random_access_slice(docs_start_pointer, docs_end_pointer - docs_start_pointer)?;
        let start_pointers_slice = index_input.random_access_slice(
            start_pointers_start_pointer,
            start_pointers_end_pointer - start_pointers_start_pointer,
        )?;
        let docs =
            DirectMonotonicReader::get_instance(&docs_meta, Rc::new(RefCell::new(docs_slice)))?;
        let start_pointers = DirectMonotonicReader::get_instance(
            &start_pointers_meta,
            Rc::new(RefCell::new(start_pointers_slice)),
        )?;

        Ok(FieldsIndexReader {
            max_doc,
            block_shift,
            num_chunks,
            docs_meta,
            start_pointers_meta,
            index_input,
            docs_start_pointer,
            docs_end_pointer,
            start_pointers_start_pointer,
            start_pointers_end_pointer,
            max_pointer,
            docs,
            start_pointers,
        })
    }
    fn new_with_other(other: &FieldsIndexReader<I>) -> Result<Self> {
        let docs_meta = other.docs_meta.clone();
        let start_pointers_meta = other.start_pointers_meta.clone();
        let docs_slice = Rc::new(RefCell::new(other.index_input.random_access_slice(
            other.docs_start_pointer,
            other.docs_end_pointer - other.docs_start_pointer,
        )?));
        let start_pointers_slice = Rc::new(RefCell::new(other.index_input.random_access_slice(
            other.start_pointers_start_pointer,
            other.start_pointers_end_pointer - other.start_pointers_start_pointer,
        )?));
        let docs = DirectMonotonicReader::get_instance(&docs_meta, docs_slice)?;
        let start_pointers =
            DirectMonotonicReader::get_instance(&start_pointers_meta, start_pointers_slice)?;
        Ok(FieldsIndexReader {
            max_doc: other.max_doc,
            block_shift: other.block_shift,
            num_chunks: other.num_chunks,
            docs_meta,
            start_pointers_meta,
            index_input: other.index_input.try_clone()?,
            docs_start_pointer: other.docs_start_pointer,
            docs_end_pointer: other.docs_end_pointer,
            start_pointers_start_pointer: other.start_pointers_start_pointer,
            start_pointers_end_pointer: other.start_pointers_end_pointer,
            max_pointer: other.max_pointer,
            docs,
            start_pointers,
        })
    }
    pub(crate) fn get_max_pointer(&self) -> i64 {
        self.max_pointer
    }
}

impl<I> crate::util::clone::TryClone for FieldsIndexReader<I>
where
    I: IndexInput,
{
    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized,
    {
        FieldsIndexReader::new_with_other(self)
    }
}

impl<I> FieldsIndex for FieldsIndexReader<I>
where
    I: IndexInput,
{
    fn get_block_id(&mut self, doc_id: i32) -> Result<i64> {
        assert!(doc_id >= 0 && doc_id < self.max_doc);
        let block_index = self
            .docs
            .binary_search(0, self.num_chunks as i64, doc_id as i64)?;
        let block_index = if block_index < 0 {
            -(2 + block_index)
        } else {
            block_index
        };
        Ok(block_index)
    }

    fn get_block_start_pointer(&mut self, block_id: i64) -> Result<i64> {
        self.start_pointers.get(block_id)
    }

    fn get_block_length(&mut self, block_id: i64) -> Result<i64> {
        let end_pointer = if block_id == (self.num_chunks - 1) as i64 {
            self.max_pointer
        } else {
            self.start_pointers.get(block_id + 1)?
        };
        Ok(end_pointer - self.get_block_start_pointer(block_id)?)
    }

    fn check_integrity(&mut self) -> Result<()> {
        CodecUtil::checksum_entire_file(&self.index_input)?;
        Ok(())
    }
}

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
use crate::codecs::block_tree::lucene90_block_tree_terms_reader::Lucene90BlockTreeTermsReader;
use crate::codecs::lucene101::lucene101_postings_reader::Lucene101PostingsReader;
use crate::store::IndexInput;
use crate::util::error::lucene_error::Result;
pub trait FieldsProducer<I>
where
    I: IndexInput,
{
    fn close(&mut self) -> Result<()>;
    /// Checks consistency of this reader.
    ///
    /// Note that this may be costly in terms of I/O, e.g. may involve computing
    /// a checksum value against large data files.
    fn check_integrity(&mut self) -> Result<()>;
    /// Returns an instance optimized for merging. This instance may only be
    /// cloned # Note
    /// Returning None means returning itself.
    fn get_merge_instance(&self) -> Result<Option<FieldsProducerEnum<I>>> {
        Ok(None)
    }
}

pub enum FieldsProducerEnum<I>
where
    I: IndexInput,
{
    Lucene90(Lucene90BlockTreeTermsReader<I, Lucene101PostingsReader<I>>),
}
impl<I> FieldsProducer<I> for FieldsProducerEnum<I>
where
    I: IndexInput,
{
    fn close(&mut self) -> Result<()> {
        match self {
            FieldsProducerEnum::Lucene90(reader) => reader.close(),
        }
    }

    fn check_integrity(&mut self) -> Result<()> {
        match self {
            FieldsProducerEnum::Lucene90(reader) => reader.check_integrity(),
        }
    }

    fn get_merge_instance(&self) -> Result<Option<FieldsProducerEnum<I>>> {
        match self {
            FieldsProducerEnum::Lucene90(reader) => reader.get_merge_instance(),
        }
    }
}

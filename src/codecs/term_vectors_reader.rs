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
use crate::codecs::compressing::lucene90_compressing_term_vectors_reader::{
    Lucene90CompressingTermVectorsReader, TVFields,
};
use crate::index::term_vectors::TermVectors;
use crate::store::IndexInput;
use crate::util::clone::TryClone;
use crate::util::error::lucene_error::Result;
/// Codec API for reading term vectors:
pub trait TermVectorsReader<I>: TermVectors + TryClone
where
    I: IndexInput,
{
    /// Checks consistency of this reader.
    ///
    /// Note that this may be costly in terms of I/O, e.g. may involve computing
    /// a checksum value against large data files.
    fn check_integrity(&mut self) -> Result<()>;

    /// Returns an instance optimized for merging.
    ///
    /// This instance may only be used from the thread that acquires it.
    fn get_merge_instance(&self) -> Result<Option<TermVectorsReaderEnum<I>>> {
        Ok(None)
    }
}

pub enum TermVectorsReaderEnum<I>
where
    I: IndexInput,
{
    Lucene90(Lucene90CompressingTermVectorsReader<I>),
}

impl<I> TermVectors for TermVectorsReaderEnum<I>
where
    I: IndexInput,
{
    type Fields = TVFields;

    fn get(&mut self, doc: i32) -> Result<Option<Self::Fields>> {
        match self {
            TermVectorsReaderEnum::Lucene90(reader) => reader.get(doc),
        }
    }
}

impl<I> TryClone for TermVectorsReaderEnum<I>
where
    I: IndexInput,
{
    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized,
    {
        match self {
            TermVectorsReaderEnum::Lucene90(reader) => {
                Ok(TermVectorsReaderEnum::Lucene90(reader.try_clone()?))
            },
        }
    }
}

impl<I> TermVectorsReader<I> for TermVectorsReaderEnum<I>
where
    I: IndexInput,
{
    fn check_integrity(&mut self) -> Result<()> {
        match self {
            TermVectorsReaderEnum::Lucene90(reader) => reader.check_integrity(),
        }
    }

    fn get_merge_instance(&self) -> Result<Option<TermVectorsReaderEnum<I>>> {
        match self {
            TermVectorsReaderEnum::Lucene90(reader) => reader.get_merge_instance(),
        }
    }
}

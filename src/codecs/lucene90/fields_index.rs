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
use crate::codecs::lucene90::fields_index_reader::FieldsIndexReader;
use crate::store::IndexInput;
use crate::util::clone::TryClone;
use crate::util::error::lucene_error::Result;
#[allow(unused)]
pub(crate) trait FieldsIndex: TryClone {
    /// Get the ID of the block that contains the given docID.
    fn get_block_id(&mut self, doc_id: i32) -> Result<i64>;

    /// Get the start pointer of the block with the given ID.
    fn get_block_start_pointer(&mut self, block_id: i64) -> Result<i64>;

    /// Get the number of bytes of the block with the given ID.
    fn get_block_length(&mut self, block_id: i64) -> Result<i64>;

    /// Get the start pointer of the block that contains the given docID.
    /// This is a final method in the original struct, so it's implemented
    /// directly here.
    fn get_start_pointer(&mut self, doc_id: i32) -> Result<i64> {
        let block_id = self.get_block_id(doc_id)?;
        self.get_block_start_pointer(block_id)
    }

    /// Check the integrity of the index.
    fn check_integrity(&mut self) -> Result<()>;
}

pub(crate) enum FieldsIndexEnum<I>
where
    I: IndexInput,
{
    Lucene90(FieldsIndexReader<I>),
}

impl<I> TryClone for FieldsIndexEnum<I>
where
    I: IndexInput,
{
    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized,
    {
        match self {
            FieldsIndexEnum::Lucene90(reader) => {
                let cloned_reader = reader.try_clone()?;
                Ok(FieldsIndexEnum::Lucene90(cloned_reader))
            },
        }
    }
}

impl<I> FieldsIndex for FieldsIndexEnum<I>
where
    I: IndexInput,
{
    fn get_block_id(&mut self, doc_id: i32) -> Result<i64> {
        match self {
            FieldsIndexEnum::Lucene90(reader) => reader.get_block_id(doc_id),
        }
    }

    fn get_block_start_pointer(&mut self, block_id: i64) -> Result<i64> {
        match self {
            FieldsIndexEnum::Lucene90(reader) => reader.get_block_start_pointer(block_id),
        }
    }

    fn get_block_length(&mut self, block_id: i64) -> Result<i64> {
        match self {
            FieldsIndexEnum::Lucene90(reader) => reader.get_block_length(block_id),
        }
    }

    fn get_start_pointer(&mut self, doc_id: i32) -> Result<i64> {
        match self {
            FieldsIndexEnum::Lucene90(reader) => reader.get_start_pointer(doc_id),
        }
    }

    fn check_integrity(&mut self) -> Result<()> {
        match self {
            FieldsIndexEnum::Lucene90(reader) => reader.check_integrity(),
        }
    }
}

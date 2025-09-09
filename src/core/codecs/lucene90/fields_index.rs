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
use crate::core::codecs::lucene90::fields_index_reader::FieldsIndexReader;
use crate::core::store::IndexInput;
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error::Result;
pub(crate) trait FieldsIndex: TryClone {
    /// Get the ID of the block that contains the given docID.
    fn get_block_id(&self, doc_id: i32) -> Result<i64>;

    /// Get the start pointer of the block with the given ID.
    fn get_block_start_pointer(&self, block_id: i64) -> Result<i64>;

    /// Get the number of bytes of the block with the given ID.
    fn get_block_length(&self, block_id: i64) -> Result<i64>;

    /// Get the start pointer of the block that contains the given docID.
    /// This is a final method in the original struct, so it's implemented
    /// directly here.
    fn get_start_pointer(&self, doc_id: i32) -> Result<i64> {
        let block_id = self.get_block_id(doc_id)?;
        self.get_block_start_pointer(block_id)
    }

    /// Check the integrity of the index.
    fn check_integrity(&self) -> Result<()>;
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
    fn get_block_id(&self, doc_id: i32) -> Result<i64> {
        match self {
            FieldsIndexEnum::Lucene90(reader) => reader.get_block_id(doc_id),
        }
    }

    fn get_block_start_pointer(&self, block_id: i64) -> Result<i64> {
        match self {
            FieldsIndexEnum::Lucene90(reader) => reader.get_block_start_pointer(block_id),
        }
    }

    fn get_block_length(&self, block_id: i64) -> Result<i64> {
        match self {
            FieldsIndexEnum::Lucene90(reader) => reader.get_block_length(block_id),
        }
    }

    fn get_start_pointer(&self, doc_id: i32) -> Result<i64> {
        match self {
            FieldsIndexEnum::Lucene90(reader) => reader.get_start_pointer(doc_id),
        }
    }

    fn check_integrity(&self) -> Result<()> {
        match self {
            FieldsIndexEnum::Lucene90(reader) => reader.check_integrity(),
        }
    }
}

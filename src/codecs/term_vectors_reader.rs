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
use crate::codecs::compressing::lucene90_compressing_term_vectors_reader::{
    Lucene90CompressingTermVectorsReader, TVFields,
};
use crate::index::term_vectors::TermVectors;
use crate::store::IndexInput;
use crate::util::clone::TryClone;
use crate::util::error::lucene_error::Result;
/// Codec API for reading term vectors:
pub trait TermVectorsReader: TermVectors + TryClone {
    /// Checks consistency of this reader.
    ///
    /// Note that this may be costly in terms of I/O, e.g. may involve computing
    /// a checksum value against large data files.
    fn check_integrity(&self) -> Result<()>;

    /// Returns an instance optimized for merging.
    ///
    /// This instance may only be used from the thread that acquires it.
    fn get_merge_instance(&self) -> Result<Option<Self>>
    where
        Self: Sized,
    {
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

impl<I> TermVectorsReader for TermVectorsReaderEnum<I>
where
    I: IndexInput,
{
    fn check_integrity(&self) -> Result<()> {
        match self {
            TermVectorsReaderEnum::Lucene90(reader) => reader.check_integrity(),
        }
    }

    fn get_merge_instance(&self) -> Result<Option<TermVectorsReaderEnum<I>>> {
        match self {
            TermVectorsReaderEnum::Lucene90(reader) => {
                let merge_instance = reader.get_merge_instance()?;
                Ok(merge_instance.map(TermVectorsReaderEnum::Lucene90))
            },
        }
    }
}

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

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
use crate::codecs::block_tree::lucene90_block_tree_terms_writer::Lucene90BlockTreeTermsWriter;
use crate::codecs::lucene101::lucene101_postings_writer::Lucene101PostingsWriter;
use crate::codecs::norms_producer::NormsProducer;
use crate::codecs::push_postings_writer_base::PushPostingsWriterBase;
use crate::index::fields::Fields;
use crate::store::IndexOutput;
use crate::util::error::lucene_error::Result;

pub trait FieldsConsumer {
    fn write<F, N>(&mut self, fields: &mut F, norms: &mut N) -> Result<()>
    where
        F: Fields,
        N: NormsProducer;
    fn close(&mut self) -> Result<()>;
}

pub enum FieldsConsumerEnum<O>
where
    O: IndexOutput,
{
    Lucene90(Lucene90BlockTreeTermsWriter<O, PushPostingsWriterBase<Lucene101PostingsWriter<O>>>),
}
impl<O> FieldsConsumer for FieldsConsumerEnum<O>
where
    O: IndexOutput,
{
    fn write<F, N>(&mut self, fields: &mut F, norms: &mut N) -> Result<()>
    where
        F: Fields,
        N: NormsProducer,
    {
        match self {
            FieldsConsumerEnum::Lucene90(writer) => writer.write(fields, norms),
        }
    }

    fn close(&mut self) -> Result<()> {
        match self {
            FieldsConsumerEnum::Lucene90(writer) => writer.close(),
        }
    }
}

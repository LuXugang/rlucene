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
use crate::core::index::composite_reader_context::{CompositeReaderContext, create};
use crate::core::index::index_reader::{IndexReader, IndexReaderEnum};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

pub trait CompositeReader: IndexReader {
    type LeafReader: LeafReader + Clone;
    type CompositeReader: CompositeReader<LeafReader = Self::LeafReader>;
    fn get_sequential_sub_readers(
        &self,
    ) -> Vec<IndexReaderEnum<Self::LeafReader, Self::CompositeReader>>;
    fn to_string(&self) -> String {
        todo!()
    }
}

pub fn get_context<CR>(composite_reader: CR) -> Result<CompositeReaderContext<CR>>
where
    CR: CompositeReader + Clone,
{
    create(composite_reader)
}
impl<CR> CompositeReader for Arc<CR>
where
    CR: CompositeReader,
{
    type LeafReader = CR::LeafReader;
    type CompositeReader = CR::CompositeReader;

    fn get_sequential_sub_readers(
        &self,
    ) -> Vec<IndexReaderEnum<Self::LeafReader, Self::CompositeReader>> {
        (**self).get_sequential_sub_readers()
    }

    fn to_string(&self) -> String {
        (**self).to_string()
    }
}

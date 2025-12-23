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
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::dummy::dummy_stored_fields::DummyStoredFields;
use crate::core::index::dummy::dummy_term_vectors::DummyTermVectors;
use crate::core::index::index_reader::{IndexReader, IndexReaderBase, IndexReaderEnum};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::term::Term;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;

#[derive(Clone, Default)]
pub struct DummyCompositeReader<LR> {
    _marker: PhantomData<LR>,
}

impl<LR> DummyCompositeReader<LR> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<LR> IndexReader for DummyCompositeReader<LR> {
    type TermVectors = DummyTermVectors;

    fn term_vectors(&self) -> Result<Self::TermVectors> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn max_doc(&self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn num_docs(&self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type StoredFields = DummyStoredFields;

    fn stored_fields(&self) -> Result<Self::StoredFields> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn do_close(&self) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn doc_freq(&self, _term: &Term) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn total_term_freq(&self, _term: &Term) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_sum_doc_freq(&self, _field: &str) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_doc_count(&self, _field: &str) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_sum_total_term_freq(&self, _field: &str) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn base(&self) -> &IndexReaderBase {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl<LR> Display for DummyCompositeReader<LR> {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl<LR> CompositeReader for DummyCompositeReader<LR>
where
    LR: LeafReader + Clone,
{
    type LeafReader = LR;
    type SubCompositeReader = DummyCompositeReader<LR>;

    fn get_sequential_sub_readers(
        &self,
    ) -> Vec<IndexReaderEnum<Self::LeafReader, Self::SubCompositeReader>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn to_string(&self) -> String {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

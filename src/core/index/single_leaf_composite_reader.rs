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
use crate::core::index::dummy::dummy_cache_helper::DummyCacheHelper;
use crate::core::index::dummy::dummy_composite_reader::DummyCompositeReader;
use crate::core::index::index_reader::{
    IRStoredFields, IRTermVectors, IndexReader, IndexReaderBase, IndexReaderEnum,
    IndexReaderEnumCacheHelperType,
};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::term::Term;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub struct SingleLeafCompositeReader<LR>
where
    LR: LeafReader + Clone,
{
    leaf_reader: Arc<Vec<IndexReaderEnum<LR, DummyCompositeReader<LR>>>>,
}
impl<LR> SingleLeafCompositeReader<LR>
where
    LR: LeafReader + Clone,
{
    pub fn new(lr: LR) -> Self {
        let v = IndexReaderEnum::Leaf(lr);
        Self {
            leaf_reader: Arc::new(vec![v]),
        }
    }
}

impl<LR> IndexReader for SingleLeafCompositeReader<LR>
where
    LR: LeafReader + Clone,
{
    type TermVectors = IRTermVectors<LR, DummyCompositeReader<LR>>;

    fn term_vectors(&self) -> Result<Self::TermVectors> {
        self.leaf_reader[0].term_vectors()
    }

    fn max_doc(&self) -> Result<i32> {
        self.leaf_reader[0].max_doc()
    }

    fn num_docs(&self) -> Result<i32> {
        self.leaf_reader[0].num_docs()
    }

    type StoredFields = IRStoredFields<LR, DummyCompositeReader<LR>>;

    fn stored_fields(&self) -> Result<Self::StoredFields> {
        self.leaf_reader[0].stored_fields()
    }

    type ReaderCacheHelper =
        IndexReaderEnumCacheHelperType<LR::ReaderCacheHelper, DummyCacheHelper>;

    fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
        self.leaf_reader[0].get_reader_cache_helper()
    }

    fn doc_freq(&self, term: &Term) -> Result<i32> {
        self.leaf_reader[0].doc_freq(term)
    }

    fn total_term_freq(&self, term: &Term) -> Result<i64> {
        self.leaf_reader[0].total_term_freq(term)
    }

    fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
        self.leaf_reader[0].get_sum_doc_freq(field)
    }

    fn get_doc_count(&self, field: &str) -> Result<i32> {
        self.leaf_reader[0].get_doc_count(field)
    }

    fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
        self.leaf_reader[0].get_sum_total_term_freq(field)
    }

    fn index_base(&self) -> &IndexReaderBase {
        self.leaf_reader[0].index_base()
    }
}

impl<LR> Display for SingleLeafCompositeReader<LR>
where
    LR: LeafReader + Clone,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::any::type_name::<Self>())
    }
}

impl<LR> CompositeReader for SingleLeafCompositeReader<LR>
where
    LR: LeafReader + Clone,
{
    type LeafReader = LR;
    type SubCompositeReader = DummyCompositeReader<LR>;

    fn get_sequential_sub_readers(
        &self,
    ) -> &[IndexReaderEnum<Self::LeafReader, Self::SubCompositeReader>] {
        &self.leaf_reader
    }

    fn to_string(&self) -> String {
        todo!()
    }
}

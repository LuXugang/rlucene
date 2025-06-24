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
use crate::index::dummy::doc_comparator::DummyDocComparator;
use crate::index::dummy::dummy_leaf_reader::DummyLeafReader;
use crate::index::index_sorter::IndexSorter;

pub struct DummyIndexSorter;
impl IndexSorter for DummyIndexSorter {
    fn get_provider_name(&self) -> &str {
        todo!()
    }

    type DocComparator = DummyDocComparator;
    type LeafReader = DummyLeafReader;

    fn get_doc_comparator(
        &mut self,
        _leaf_reader: &mut Self::LeafReader,
        _max_doc: i32,
    ) -> crate::util::error::lucene_error::Result<Option<Self::DocComparator>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

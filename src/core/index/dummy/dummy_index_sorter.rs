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
use crate::core::index::dummy::doc_comparator::DummyDocComparator;
use crate::core::index::dummy::dummy_comparable_provider::DummyComparableProvider;
use crate::core::index::index_sorter::IndexSorter;
use crate::core::index::leaf_reader::LeafReader;

pub struct DummyIndexSorter;
impl IndexSorter for DummyIndexSorter {
    fn get_provider_name(&self) -> &str {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type ComparableProvider<LR>
        = DummyComparableProvider
    where
        LR: LeafReader;

    fn get_comparable_providers<LR>(
        &self,
        _readers: &[LR],
    ) -> crate::core::util::error::lucene_error::Result<Vec<Self::ComparableProvider<LR>>>
    where
        LR: LeafReader,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type DocComparator = DummyDocComparator;

    fn get_doc_comparator<LR>(
        &self,
        _leaf_reader: &LR,
        _max_doc: i32,
    ) -> crate::core::util::error::lucene_error::Result<Self::DocComparator>
    where
        LR: LeafReader,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

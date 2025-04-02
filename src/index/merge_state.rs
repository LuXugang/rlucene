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
use crate::codecs::doc_values_producer::DocValuesProducerEnum;
use crate::codecs::norms_producer::NormsProducerEnum;
#[cfg(test)]
use crate::index::doc_id_merger::tests::DocMapMock1;
use crate::index::dummy::dummy_doc_map::DummyDocMap;
use crate::index::field_infos::FieldInfos;
use crate::index::index_writer::DocMapIndexWriter;
#[cfg(test)]
use crate::index::tests::DocMapMock2;
use crate::store::IndexInput;
#[cfg(test)]
use crate::test::util::bkd::test_bkd::DocMapMock;
use std::rc::Rc;

pub struct MergeState<I>
where
    I: IndexInput,
{
    pub doc_maps: Vec<Rc<DocMapEnum>>,
    pub merge_field_infos: Rc<FieldInfos>,
    pub norms_producers: Vec<Option<NormsProducerEnum<I>>>,
    pub doc_values_producers: Vec<DocValuesProducerEnum<I>>,
    pub field_infos: Vec<Rc<FieldInfos>>,
    pub needs_index_sort: bool,
}

/// A map of doc IDs.
pub trait DocMap {
    /// Return the mapped docID or -1 if the given doc is not mapped.
    fn get(&self, doc_id: i32) -> i32;
}

pub enum DocMapEnum {
    #[cfg(test)]
    Mock(DocMapMock),
    #[cfg(test)]
    MocK1(DocMapMock1),
    #[cfg(test)]
    MocK2(DocMapMock2),
    DocMapImpl(DocMapIndexWriter),
    Dummy(DummyDocMap),
}
/// # Note:
/// Default value used for padding
impl Default for DocMapEnum {
    fn default() -> Self {
        DocMapEnum::Dummy(DummyDocMap)
    }
}
impl DocMap for DocMapEnum {
    fn get(&self, doc_id: i32) -> i32 {
        match self {
            #[cfg(test)]
            DocMapEnum::Mock(doc_map) => doc_map.get(doc_id),
            #[cfg(test)]
            DocMapEnum::MocK1(doc_map) => doc_map.get(doc_id),
            #[cfg(test)]
            DocMapEnum::MocK2(doc_map) => doc_map.get(doc_id),
            DocMapEnum::DocMapImpl(doc_map) => doc_map.get(doc_id),
            DocMapEnum::Dummy(doc_map) => doc_map.get(doc_id),
        }
    }
}

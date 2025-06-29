/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/
use std::rc::Rc;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::codecs::doc_values_producer::DocValuesProducerEnum;
use crate::codecs::norms_producer::NormsProducerEnum;
use crate::codecs::stored_fields_reader::StoredFieldsReaderEnum;
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
use crate::util::bits::BitsEnum;
use crate::util::info_stream::InfoStreamEnum;

pub struct MergeState<I>
where
    I: IndexInput,
{
    pub doc_maps: Vec<Rc<DocMapEnum>>,
    pub merge_field_infos: Rc<FieldInfos>,
    pub stored_fields_readers: Vec<StoredFieldsReaderEnum<I>>,
    pub norms_producers: Vec<Option<NormsProducerEnum<I>>>,
    pub doc_values_producers: Vec<Option<DocValuesProducerEnum<I>>>,
    pub field_infos: Vec<Rc<FieldInfos>>,
    pub live_docs: Vec<Option<Rc<BitsEnum>>>,
    pub needs_index_sort: bool,
    pub max_docs: Vec<i32>,
    pub info_stream: Arc<Mutex<InfoStreamEnum>>,
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

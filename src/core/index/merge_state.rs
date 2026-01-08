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
use crate::core::codecs::doc_values_producer::DocValuesProducerType;
use crate::core::codecs::norms_producer::NormsProducerType;
use crate::core::codecs::stored_fields_reader::StoredFieldsReaderType;
use crate::core::index::codec_reader::{CRBits, CodecReader};
#[cfg(test)]
use crate::core::index::doc_id_merger::tests::DocMapMock1;
use crate::core::index::dummy::dummy_doc_map::DummyDocMap;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::index_writer::{DocMapIndexWriter, is_congruent_sort};
use crate::core::index::multi_sorter::{MultiSorter, MultiSorterDocMap};
use crate::core::index::segment_info::SegmentInfo;
#[cfg(test)]
use crate::core::index::tests::DocMapMock2;
use crate::core::search::sort::Sort;
use crate::core::store::IndexInput;
use crate::core::store::directory::Directory;
use crate::core::util::bits::{Bits, BitsEnum};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::info_stream::{InfoStream, InfoStreamEnum};
use crate::core::util::long_values::LongValues;
use crate::core::util::packed::PackedInts;
use crate::core::util::packed::packed_long_values::PackedLongValues;
#[cfg(test)]
use crate::test::util::bkd::test_bkd::DocMapMock;
use std::rc::Rc;
use std::sync::Arc;
use std::time::SystemTime;

pub struct MergeState<I>
where
    I: IndexInput,
{
    pub doc_maps: Vec<Rc<DocMapEnum>>,
    pub merge_field_infos: Arc<FieldInfos>,
    pub stored_fields_readers: Vec<StoredFieldsReaderType<I>>,
    pub norms_producers: Vec<Option<NormsProducerType<I>>>,
    pub doc_values_producers: Vec<Option<DocValuesProducerType<I>>>,
    pub field_infos: Vec<Arc<FieldInfos>>,
    pub live_docs: Vec<Option<Rc<BitsEnum>>>,
    pub needs_index_sort: bool,
    pub max_docs: Vec<i32>,
    pub info_stream: Arc<InfoStreamEnum>,
}
impl<I> MergeState<I>
where
    I: IndexInput,
{
    fn build_doc_maps<CR>(
        &mut self,
        readers: &[CR],
        index_sort: Option<Sort>,
    ) -> Result<Vec<MergeStateDocMap<CR>>>
    where
        CR: CodecReader,
    {
        if let Some(ref sort) = index_sort {
            // do a merge sort of the incoming leaves:
            let t0 = SystemTime::now();
            match MultiSorter::sort(sort, readers)? {
                None => {
                    // already sorted, fall back to deletion-only mapping
                    build_deletion_doc_maps(readers)
                },
                Some(result) => {
                    self.needs_index_sort = true;

                    let t1 = SystemTime::now();
                    if self.info_stream.enabled("SM") {
                        let elapsed = t1.duration_since(t0).unwrap().as_secs_f64() * 1000.0;
                        self.info_stream.message(
                            "SM",
                            &format!("{:.2} msec to build merge sorted DocMaps", elapsed),
                        );
                    }
                    Ok(result)
                },
            }
        } else {
            // no index sort ... we only must map around deletions, and rebase to the merged segment's
            // docID space
            build_deletion_doc_maps(readers)
        }
    }
}

pub type MergeStateDocMap<CR> = DocMapEnum2<MultiSorterDocMap<CR>, DocMapImpl2<CRBits<CR>>>;

// Remap docIDs around deletions
fn build_deletion_doc_maps<CR>(readers: &[CR]) -> Result<Vec<MergeStateDocMap<CR>>>
where
    CR: CodecReader,
{
    let mut total_docs: i32 = 0;
    let num_readers = readers.len();
    let mut doc_maps = Vec::with_capacity(num_readers);

    for reader in readers.iter() {
        let live_docs = reader.get_live_docs()?;

        let del_doc_map = if let Some(ref bits) = live_docs {
            Some(remove_deletes(reader.max_doc()?, bits)?)
        } else {
            None
        };

        let doc_base = total_docs;

        doc_maps.push(DocMapEnum2::B(DocMapImpl2::new(
            live_docs,
            del_doc_map,
            doc_base,
        )));

        total_docs += reader.num_docs()?;
    }

    Ok(doc_maps)
}
fn verify_index_sort<CR, D>(readers: &[CR], segment_info: &SegmentInfo<D>) -> Result<()>
where
    CR: CodecReader,
    D: Directory,
{
    let index_sort = match segment_info.get_index_sort() {
        Some(sort) => sort,
        None => return Ok(()),
    };

    for leaf in readers {
        let segment_sort = leaf.get_metadata()?.get_sort();
        if !segment_sort
            .as_ref()
            .map(|s| is_congruent_sort(&index_sort, s))
            .unwrap_or(false)
        {
            return Err(LuceneError::illegal_argument(format!(
                "index sort mismatch: merged segment has sort={} but to-be-merged segment has sort={}",
                index_sort,
                segment_sort
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "null".to_string())
            )));
        }
    }

    Ok(())
}

pub(crate) fn remove_deletes<B>(max_doc: i32, live_docs: &B) -> Result<PackedLongValues>
where
    B: Bits,
{
    let mut builder = PackedLongValues::monotonic_long_values_builder_default(PackedInts::COMPACT)?;

    let mut del = 0;
    for i in 0..max_doc {
        builder.add(i as i64 - del)?;
        if !live_docs.get(i as usize) {
            del += 1;
        }
    }
    builder.build()
}

pub struct DocMapImpl2<B>
where
    B: Bits,
{
    live_docs: Option<B>,
    del_doc_map: Option<PackedLongValues>,
    doc_base: i32,
}
impl<B> DocMapImpl2<B>
where
    B: Bits,
{
    fn new(live_docs: Option<B>, del_doc_map: Option<PackedLongValues>, doc_base: i32) -> Self {
        Self {
            live_docs,
            del_doc_map,
            doc_base,
        }
    }
}
impl<B> DocMap for DocMapImpl2<B>
where
    B: Bits,
{
    fn get(&self, doc_id: i32) -> Result<i32> {
        match (&self.live_docs, &self.del_doc_map) {
            (None, None) => Ok(self.doc_base + doc_id),
            (Some(bits), Some(map)) => {
                if bits.get(doc_id as usize) {
                    Ok(self.doc_base + map.get(doc_id as usize)? as i32)
                } else {
                    Ok(-1)
                }
            },
            _ => Err(LuceneError::illegal_state("should not be here")),
        }
    }
}

/// A map of doc IDs.
pub trait DocMap {
    /// Return the mapped docID or -1 if the given doc is not mapped.
    fn get(&self, doc_id: i32) -> Result<i32>;
}
macro_rules! either_doc_map {
    ($vis:vis $name:ident { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> DocMap for $name<$( $T ),+>
        where
            $( $T: DocMap ),+
        {
            #[inline]
            fn get(&self, doc_id: i32) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.get(doc_id), )+
                }
            }
        }
    };
}
either_doc_map!(pub DocMapEnum2 { A: A, B: B});

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
    fn get(&self, doc_id: i32) -> Result<i32> {
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

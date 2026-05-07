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
use crate::core::index::composite_reader::get_context;
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::dummy::dummy_composite_reader::DummyCompositeReader;
use crate::core::index::dummy::dummy_leaf_reader::DummyLeafReader;
use crate::core::index::leaf_reader_context::{LeafReaderContext, TopParentMeta};
use crate::core::index::segment_reader::SegmentReader;
use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
use crate::core::search::index_searcher::{DefaultIndexSearcher, IndexSearcher};
use crate::core::store::directory::DirEnum;
use std::sync::Arc;

pub(crate) mod automaton;
pub(crate) mod base_bit_set_test_case;
pub(crate) mod base_doc_id_set_test_case;
pub(crate) mod base_sort_test_case;
pub(crate) mod bkd;
pub(crate) mod common_method;
pub mod english;
pub(crate) mod fst;
pub mod hnsw;
pub(crate) mod id_set_common;
pub(crate) mod index_package_access;
pub(crate) mod line_file_docs;
pub(crate) mod lucene_test_case;
mod packed;
mod test_fixed_bit_doc_id_set;
mod test_fixed_bit_set;
mod test_int_array_doc_id_set;
mod test_line_file_docs;
mod test_not_doc_id_set;
mod test_roaring_doc_id_set;
mod test_sparse_fixed_bit_set;
pub mod test_util;

pub type DefaultCRReaderShared = Arc<StandardDirectoryReaderType<DirEnum>>;
pub type DefaultCRReader = StandardDirectoryReaderType<DirEnum>;
pub type DefaultLRReader = Arc<SegmentReader<DirEnum>>;
pub type DefaultIRCRC = CompositeReaderContext<DefaultCRReader>;
pub type DefaultIRCLR = LeafReaderContext<DefaultLRReader>;
pub type DefaultIndexSearchCRShared =
  DefaultIndexSearcher<CompositeReaderContext<DefaultCRReaderShared>>;
pub type DefaultIndexSearchCR = DefaultIndexSearcher<CompositeReaderContext<DefaultCRReader>>;
pub type DefaultIndexSearchLR = DefaultIndexSearcher<LeafReaderContext<DefaultLRReader>>;
pub(crate) fn dummy_index_searcher() -> crate::core::util::error::lucene_error::Result<
  DefaultIndexSearcher<CompositeReaderContext<DummyCompositeReader<DummyLeafReader>>>,
> {
  let dummy_lr = DummyLeafReader;
  let cr = DummyCompositeReader::new(dummy_lr);
  let irc = get_context(cr)?;
  IndexSearcher::new(irc)
}

impl LeafReaderContext<DummyLeafReader> {
  pub(crate) fn dummy_lrc() -> Self {
    let parent = TopParentMeta::default();
    Self::new(DummyLeafReader, 0, 0, 0, 0, parent)
  }
}

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
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::string_field::StringField;
use crate::core::index::composite_reader::get_context;
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
use crate::core::search::index_searcher::{DefaultIndexSearcher, IndexSearcher};
use crate::core::store::directory::DirEnum;
use crate::core::store::nio_fs_directory::NIOFSDirectory;
use crate::core::store::{FSDirectory, NativeFSLockFactory};
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;
use tempfile::TempDir;

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
pub(crate) mod null_info_stream;
mod packed;

mod test_array_util;
mod test_bit_util;
mod test_byte_block_pool;
mod test_bytes_ref;
mod test_bytes_ref_array;
mod test_bytes_ref_hash;
mod test_collection_util;
mod test_doc_id_set_builder;
mod test_fixed_bit_doc_id_set;
mod test_fixed_bit_set;
mod test_frequency_tracking_ring_buffer;
mod test_int_array_doc_id_set;
mod test_intro_selector;
mod test_ints_ref;
mod test_line_file_docs;
mod test_long_bit_set;
mod test_long_heap;
mod test_longs_ref;
mod test_lsb_radix_sorter;
mod test_math_util;
mod test_merged_iterator;
mod test_msb_radix_sorter;
mod test_not_doc_id_set;
mod test_numeric_utils;
mod test_paged_bytes;
mod test_priority_queue;
mod test_radix_selector;
mod test_roaring_doc_id_set;
mod test_sloppy_math;
mod test_small_float;
mod test_sparse_fixed_bit_set;
mod test_stable_msb_radix_sorter;
mod test_string_helper;
mod test_string_sorter;
mod test_tim_sorter;
pub mod test_vector_util;
mod test_version;

mod compress;
mod test_intro_sorter;
pub mod test_util;
pub mod throttled_index_output;

pub type DefaultCRReaderShared = Arc<StandardDirectoryReaderType<DirEnum>>;
pub type DefaultCRReader = StandardDirectoryReaderType<DirEnum>;
pub type DefaultLRReader = DefaultLeafReader<DirEnum>;
pub type DefaultIRCRC = CompositeReaderContext<DefaultCRReader>;
pub type DefaultIRCLR = LeafReaderContext<DefaultLRReader>;
pub type DefaultIndexSearchCRShared =
  DefaultIndexSearcher<CompositeReaderContext<DefaultCRReaderShared>>;
pub type DefaultIndexSearchCR = DefaultIndexSearcher<CompositeReaderContext<DefaultCRReader>>;
pub type DefaultIndexSearchLR = DefaultIndexSearcher<LeafReaderContext<DefaultLRReader>>;
pub type DummyCR = StandardDirectoryReaderType<FSDirectory<NativeFSLockFactory, NIOFSDirectory>>;
pub(crate) fn dummy_directory() -> Result<Arc<FSDirectory<NativeFSLockFactory, NIOFSDirectory>>> {
  let temp_dir = TempDir::new()?;
  Ok(Arc::new(NIOFSDirectory::new(temp_dir.keep())?))
}
pub(crate) fn dummy_index_searcher(
  dir: Arc<FSDirectory<NativeFSLockFactory, NIOFSDirectory>>,
) -> crate::core::util::error::lucene_error::Result<
  DefaultIndexSearcher<CompositeReaderContext<DummyCR>>,
> {
  let iw = IndexWriter::new(dir, IndexWriterConfig::new())?;
  let mut doc = Document::new();
  doc.add(StringField::from_string(
    "id",
    format!("doc-{}", 0),
    Store::No,
  )?);
  iw.add_document(doc)?;
  let reader = iw.get_reader(true, true)?;
  let irc = get_context(reader)?;
  iw.close()?;
  IndexSearcher::new(irc)
}

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
pub(crate) mod automaton;
pub(crate) mod bkd;
pub(crate) mod compress;
pub(crate) mod fst;
pub(crate) mod hnsw;
pub(crate) mod packed;
pub mod quantization;

pub(crate) mod base_bit_set_test_case;
pub(crate) mod base_doc_id_set_test_case;
pub(crate) mod base_sort_test_case;
pub(crate) mod common_method;
pub(crate) mod id_set_common;

#[cfg(feature = "monster")]
mod test_2b_paged_bytes;
mod test_array_util;
mod test_bit_util;
mod test_byte_block_pool;
mod test_bytes_ref;
mod test_bytes_ref_array;
mod test_bytes_ref_hash;
mod test_class_loader_utils;
mod test_closeable_thread_local;
mod test_collection_util;
mod test_doc_id_set_builder;
mod test_filter_iterator;
mod test_fixed_bit_doc_id_set;
mod test_fixed_bit_set;
mod test_frequency_tracking_ring_buffer;
mod test_int_array_doc_id_set;
mod test_intro_selector;
mod test_intro_sorter;
mod test_ints_ref;
mod test_io_utils;
mod test_java_logging_info_stream;
mod test_java_test_harness;
mod test_line_file_docs;
mod test_long_bit_set;
mod test_long_heap;
mod test_longs_ref;
mod test_lsb_radix_sorter;
mod test_math_util;
mod test_merged_iterator;
mod test_msb_radix_sorter;
mod test_named_spi_loader;
mod test_not_doc_id_set;
mod test_numeric_utils;
mod test_paged_bytes;
mod test_priority_queue;
mod test_radix_selector;
mod test_ram_usage_estimator;
mod test_roaring_doc_id_set;
mod test_sloppy_math;
mod test_small_float;
mod test_sparse_fixed_bit_doc_id_set;
mod test_sparse_fixed_bit_set;
mod test_stable_msb_radix_sorter;
#[cfg(feature = "nightly")]
mod test_stress_ram_usage_estimator;
mod test_string_helper;
mod test_string_sorter;
mod test_tim_sorter;
#[cfg(feature = "nightly")]
mod test_tim_sorter_worst_case;
mod test_unicode_util;
pub mod test_vector_util;
mod test_version;
mod test_virtual_method;

pub use crate::test_framework::core::util::{
  DefaultCRReader, DefaultCRReaderShared, DefaultIRCLR, DefaultIRCRC, DefaultIndexSearchCR,
  DefaultIndexSearchCRShared, DefaultIndexSearchLR, DefaultLRReader, DummyCR,
};
pub(crate) use crate::test_framework::core::util::{dummy_directory, dummy_index_searcher};

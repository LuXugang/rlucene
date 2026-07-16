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
mod test_add_indexes;
mod test_approximate_priority_queue;
mod test_buffered_updates;
mod test_byte_slice_pool;
mod test_byte_slice_reader;
mod test_bytes_ref_builder;
mod test_caching_merge_context;
mod test_codecs;
mod test_concurrent_approximate_priority_queue;
mod test_concurrent_merge_scheduler;
mod test_crash;
mod test_custom_norms;
mod test_custom_term_freq;
mod test_deletion_policy;
mod test_directory_reader;
mod test_directory_reader_reopen;
mod test_doc;
mod test_doc_id_merger;
pub mod test_doc_inverter_per_field_error_info;
mod test_doc_values_field_updates;
mod test_docs_and_positions;
pub(crate) mod test_document_writer;
mod test_documents_writer_delete_queue;
mod test_documents_writer_per_thread_pool;
mod test_documents_writer_stall_control;
mod test_field_infos;
mod test_field_invert_state;
mod test_field_updates_buffer;
mod test_fields_reader;
mod test_flush_by_ram_or_counts_policy;
mod test_force_merge_forever;
mod test_freq_prox_terms_writer;
mod test_frozen_buffered_updates;
mod test_index_commit;
mod test_index_file_deleter;
mod test_index_sorting;
mod test_index_writer;
mod test_index_writer_commit;
mod test_index_writer_config;
mod test_index_writer_exceptions;
mod test_index_writer_force_merge;
pub mod test_index_writer_merge_policy;
mod test_index_writer_on_disk_full;
mod test_index_writer_on_error;
mod test_index_writer_out_of_file_descriptors;
pub(crate) mod test_index_writer_reader;
mod test_index_writer_threads_to_segments;
mod test_index_writer_with_threads;
mod test_indexing_sequence_numbers;
mod test_int_block_pool;
mod test_lockable_concurrent_approximate_priority_queue;
mod test_log_merge_policy;
mod test_many_fields;
mod test_max_term_frequency;
mod test_merge_on_flush_merge_policy;
mod test_merge_policy;
mod test_mixed_doc_values_updates;
mod test_multi_terms_enum;
mod test_newest_segment;
mod test_no_merge_policy;
mod test_norms;
mod test_ordinal_map;
mod test_pending_deletes;
mod test_pending_soft_deletes;
pub(crate) mod test_per_segment_deletes;
mod test_persistent_snapshot_deletion_policy;
mod test_reader_pool;
mod test_rolling_updates;
mod test_segment_merger;
mod test_segment_reader;
mod test_segment_term_docs;
mod test_segment_term_enum;
mod test_segment_to_thread_mapping;
mod test_snapshot_deletion_policy;
mod test_soft_deletes_retention_merge_policy;
mod test_term_doc_perf;
mod test_term_vectors_reader;
mod test_terms_enum2;
mod test_terms_enum_index;
pub(crate) mod test_terms_hash_per_field;
mod test_tiered_merge_policy;
mod test_tragic_index_writer_deadlock;
mod test_transaction_rollback;
mod test_transactions;
mod test_unique_term_count;

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
pub(crate) mod base_compound_format_test_case;
pub(crate) mod base_field_info_format_test_case;
pub(crate) mod base_index_file_format_test_case;
pub(crate) mod base_live_docs_format_test_case;
mod base_postings_format_test_case;
pub(crate) mod base_segment_info_format_test_case;
pub mod doc_helper;
pub mod random_index_writer;
mod test_binary_terms;
mod test_consistent_field_numbers;
mod test_doc_count;
pub(crate) mod test_doc_values_indexing;
mod test_docs_and_positions;
mod test_exceed_max_term_length;
mod test_index_many_documents;
pub(crate) mod test_index_writer;
mod test_index_writer_commit;
mod test_index_writer_delete;
mod test_index_writer_max_docs;
mod test_many_fields;
mod test_sum_doc_freq;

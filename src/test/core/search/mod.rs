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
pub mod base_knn_vector_query_test_case;
pub mod base_vector_similarity_query_test_case;
pub(crate) mod block_score_query_wrapper;
pub mod bulk_scorer_wrapper_scorer;
pub mod check_hits;
pub mod dummy_total_hit_count_collector;
pub(crate) mod fixed_bit_set_collector;
pub mod query_utils;
pub mod random_approximation_query;
pub mod scorer_index_searcher;
mod search_equivalence_test_base;
pub mod similarities;
mod test_base_range_filter;
mod test_block_max_conjunction;
pub(crate) mod test_boolean2;
pub(crate) mod test_boolean_min_should_match;
mod test_boolean_or;
mod test_boolean_query_visit_sub_scorers;
pub mod test_boolean_rewrites;
mod test_byte_vector_similarity_query;
mod test_conjunctions;
mod test_doc_values_queries;
pub mod test_doc_values_rewrite_method;
mod test_early_termination;
mod test_float_vector_similarity_query;
mod test_knn_byte_vector_query;
mod test_knn_float_vector_query;
mod test_lat_lon_doc_values_queries;
mod test_lat_lon_point_queries;
pub mod test_max_clause_limit;
pub mod test_min_should_match2;
pub mod test_multi_slice_merge;
pub(crate) mod test_point_queries;
mod test_regexp_random;
pub(crate) mod test_scorer_perf;
mod test_search_after;
mod test_similarity_provider;
mod test_simple_search_equivalence;
mod test_sloppy_phrase_query;
mod test_sloppy_phrase_query2;
mod test_sort_optimization;
pub(crate) mod test_sort_random;
mod test_top_docs_merge;
mod test_top_field_collector_early_termination;

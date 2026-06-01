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
mod knn;
pub mod query_utils;
pub mod random_approximation_query;
pub mod scorer_index_searcher;
mod search_equivalence_test_base;
pub mod similarities;
mod test_approximation_search_equivalence;
mod test_automaton_query;
mod test_automaton_query_unicode;
pub(crate) mod test_base_range_filter;
mod test_blended_term_query;
mod test_block_max_conjunction;
pub(crate) mod test_boolean2;
mod test_boolean2_scorer_supplier;
pub(crate) mod test_boolean_min_should_match;
mod test_boolean_or;
mod test_boolean_query;
mod test_boolean_query_visit_sub_scorers;
pub mod test_boolean_rewrites;
pub(crate) mod test_boolean_scorer;
mod test_boost_query;
mod test_byte_vector_similarity_query;
mod test_conjunction_disi;
mod test_conjunctions;
mod test_constant_score_query;
mod test_constant_score_scorer;
mod test_custom_searcher_sort;
mod test_date_sort;
mod test_disi_priority_queue;
pub(crate) mod test_disjunction_max_query;
mod test_disjunction_score_block_boundary_propagator;
mod test_doc_id_set_iterator;
mod test_doc_values_queries;
mod test_doc_values_range_iterator;
pub mod test_doc_values_rewrite_method;
mod test_early_termination;
mod test_field_exists_query;
mod test_float_vector_similarity_query;
mod test_fuzzy_query;
mod test_fuzzy_term_on_short_terms;
mod test_index_or_doc_values_query;
mod test_index_searcher;
mod test_index_sort_sorted_numeric_doc_values_range_query;
mod test_knn_byte_vector_query;
mod test_knn_float_vector_query;
mod test_lat_lon_doc_values_queries;
mod test_lat_lon_point_queries;
mod test_match_all_docs_query;
mod test_match_no_docs_query;
pub mod test_max_clause_limit;
mod test_max_score_accumulator;
mod test_max_score_bulk_scorer;
pub mod test_min_should_match2;
mod test_multi_phrase_enum;
mod test_multi_phrase_query;
pub mod test_multi_slice_merge;
mod test_multi_term_constant_score;
pub(crate) mod test_multi_term_query_rewrites;
mod test_multi_thread_term_vectors;
mod test_n_gram_phrase_query;
mod test_nearest;
pub(crate) mod test_needs_scores;
mod test_not;
pub(crate) mod test_per_thread_pk_lookup;
mod test_phrase_prefix_query;
mod test_phrase_query;
pub(crate) mod test_point_queries;
mod test_positive_scores_only_collector;
mod test_prefix_in_boolean_query;
mod test_prefix_query;
pub(crate) mod test_prefix_random;
mod test_range_fields_doc_values_query;
mod test_regexp_query;
mod test_regexp_random;
pub(crate) mod test_regexp_random2;
mod test_req_excl_bulk_scorer;
mod test_req_opt_sum_scorer;
pub(crate) mod test_scorer_perf;
mod test_search_after;
mod test_search_with_threads;
mod test_segment_cacheables;
pub(crate) mod test_similarity;
mod test_similarity_provider;
mod test_simple_search_equivalence;
mod test_sloppy_phrase_query;
mod test_sloppy_phrase_query2;
mod test_sort;
mod test_sort_optimization;
pub(crate) mod test_sort_random;
mod test_sorted_numeric_sort_field;
mod test_sorted_set_selector;
mod test_sorted_set_sort_field;
mod test_term_query;
mod test_term_range_query;
mod test_term_scorer;
mod test_time_limiting_bulk_scorer;
mod test_top_docs_collector;
mod test_top_docs_merge;
mod test_top_field_collector;
mod test_top_field_collector_early_termination;
mod test_top_knn_results;
mod test_total_hit_count_collector;
mod test_total_hits;
pub(crate) mod test_usage_tracking_filter_caching_policy;
mod test_vector_scorer;
mod test_vector_similarity_collector;
pub(crate) mod test_wand_scorer;
mod test_wildcard_query;
mod test_wildcard_random;
mod test_xy_doc_values_queries;
mod test_xy_point_distance_sort;
mod test_xy_point_queries;

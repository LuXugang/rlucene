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
mod abstract_knn_collector;
pub mod boost_attribute;
pub mod bulk_scorer;
pub mod collection_statistics;
pub mod collector;
pub mod collector_manager;
pub mod comparators;
pub mod constant_score_scorer;
pub mod constant_score_weight;
mod disjunction_matches_iterator;
pub mod doc_id_set;
pub mod doc_id_set_iterator;
pub mod doc_id_stream;
pub mod dummy;
pub mod explanation;
pub mod field_comparator;
pub mod field_comparator_source;
pub mod field_doc;
pub mod field_value_hit_queue;
mod filter_leaf_collector;
mod hit_queue;
mod impacts_disi;
pub mod index_searcher;
pub mod knn_collector;
pub mod leaf_collector;
pub mod leaf_field_comparator;
mod lru_query_cache;
pub mod match_all_docs_query;
pub mod match_no_docs_query;
pub mod matches;
pub mod matches_iterator;
pub(crate) mod matches_utils;
mod max_score_accumulator;
mod max_score_cache;
pub mod multi_leaf_field_comparator;
pub mod positive_scores_only_collector;
pub mod pruning;
pub mod query;
mod query_cache;
pub mod query_caching_policy;
pub mod query_visitor;
pub mod scorable;
pub mod score;
pub mod score_caching_wrapping_scorer;
pub mod score_doc;
pub mod score_mode;
pub mod scorer;
pub mod scorer_supplier;
pub mod segment_cacheable;
pub mod similarities_impl;
pub mod simple_collector;
pub mod sort_field;
pub mod sort_field_enum;
pub mod sorted_numeric_selector;
pub mod sorted_numeric_sort_field;
pub mod sorted_set_selector;
pub mod sorted_set_sort_field;
mod term_matches_iterator;
pub mod term_query;
mod term_scorer;
pub mod term_statistics;
mod time_limiting_bulk_scorer;
pub mod top_docs;
pub mod top_docs_collector;

pub use query_cache::QueryCache;
pub mod abstract_knn_vector_query;
pub(crate) mod abstract_multi_term_query_constant_score_wrapper;
pub mod abstract_vector_similarity_query;
pub mod automaton_query;
pub(crate) mod block_max_conjunction_bulk_scorer;
pub(crate) mod block_max_conjunction_scorer;
pub mod boolean_clause;
pub mod boolean_query;
pub(crate) mod boolean_scorer;
mod boolean_scorer_supplier;
pub(crate) mod boolean_weight;
pub mod boost_query;
pub mod byte_vector_similarity_query;
pub(crate) mod conjunction_bulk_scorer;
pub(crate) mod conjunction_disi;
pub(crate) mod conjunction_scorer;
pub mod conjunction_utils;
pub mod constant_score_query;
pub mod disi_priority_queue;
pub mod disi_wrapper;
pub mod disjunction_disi_approximation;
pub mod disjunction_max_bulk_scorer;
pub mod disjunction_max_query;
mod disjunction_max_scorer;
mod disjunction_score_block_boundary_propagator;
pub(crate) mod disjunction_scorer;
pub(crate) mod disjunction_sum_scorer;
pub mod doc_values_range_iterator;
mod dummy_query_caching_policy;
mod exact_phrase_matcher;
pub mod field_exists_query;
pub mod filter_doc_id_set_iterator;
pub mod filter_scorable;
pub mod filter_scorer;
pub mod filtered_doc_id_set_iterator;
pub mod float_vector_similarity_query;
pub mod index_or_doc_values_query;
pub mod index_sort_sorted_numeric_doc_values_range_query;
pub mod knn;
pub mod knn_byte_vector_query;
pub mod knn_float_vector_query;
mod max_score_bulk_scorer;
pub mod multi_term_query;
mod multi_term_query_constant_score_blended_wrapper;
mod multi_term_query_constant_score_wrapper;
pub mod n_gram_phrase_query;
pub mod phrase_matcher;
mod phrase_positions;
pub mod phrase_query;
pub(crate) mod phrase_queue;
pub(crate) mod phrase_scorer;
pub(crate) mod phrase_weight;
pub mod point_range_query;
pub mod prefix_query;
pub mod regexp_query;
pub(crate) mod req_excl_bulk_scorer;
pub(crate) mod req_excl_scorer;
pub(crate) mod req_opt_sum_scorer;
mod scorer_util;
pub(crate) mod simple_scorable;
mod sloppy_phrase_matcher;
pub mod sort;
pub mod term_in_set_query;
pub mod term_range_query;
pub mod time_limiting_knn_collector_manager;
pub mod top_field_collector;
pub mod top_field_collector_manager;
pub mod top_field_docs;
pub mod top_knn_collector;
mod top_score_doc_collector;
pub mod top_score_doc_collector_manager;
pub mod total_hit_count_collector;
pub mod total_hit_count_collector_manager;
pub mod total_hits;
pub mod two_phase_iterator;
pub mod usage_tracking_query_caching_policy;
pub(crate) mod vector_scorer;
mod vector_similarity_collector;
pub(crate) mod wand_scorer;
pub mod weight;
pub mod wildcard_query;

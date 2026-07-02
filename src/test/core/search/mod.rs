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
pub(crate) mod knn;
pub(crate) mod similarities;
mod test_automaton_query;
pub(crate) mod test_boolean2;
mod test_boolean2_scorer_supplier;
mod test_boolean_or;
mod test_boolean_query;
pub(crate) mod test_boolean_scorer;
mod test_byte_vector_similarity_query;
mod test_conjunction_disi;
mod test_custom_searcher_sort;
mod test_disi_priority_queue;
pub(crate) mod test_disjunction_max_query;
mod test_disjunction_score_block_boundary_propagator;
mod test_doc_values_range_iterator;
mod test_field_exists_query;
mod test_index_or_doc_values_query;
mod test_index_sort_sorted_numeric_doc_values_range_query;
pub mod test_lru_query_cache;
mod test_max_score_accumulator;
mod test_max_score_bulk_scorer;
pub mod test_min_should_match2;
mod test_phrase_query;
mod test_req_excl_bulk_scorer;
mod test_req_opt_sum_scorer;
mod test_score_caching_wrapping_scorer;
mod test_top_docs_collector;
mod test_top_docs_merge;
mod test_top_field_collector;

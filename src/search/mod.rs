/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/
mod abstract_knn_collector;
pub mod collection_statistics;
pub mod doc_id_set;
pub mod doc_id_set_iterator;
pub mod dummy;
mod explanation;
pub mod field_comparator;
pub mod field_comparator_source;
pub mod field_value_hit_queue;
pub mod index_searcher;
pub mod knn_collector;
pub mod leaf_field_comparator;
pub mod pruning;
pub mod query;
pub mod scorable;
pub mod score_doc;
pub mod score_mode;
pub mod similarities;
pub mod sort_field;
pub mod sort_field_enum;
pub mod sorted_numeric_selector;
pub mod sorted_numeric_sort_field;
pub mod sorted_set_selector;
pub mod sorted_set_sort_field;
pub mod term_query;
mod term_statistics;
pub(crate) mod top_docs;
pub mod top_knn_collector;
mod total_hits;
mod vector_scorer;
mod vector_similarity_collector;

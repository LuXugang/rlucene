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
use crate::search::top_docs::TopDocs;
use crate::util::error::lucene_error::Result;

/// KnnCollector is a knn collector used for gathering kNN results and providing
/// topDocs from the gathered neighbors
pub trait KnnCollector {
    /// If search visits too many documents, the results collector will
    /// terminate early.
    ///
    /// Usually, this is due to some restricted filter on the document set.
    ///
    /// When collection is early terminated, the results are not a correct
    /// representation of k nearest neighbors.
    ///
    /// # Returns
    ///
    /// Whether the current result set is marked as incomplete.
    fn early_terminated(&self) -> bool;

    /// Increments the visited vector count.
    ///
    /// # Arguments
    ///
    /// * `count` - must be greater than 0.
    fn inc_visited_count(&mut self, count: usize);

    /// Returns the current visited vector count.
    fn visited_count(&self) -> usize;

    /// Returns the visited vector limit.
    fn visit_limit(&self) -> usize;

    /// Returns the expected number of collected results.
    fn k(&self) -> i32;

    /// Collects the provided `doc_id` and includes it in the result set.
    ///
    /// # Arguments
    ///
    /// * `doc_id` - ID of the vector to collect.
    /// * `similarity` - its calculated similarity.
    ///
    /// # Returns
    ///
    /// `true` if the vector is collected.
    fn collect(&mut self, doc_id: i32, similarity: f32) -> bool;

    /// This method is utilized during search to ensure only competitive results
    /// are explored.
    ///
    /// If this results collector wants to collect `k` results, this should
    /// return [`f32::NEG_INFINITY`] when not full. When full, the minimum
    /// score should be returned.
    ///
    /// # Returns
    ///
    /// The current minimum competitive similarity in the collection.
    fn min_competitive_similarity(&self) -> f32;

    /// This drains the collected nearest kNN results and returns them as a
    /// [`TopDocs`] collection, ordered by score descending.
    ///
    /// **Note:** This is generally a destructive action and the collector
    /// should not be used after `top_docs()` is called.
    ///
    /// # Returns
    ///
    /// The collected top documents.
    fn top_docs(&mut self) -> Result<TopDocs>;
}

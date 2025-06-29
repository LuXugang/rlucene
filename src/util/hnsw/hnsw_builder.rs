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
use crate::util::error::lucene_error::Result;
use crate::util::hnsw::on_heap_hnsw_graph::OnHeapHnswGraph;
use crate::util::info_stream::InfoStreamLock;

/// Interface for building an [`OnHeapHnswGraph`].
///
/// # Experimental
/// This API is experimental and subject to change.
pub trait HnswBuilder {
    /// Adds all nodes to the graph up to the provided `max_ord` (exclusive).
    ///
    /// # Arguments
    ///
    /// * `max_ord` - The maximum ordinal (excluded) of the nodes to be added.
    ///
    /// # Returns
    ///
    /// The built [`OnHeapHnswGraph`].
    fn build(&mut self, max_ord: i32) -> Result<&mut OnHeapHnswGraph>;

    /// Inserts a doc with vector value to the graph.
    fn add_graph_node(&mut self, node: i32) -> Result<()>;

    /// Sets the info stream for debug output.
    fn set_info_stream(&mut self, info_stream: InfoStreamLock);

    /// Returns a reference to the current graph under construction.
    fn get_graph(&mut self) -> &mut OnHeapHnswGraph;
    /// Once this method is called, no further updates to the graph are
    /// accepted.
    ///
    /// Calling this method disables further calls to `add_graph_node`, which
    /// will panic (equivalent to throwing `IllegalStateException` in Java).
    /// Final modifications to the graph—such as patching disconnected
    /// components or reordering node IDs for better delta compression—may be
    /// triggered.
    ///
    /// This operation may be time-consuming, and callers should expect it to
    /// take some time.
    fn get_completed_graph(&mut self) -> Result<&mut OnHeapHnswGraph>;
}

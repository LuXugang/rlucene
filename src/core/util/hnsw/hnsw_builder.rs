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
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::on_heap_hnsw_graph::OnHeapHnswGraph;
use crate::core::util::info_stream::InfoStreamMT;

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
    fn build(&mut self, max_ord: usize) -> Result<&mut OnHeapHnswGraph>;

    /// Inserts a doc with vector value to the graph.
    fn add_graph_node(&mut self, node: usize) -> Result<()>;

    /// Sets the info stream for debug output.
    fn set_info_stream(&mut self, info_stream: InfoStreamMT);

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

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
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::hnsw::hnsw_graph::HnswGraph;
use crate::core::util::hnsw::hnsw_graph_builder::{
  HnswGraphBuilder, HnswGraphBuilderBase, HnswGraphBuilderBaseEnum,
};
use crate::core::util::hnsw::hnsw_graph_searcher::HnswGraphSearcherBaseDefault;
use crate::core::util::hnsw::on_heap_hnsw_graph::OnHeapHnswGraph;
use crate::core::util::hnsw::random_vector_scorer_supplier::RandomVectorScorerSupplier;
/// This creates a graph builder that is initialized with the provided [`HnswGraph`]. This is useful for
/// merging HnswGraphs from multiple segments.
pub struct InitializedHnswGraphBuilder<B>
where
  B: Bits,
{
  initialized_nodes: B,
}
pub fn new<B, S>(
  scorer_supplier: S,
  m: usize,
  beam_width: usize,
  random: u64,
  hnsw: OnHeapHnswGraph,
  initialized_nodes: B,
) -> Result<HnswGraphBuilder<B, S, FixedBitSet, HnswGraphSearcherBaseDefault>>
where
  B: Bits,
  S: RandomVectorScorerSupplier,
{
  let sub = Some(HnswGraphBuilderBaseEnum::Initialized(
    InitializedHnswGraphBuilder { initialized_nodes },
  ));
  let base = HnswGraphBuilder::from_hnsw(scorer_supplier, m, beam_width, random, hnsw, sub)?;
  Ok(base)
}
impl<B> HnswGraphBuilderBase for InitializedHnswGraphBuilder<B>
where
  B: Bits,
{
  fn do_add_graph_node(&mut self, node: usize) -> Result<bool> {
    self.initialized_nodes.get(node)
  }
}
/// Create a new [`HnswGraphBuilder`] that is initialized with the provided [`HnswGraph`].
///
/// # Arguments
///
/// * `scorer_supplier` - the scorer to use for vectors
/// * `m` - the number of connections to keep per node
/// * `beam_width` - the number of nodes to explore in the search
/// * `seed` - the seed for the random number generator
/// * `initializer_graph` - the graph to initialize the new graph builder
/// * `new_ord_map` - a mapping from the old node ordinal to the new node ordinal
/// * `initialized_nodes` - a bitset of nodes that are already initialized in the `initializer_graph`
/// * `total_number_of_vectors` - the total number of vectors in the new graph, this should include
///   all vectors expected to be added to the graph in the future
///
/// # Returns
///
/// A new [`HnswGraphBuilder`] that is initialized with the provided [`HnswGraph`].
///
/// # Errors
///
/// Returns an error when reading the graph fails.
#[allow(clippy::too_many_arguments)]
pub fn from_graph<B, S, G>(
  scorer_supplier: S,
  m: usize,
  beam_width: usize,
  seed: u64,
  initializer_graph: &mut G,
  new_ord_map: &[usize],
  initialized_nodes: B,
  total_number_of_vectors: i32,
) -> Result<HnswGraphBuilder<B, S, FixedBitSet, HnswGraphSearcherBaseDefault>>
where
  G: HnswGraph,
  B: Bits,
  S: RandomVectorScorerSupplier,
{
  new(
    scorer_supplier,
    m,
    beam_width,
    seed,
    init_graph(m, initializer_graph, new_ord_map, total_number_of_vectors)?,
    initialized_nodes,
  )
}

pub fn init_graph<G>(
  m: usize,
  initializer_graph: &mut G,
  new_ord_map: &[usize],
  total_number_of_vectors: i32,
) -> Result<OnHeapHnswGraph>
where
  G: HnswGraph,
{
  let mut hnsw = OnHeapHnswGraph::new(m, total_number_of_vectors);
  for level in (0..initializer_graph.num_levels()?).rev() {
    let it = initializer_graph.get_nodes_on_level(level)?;
    for old_ord in it {
      let new_ord = new_ord_map[old_ord];
      hnsw.add_node(level, new_ord)?;
      hnsw.try_set_new_entry_node(new_ord, level);
      initializer_graph.seek(level, old_ord)?;
      loop {
        let old_neighbor = initializer_graph.next_neighbor()?;
        if old_neighbor == NO_MORE_DOCS as usize {
          break;
        }
        let new_neighbor = new_ord_map[old_neighbor];
        // we will compute these scores later when we need to pop out the non-diverse nodes
        hnsw
          .get_neighbors_mut(level, new_ord)?
          .add_out_of_order(new_neighbor, f32::NAN)?;
      }
    }
  }
  Ok(hnsw)
}

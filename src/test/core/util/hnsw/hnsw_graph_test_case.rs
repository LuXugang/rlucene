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
use crate::core::codecs::hnsw::default_flat_vector_scorer::{
  ByteVectorScorer, DefaultFlatVectorScorer, FloatVectorScorer,
};
use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
use crate::core::document::field::Field;
use crate::core::index::byte_vector_values::ByteVectorValues;
use crate::core::index::dummy::dummy_doc_index_iterator::DummyDocIndexIterator;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::knn_vector_values::{
  BitsImpl1, DenseDocIndexIterator, KnnVectorValues, KnnVectorValuesEnm2, create_dense_iterator,
};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, RangeDISI};
use crate::core::search::dummy::dummy_vector_scorer::DummyVectorScorer;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::query::Query;
use crate::core::search::top_knn_collector::TopKnnCollector;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::hnsw::hnsw_builder::HnswBuilder;
use crate::core::util::hnsw::hnsw_graph::{HnswGraph, NodesIterator};
use crate::core::util::hnsw::neighbor_array::NeighborArray;
use crate::core::util::hnsw::neighbor_queue::NeighborQueue;
use crate::core::util::hnsw::on_heap_hnsw_graph::OnHeapHnswGraph;
use crate::core::util::hnsw::random_vector_scorer::RandomVectorScorerEnum2;
use crate::core::util::hnsw::{
  hnsw_graph_builder, hnsw_graph_searcher, initialized_hnsw_graph_builder,
};
use crate::core::util::vector_util::VectorUtil;
use crate::test::core::util::hnsw::mock_byte_vector_values::MockByteVectorValues;
use crate::test::core::util::hnsw::mock_vector_values::MockVectorValues;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::at_least_usize;
use rand::{Rng, RngExt};
use std::borrow::Cow;
use std::collections::HashSet;

pub trait HnswGraphTestCase<T>
where
  T: Clone,
{
  fn score(&self, query: &T, vector: &T) -> f32;
  fn set_similarity_function(&mut self, s: VectorSimilarityFunction);
  fn similarity_function(&self) -> VectorSimilarityFunction;
  fn flat_vector_scorer(&self) -> DefaultFlatVectorScorer {
    DefaultFlatVectorScorer
  }

  fn get_vector_encoding(&self) -> VectorEncoding;

  fn knn_query(&self, field: &str, vector: T, k: usize) -> Result<Query>;

  fn random_vector<R>(&self, random: &mut R, dim: usize) -> T
  where
    R: Rng + ?Sized;

  fn vector_values<R>(&self, size: usize, dimension: usize, random: &mut R) -> TestsKnnVectorValues
  where
    R: Rng + ?Sized;

  fn vector_values_from_values<R>(
    &self,
    values: Vec<Vec<f32>>,
    random: &mut R,
  ) -> TestsKnnVectorValues
  where
    R: Rng + ?Sized;

  fn vector_values_from_reader<LR, R>(
    &self,
    reader: &LR,
    field_name: &str,
    random: &mut R,
  ) -> Result<TestsKnnVectorValues>
  where
    LR: LeafReader,
    R: Rng + ?Sized;
  fn vector_values_with_pregenerated<R>(
    &self,
    size: usize,
    dimension: usize,
    pregenerated_vector_values: TestsKnnVectorValues,
    pregenerated_offset: usize,
    random: &mut R,
  ) -> TestsKnnVectorValues
  where
    R: Rng + ?Sized;

  fn knn_vector_field(
    &self,
    name: &str,
    vector: T,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<Field>;
  fn circular_vector_values(&self, n_doc: usize) -> TestsCircularKnnVectorValues;

  fn get_target_vector(&self) -> T;

  fn build_scorer_supplier<B, F, R>(
    &self,
    vectors: KnnVectorValuesEnm2<B, F>,
    _random: &mut R,
  ) -> Result<<DefaultFlatVectorScorer as FlatVectorsScorer>::RandomVectorScorerSupplier<B, F>>
  where
    B: ByteVectorValues + Clone,
    F: FloatVectorValues + Clone,
    R: Rng + ?Sized,
  {
    let v = self.similarity_function();
    self
      .flat_vector_scorer()
      .get_random_vector_scorer_supplier(v, vectors)
  }

  fn build_scorer<B, F>(
    &self,
    vectors: KnnVectorValuesEnm2<B, F>,
    query: T,
  ) -> Result<RandomVectorScorerEnum2<ByteVectorScorer<B>, FloatVectorScorer<F>>>
  where
    B: ByteVectorValues,
    F: FloatVectorValues;

  fn test_random_read_write_and_merge<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO Knn 合并未实现
    Ok(())
  }
  fn vector_value(&self, vectors: &TestsKnnVectorValues, ord: usize) -> Result<T>;
  fn test_read_write<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_sorted_and_unsorted_indices_return_same_results<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_aknn_diverse<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let n_doc = 100;
    self.set_similarity_function(VectorSimilarityFunction::DotProduct);
    let vectors = self.circular_vector_values(n_doc);
    let scorer_supplier = self.build_scorer_supplier(vectors.clone(), random)?;
    let mut builder = hnsw_graph_builder::create(scorer_supplier, 10, 100, random.random::<u64>())?;
    let hnsw = builder.build(vectors.size())?;
    let scorer = self.build_scorer(vectors, self.get_target_vector())?;
    let mut nn = hnsw_graph_searcher::search_with_top_k(
      &scorer,
      10,
      hnsw,
      None::<&FixedBitSet>,
      i32::MAX as usize,
    )?;
    let top_docs = nn.top_docs()?;
    assert_eq!(
      10,
      top_docs.score_docs.len(),
      "Number of found results is not equal to [10]."
    );

    let mut sum = 0_i32;
    for node in &top_docs.score_docs {
      sum += node.doc;
    }
    assert!(sum < 75, "sum(result docs)={sum}");

    for i in 0..n_doc {
      let neighbors = hnsw.get_neighbors(0, i)?;
      let nnodes = neighbors.nodes();
      for &neighbor in nnodes.iter().take(neighbors.size()) {
        assert!(
          neighbor < n_doc,
          "neighbor node id {neighbor} is out of range"
        );
      }
    }

    Ok(())
  }

  fn test_search_with_accept_ords<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let n_doc = 100;
    let vectors = self.circular_vector_values(n_doc);
    self.set_similarity_function(VectorSimilarityFunction::DotProduct);
    let scorer_supplier = self.build_scorer_supplier(vectors.clone(), random)?;
    let mut builder = hnsw_graph_builder::create(scorer_supplier, 16, 100, random.random::<u64>())?;
    let hnsw = builder.build(vectors.size())?;

    // The first 10 docs must remain accepted to preserve the expected recall.
    let accept_ords = create_random_accept_ords(10, n_doc, random);

    let scorer = self.build_scorer(vectors, self.get_target_vector())?;
    let mut nn = hnsw_graph_searcher::search_with_top_k(
      &scorer,
      10,
      hnsw,
      Some(&accept_ords),
      i32::MAX as usize,
    )?;
    let top_docs = nn.top_docs()?;
    assert_eq!(
      10,
      top_docs.score_docs.len(),
      "Number of found results is not equal to [10]."
    );

    let mut sum = 0_i32;
    for node in &top_docs.score_docs {
      assert!(
        accept_ords.get(node.doc as usize)?,
        "the results include a deleted document: {node:?}"
      );
      sum += node.doc;
    }

    // We expect to get approximately 100% recall;
    // the lowest docIds are closest to zero; sum(0,9) = 45.
    assert!(sum < 75, "sum(result docs)={sum}");

    Ok(())
  }
  fn test_search_with_selective_accept_ords<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let n_doc = 100;
    let vectors = self.circular_vector_values(n_doc);
    self.set_similarity_function(VectorSimilarityFunction::DotProduct);
    let scorer_supplier = self.build_scorer_supplier(vectors.clone(), random)?;
    let mut builder = hnsw_graph_builder::create(scorer_supplier, 16, 100, random.random::<u64>())?;
    let hnsw = builder.build(vectors.size())?;

    // Only mark a few vectors as accepted.
    let mut accept_ords = FixedBitSet::new(n_doc);
    let mut i = 0;
    while i < n_doc {
      accept_ords.set(i);
      i += random.random_range(15..20);
    }

    // Check the search finds all accepted vectors.
    let num_accepted = accept_ords.cardinality();
    let scorer = self.build_scorer(vectors, self.get_target_vector())?;
    let mut nn = hnsw_graph_searcher::search_with_top_k(
      &scorer,
      num_accepted,
      hnsw,
      Some(&accept_ords),
      i32::MAX as usize,
    )?;
    let top_docs = nn.top_docs()?;
    assert_eq!(num_accepted, top_docs.score_docs.len());
    for node in &top_docs.score_docs {
      assert!(
        accept_ords.get(node.doc as usize)?,
        "the results include a deleted document: {node:?}"
      );
    }

    Ok(())
  }

  fn test_hnsw_graph_builder_initialization_from_graph_with_offset_zero<R>(
    &self,
    random: &mut R,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let total_size = at_least_usize(random, 100);
    let initializer_size = random.random_range(5..total_size);
    let doc_id_offset = 0usize;
    let dim = at_least_usize(random, 10);
    let seed = random.random::<u64>();

    let initializer_vectors = self.vector_values(initializer_size, dim, random);
    let initial_scorer_supplier =
      self.build_scorer_supplier(initializer_vectors.clone(), random)?;
    let mut initializer_builder =
      hnsw_graph_builder::create(initial_scorer_supplier, 10, 30, seed)?;

    let initializer_graph = initializer_builder.build(initializer_vectors.size())?;
    let mut final_vector_values = self.vector_values_with_pregenerated(
      total_size,
      dim,
      initializer_vectors,
      doc_id_offset,
      random,
    );
    let initializer_ord_map = create_offset_ordinal_map(
      initializer_size,
      &mut final_vector_values,
      doc_id_offset as i32,
    )?;
    let _size = final_vector_values.size();
    let final_scorer_supplier = self.build_scorer_supplier(final_vector_values, random)?;

    // We cannot call get_nodes_on_level before the graph reaches the size it claimed,
    // so create another graph for the equality assertion.
    let mut graph_after_init = initialized_hnsw_graph_builder::init_graph(
      10,
      initializer_graph,
      &initializer_ord_map,
      initializer_graph.size() as i32,
    )?;

    let mut initialized_nodes_it = RangeDISI::new(
      doc_id_offset as i32,
      (initializer_size + doc_id_offset) as i32,
    )?;
    let initialized_nodes =
      crate::core::util::bit_set::of(&mut initialized_nodes_it, total_size + 1)?;

    let mut final_builder = initialized_hnsw_graph_builder::from_graph(
      final_scorer_supplier,
      10,
      30,
      seed,
      initializer_graph,
      &initializer_ord_map,
      initialized_nodes,
      total_size as i32,
    )?;

    assert_graph_equal(initializer_graph, &mut graph_after_init)?;

    let final_graph = final_builder.build(total_size)?;
    assert_graph_contains_graph(final_graph, initializer_graph, &initializer_ord_map)?;

    Ok(())
  }

  fn test_hnsw_graph_builder_initialization_from_graph_with_non_zero_offset<R>(
    &self,
    random: &mut R,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let total_size = at_least_usize(random, 100);
    let initializer_size = random.random_range(5..total_size);
    let doc_id_offset = random.random_range(1..(total_size - initializer_size + 1));
    let dim = at_least_usize(random, 10);
    let seed = random.random::<u64>();

    let initializer_vectors = self.vector_values(initializer_size, dim, random);
    let initial_scorer_supplier =
      self.build_scorer_supplier(initializer_vectors.clone(), random)?;
    let mut initializer_builder =
      hnsw_graph_builder::create(initial_scorer_supplier, 10, 30, seed)?;

    let initializer_graph = initializer_builder.build(initializer_vectors.size())?;
    let mut final_vector_values = self.vector_values_with_pregenerated(
      total_size,
      dim,
      initializer_vectors.copy_()?,
      doc_id_offset,
      random,
    );
    let initializer_ord_map = create_offset_ordinal_map(
      initializer_size,
      &mut final_vector_values,
      doc_id_offset as i32,
    )?;

    let final_size = final_vector_values.size();
    let final_scorer_supplier = self.build_scorer_supplier(final_vector_values, random)?;
    let mut initialized_nodes_it = RangeDISI::new(
      doc_id_offset as i32,
      (initializer_size + doc_id_offset) as i32,
    )?;
    let initialized_nodes =
      crate::core::util::bit_set::of(&mut initialized_nodes_it, total_size + 1)?;

    let mut final_builder = initialized_hnsw_graph_builder::from_graph(
      final_scorer_supplier,
      10,
      30,
      seed,
      initializer_graph,
      &initializer_ord_map,
      initialized_nodes,
      total_size as i32,
    )?;

    assert_graph_initialized_from_graph(
      final_builder.get_graph(),
      initializer_graph,
      &initializer_ord_map,
    )?;

    let final_graph = final_builder.build(final_size)?;
    assert_graph_contains_graph(final_graph, initializer_graph, &initializer_ord_map)?;

    Ok(())
  }

  fn test_visited_limit<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let n_doc = 500;
    self.set_similarity_function(VectorSimilarityFunction::DotProduct);
    let vectors = self.circular_vector_values(n_doc);
    let scorer_supplier = self.build_scorer_supplier(vectors.clone(), random)?;
    let mut builder = hnsw_graph_builder::create(scorer_supplier, 16, 100, random.random::<u64>())?;
    let hnsw = builder.build(vectors.size())?;

    let top_k = 50;
    let visited_limit = top_k + random.random_range(0..5);
    let scorer = self.build_scorer(vectors, self.get_target_vector())?;
    let accept_ords = create_random_accept_ords(0, n_doc, random);
    let nn = hnsw_graph_searcher::search_with_top_k(
      &scorer,
      top_k,
      hnsw,
      Some(&accept_ords),
      visited_limit,
    )?;
    assert!(nn.early_terminated());
    // The visited count should not exceed the limit.
    assert!(nn.visited_count() <= visited_limit);

    Ok(())
  }

  fn test_hnsw_graph_builder_invalid<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let vectors = self.vector_values(1, 1, random);
    let scorer_supplier = self.build_scorer_supplier(vectors.clone(), random)?;
    let scorer_supplier2 = self.build_scorer_supplier(vectors, random)?;

    assert!(matches!(
      hnsw_graph_builder::create(scorer_supplier, 0, 10, 0),
      Err(LuceneError::IllegalArgument(_))
    ));
    // beam_width must be > 0.
    assert!(matches!(
      hnsw_graph_builder::create(scorer_supplier2, 10, 0, 0),
      Err(LuceneError::IllegalArgument(_))
    ));

    Ok(())
  }

  fn test_ram_usage_estimate<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO: memory calculation not implement
    // let size = at_least_usize(random, 2000);
    // let dim = random.random_range(100..=1024);
    // let m = random.random_range(4..=96);
    //
    // let vectors = self.vector_values(size, dim, random);
    // let scorer_supplier = self.build_scorer_supplier(vectors, random)?;
    // let mut builder = hnsw_graph_builder::create(scorer_supplier, m, m * 2, random.random::<u64>())?;
    // let hnsw = builder.build(size)?;
    //
    // // Rust currently exposes graph memory accounting via Accountable::ram_bytes_used.
    // let actual = hnsw.ram_bytes_used()?;
    // assert!(actual > 0, "ram_bytes_used should report a positive size");

    Ok(())
  }

  fn test_diversity<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.set_similarity_function(VectorSimilarityFunction::DotProduct);
    let mut values = Vec::with_capacity(7);
    for radians in [0.5_f64, 0.75, 0.2, 0.9, 0.8, 0.77, 0.6] {
      let mut value = [0.0_f32; 2];
      unit_vector_2d(radians, &mut value);
      values.push(value.to_vec());
    }
    let vectors = self.vector_values_from_values(values, random);
    let scorer_supplier = self.build_scorer_supplier(vectors, random)?;
    let mut builder = hnsw_graph_builder::create(scorer_supplier, 2, 10, random.random::<u64>())?;

    builder.add_graph_node(0)?;
    builder.add_graph_node(1)?;
    builder.add_graph_node(2)?;
    assert_level0_neighbors(builder.get_graph(), 0, &[1, 2])?;
    assert_level0_neighbors(builder.get_graph(), 1, &[0])?;
    assert_level0_neighbors(builder.get_graph(), 2, &[0])?;

    builder.add_graph_node(3)?;
    assert_level0_neighbors(builder.get_graph(), 0, &[1, 2])?;
    assert_level0_neighbors(builder.get_graph(), 1, &[0, 3])?;
    assert_level0_neighbors(builder.get_graph(), 2, &[0])?;
    assert_level0_neighbors(builder.get_graph(), 3, &[1])?;

    builder.add_graph_node(4)?;
    // 4 is the same distance from 0 that 2 is; keep the existing node in place.
    assert_level0_neighbors(builder.get_graph(), 0, &[1, 2])?;
    assert_level0_neighbors(builder.get_graph(), 1, &[0, 3, 4])?;
    assert_level0_neighbors(builder.get_graph(), 2, &[0])?;
    // 1 survives the diversity check.
    assert_level0_neighbors(builder.get_graph(), 3, &[1, 4])?;
    assert_level0_neighbors(builder.get_graph(), 4, &[1, 3])?;

    builder.add_graph_node(5)?;
    assert_level0_neighbors(builder.get_graph(), 0, &[1, 2])?;
    assert_level0_neighbors(builder.get_graph(), 1, &[0, 3, 4, 5])?;
    assert_level0_neighbors(builder.get_graph(), 2, &[0])?;
    // Even though 5 is closer, 3 is not a neighbor of 5, so no update to its neighbors occurs.
    assert_level0_neighbors(builder.get_graph(), 3, &[1, 4])?;
    assert_level0_neighbors(builder.get_graph(), 4, &[1, 3, 5])?;
    assert_level0_neighbors(builder.get_graph(), 5, &[1, 4])?;

    Ok(())
  }

  fn test_diversity_fallback<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.set_similarity_function(VectorSimilarityFunction::Euclidean);
    // Some test cases can't be exercised in two dimensions;
    // in particular if a new neighbor displaces an existing neighbor
    // by being closer to the target, yet none of the existing neighbors is closer to the new vector
    // than to the target -- ie they all remain diverse, so we simply drop the farthest one.
    let values = vec![
      vec![0.0, 0.0, 0.0],
      vec![0.0, 10.0, 0.0],
      vec![0.0, 0.0, 20.0],
      vec![10.0, 0.0, 0.0],
      vec![0.0, 4.0, 0.0],
    ];
    let vectors = self.vector_values_from_values(values, random);
    let scorer_supplier = self.build_scorer_supplier(vectors, random)?;
    let mut builder = hnsw_graph_builder::create(scorer_supplier, 1, 10, random.random::<u64>())?;

    builder.add_graph_node(0)?;
    builder.add_graph_node(1)?;
    builder.add_graph_node(2)?;
    assert_level0_neighbors(builder.get_graph(), 0, &[1, 2])?;
    // 2 is closer to 0 than 1, so it is excluded as non-diverse.
    assert_level0_neighbors(builder.get_graph(), 1, &[0])?;
    // 1 is closer to 0 than 2, so it is excluded as non-diverse.
    assert_level0_neighbors(builder.get_graph(), 2, &[0])?;

    builder.add_graph_node(3)?;
    // This is one case we are testing; 2 has been displaced by 3.
    assert_level0_neighbors(builder.get_graph(), 0, &[1, 3])?;
    assert_level0_neighbors(builder.get_graph(), 1, &[0])?;
    assert_level0_neighbors(builder.get_graph(), 2, &[0])?;
    assert_level0_neighbors(builder.get_graph(), 3, &[0])?;

    Ok(())
  }

  fn test_diversity_3d<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.set_similarity_function(VectorSimilarityFunction::Euclidean);
    let values = vec![
      vec![0.0, 0.0, 0.0],
      vec![0.0, 10.0, 0.0],
      vec![0.0, 0.0, 20.0],
      vec![0.0, 9.0, 0.0],
    ];
    let vectors = self.vector_values_from_values(values, random);
    let scorer_supplier = self.build_scorer_supplier(vectors, random)?;
    let mut builder = hnsw_graph_builder::create(scorer_supplier, 1, 10, random.random::<u64>())?;

    builder.add_graph_node(0)?;
    builder.add_graph_node(1)?;
    builder.add_graph_node(2)?;
    assert_level0_neighbors(builder.get_graph(), 0, &[1, 2])?;
    // 2 is closer to 0 than 1, so it is excluded as non-diverse.
    assert_level0_neighbors(builder.get_graph(), 1, &[0])?;
    // 1 is closer to 0 than 2, so it is excluded as non-diverse.
    assert_level0_neighbors(builder.get_graph(), 2, &[0])?;

    builder.add_graph_node(3)?;
    // This is one case we are testing; 1 has been displaced by 3.
    assert_level0_neighbors(builder.get_graph(), 0, &[2, 3])?;
    assert_level0_neighbors(builder.get_graph(), 1, &[0, 3])?;
    assert_level0_neighbors(builder.get_graph(), 2, &[0])?;
    assert_level0_neighbors(builder.get_graph(), 3, &[0, 1])?;

    Ok(())
  }

  fn test_random<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let size = at_least_usize(random, 100);
    let dim = at_least_usize(random, 10);
    let vectors = self.vector_values(size, dim, random);
    let top_k = 5;
    let scorer_supplier = self.build_scorer_supplier(vectors.clone(), random)?;
    let mut builder = hnsw_graph_builder::create(scorer_supplier, 10, 30, random.random::<u64>())?;
    let hnsw = builder.build(vectors.size())?;
    let accept_ords = if random.random_bool(0.5) {
      None
    } else {
      Some(create_random_accept_ords(0, size, random))
    };

    let mut total_matches = 0usize;
    for _ in 0..100 {
      let query = self.random_vector(random, dim);
      let scorer = self.build_scorer(vectors.clone(), query.clone())?;
      let mut actual = hnsw_graph_searcher::search_with_top_k(
        &scorer,
        100,
        hnsw,
        accept_ords.as_ref(),
        i32::MAX as usize,
      )?;
      let top_docs = actual.top_docs()?;

      let mut expected = NeighborQueue::new(top_k, false)?;
      for j in 0..size {
        if accept_ords
          .as_ref()
          .is_none_or(|bits| bits.get(j).expect(""))
        {
          let v = self.vector_value(&vectors, j)?;
          let score = self.score(&query, &v);
          expected.add(j, score);
          if expected.size() > top_k {
            let _ = expected.pop()?;
          }
        }
      }

      let actual_top_k_docs = top_docs
        .score_docs
        .iter()
        .take(top_k)
        .map(|doc| doc.doc as usize)
        .collect::<Vec<_>>();
      total_matches += compute_overlap(&actual_top_k_docs, &expected.nodes());
    }

    let overlap = total_matches as f64 / (100 * top_k) as f64;
    println!("overlap={overlap} total_matches={total_matches}");
    assert!(overlap > 0.9, "overlap={overlap}");

    Ok(())
  }

  fn test_on_heap_hnsw_graph_search<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO 多线程未实现
    Ok(())
  }

  fn test_concurrent_merge_builder<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO 多线程未实现
    Ok(())
  }

  fn test_all_nodes_visited_in_single_level<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let size = at_least_usize(random, 100);
    let dim = at_least_usize(random, 50);

    // Search for a large number of results.
    let top_k = size - 1;

    let doc_vectors = self.vector_values(size, dim, random);
    let scorer_supplier = self.build_scorer_supplier(doc_vectors.clone(), random)?;
    let mut builder = hnsw_graph_builder::create(scorer_supplier, 10, 30, random.random::<u64>())?;
    let graph = builder.build(size)?;

    let mut single_level_graph = DelegateHnswGraph::new(graph);

    let query_vectors = self.vector_values(1, dim, random);
    let v = self.vector_value(&query_vectors, 0)?;
    let query_scorer = self.build_scorer(doc_vectors, v)?;

    let mut collector = TopKnnCollector::new(top_k, i32::MAX as usize)?;
    hnsw_graph_searcher::search(
      &query_scorer,
      &mut collector,
      &mut single_level_graph,
      None::<&FixedBitSet>,
    )?;

    // Check that we visit all nodes.
    assert_eq!(graph.size(), collector.visited_count());

    Ok(())
  }
}
struct DelegateHnswGraph<'a, H>
where
  H: HnswGraph,
{
  delegate: &'a mut H,
}
impl<'a, H> DelegateHnswGraph<'a, H>
where
  H: HnswGraph,
{
  pub fn new(delegate: &'a mut H) -> Self {
    Self { delegate }
  }
}
impl<H> HnswGraph for DelegateHnswGraph<'_, H>
where
  H: HnswGraph,
{
  fn seek(&mut self, level: usize, target: usize) -> Result<()> {
    self.delegate.seek(level, target)
  }

  fn size(&self) -> usize {
    self.delegate.size()
  }

  fn max_node_id(&self) -> Option<usize> {
    self.delegate.max_node_id()
  }

  fn next_neighbor(&mut self) -> Result<usize> {
    self.delegate.next_neighbor()
  }

  fn num_levels(&self) -> Result<usize> {
    Ok(1)
  }

  fn entry_node(&self) -> Result<Option<usize>> {
    self.delegate.entry_node()
  }

  type NodeIterator = H::NodeIterator;

  fn get_nodes_on_level(&mut self, level: usize) -> Result<Self::NodeIterator> {
    self.delegate.get_nodes_on_level(level)
  }

  fn get_neighbors_mut(&mut self, level: usize, node: usize) -> Result<&mut NeighborArray> {
    self.delegate.get_neighbors_mut(level, node)
  }

  fn get_neighbors(&self, level: usize, node: usize) -> Result<&NeighborArray> {
    self.delegate.get_neighbors(level, node)
  }
}

#[derive(Clone)]
pub struct CircularByteVectorValues {
  size: usize,
  doc: i32,
}

impl CircularByteVectorValues {
  pub fn new(size: usize) -> Self {
    Self { size, doc: -1 }
  }

  fn vector_value_bytes(&self, ord: usize) -> Vec<u8> {
    let mut value = [0.0_f32; 2];
    unit_vector_2d(ord as f64 / self.size as f64, &mut value);
    value
      .into_iter()
      .map(|component| (component * 127.0) as i8 as u8)
      .collect()
  }
}

impl KnnVectorValues for CircularByteVectorValues {
  fn dimension(&self) -> usize {
    2
  }

  fn size(&self) -> usize {
    self.size
  }

  type KnnVectorValues = Self;

  fn copy(&self) -> Result<Self::KnnVectorValues> {
    Ok(self.clone())
  }

  fn get_encoding(&self) -> VectorEncoding {
    ByteVectorValues::get_encoding(self)
  }

  type Bits<'a, B>
    = BitsImpl1<B>
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    self.default_get_accept_ords(accept_docs)
  }

  type DocIndexIterator = DummyDocIndexIterator;
}

impl ByteVectorValues for CircularByteVectorValues {
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    Ok(Cow::Owned(VectorValueEnum::Byte(
      self.vector_value_bytes(ord),
    )))
  }

  type ByteVectorValues = Self;

  fn byte_copy(&self) -> Result<Option<Self::ByteVectorValues>> {
    Ok(Some(self.clone()))
  }

  type VectorScorer = DummyVectorScorer;
}

fn unit_vector_2d(pi_radians: f64, value: &mut [f32; 2]) {
  value[0] = (std::f64::consts::PI * pi_radians).cos() as f32;
  value[1] = (std::f64::consts::PI * pi_radians).sin() as f32;
}

#[derive(Clone)]
pub struct CircularFloatVectorValues {
  size: usize,
}

impl CircularFloatVectorValues {
  pub fn new(size: usize) -> Self {
    Self { size }
  }

  fn vector_value_f32(&self, ord: usize) -> Vec<f32> {
    let mut value = [0.0_f32; 2];
    unit_vector_2d(ord as f64 / self.size as f64, &mut value);
    value.to_vec()
  }
}

impl KnnVectorValues for CircularFloatVectorValues {
  fn dimension(&self) -> usize {
    2
  }

  fn size(&self) -> usize {
    self.size
  }

  type KnnVectorValues = Self;

  fn copy(&self) -> Result<Self::KnnVectorValues> {
    Ok(self.clone())
  }

  fn get_encoding(&self) -> VectorEncoding {
    FloatVectorValues::get_encoding(self)
  }

  type Bits<'a, B>
    = BitsImpl1<B>
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    self.default_get_accept_ords(accept_docs)
  }

  type DocIndexIterator = DenseDocIndexIterator;

  fn iterator(&mut self) -> Result<Self::DocIndexIterator> {
    Ok(create_dense_iterator(self.size as i32))
  }
}

impl FloatVectorValues for CircularFloatVectorValues {
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    Ok(Cow::Owned(VectorValueEnum::Float(
      self.vector_value_f32(ord),
    )))
  }

  type FloatVectorValues = Self;

  fn float_copy(&self) -> Result<Option<Self::FloatVectorValues>> {
    Ok(Some(self.clone()))
  }

  type VectorScorer = DummyVectorScorer;
}

pub fn sorted_nodes_on_level<G>(graph: &mut G, level: usize) -> Result<Vec<usize>>
where
  G: HnswGraph,
{
  let mut nodes_on_level = graph.get_nodes_on_level(level)?;
  let mut nodes = Vec::with_capacity(nodes_on_level.size());
  while nodes_on_level.has_next() {
    nodes.push(nodes_on_level.next().unwrap());
  }
  nodes.sort_unstable();
  Ok(nodes)
}

pub fn assert_graph_equal<G, H>(g: &mut G, h: &mut H) -> Result<()>
where
  G: HnswGraph,
  H: HnswGraph,
{
  let g_num_levels = g.num_levels()?;
  let h_num_levels = h.num_levels()?;
  assert_eq!(
    g_num_levels, h_num_levels,
    "the number of levels in the graphs are different"
  );
  assert_eq!(
    g.size(),
    h.size(),
    "the number of nodes in the graphs are different"
  );

  for level in 0..g_num_levels {
    let h_nodes = sorted_nodes_on_level(h, level)?;
    let g_nodes = sorted_nodes_on_level(g, level)?;
    assert_eq!(
      g_nodes, h_nodes,
      "nodes in the graphs are different on level {level}"
    );
  }

  for level in 0..g_num_levels {
    let g_nodes = sorted_nodes_on_level(g, level)?;
    for node in g_nodes {
      g.seek(level, node)?;
      h.seek(level, node)?;
      assert_eq!(
        get_neighbor_nodes(g)?,
        get_neighbor_nodes(h)?,
        "arcs differ for node {node} on level {level}"
      );
    }
  }

  Ok(())
}

pub fn assert_graph_contains_graph<G, H>(
  graph: &mut G,
  initializer: &mut H,
  new_ordinals: &[usize],
) -> Result<()>
where
  G: HnswGraph,
  H: HnswGraph,
{
  for level in 0..initializer.num_levels()? {
    let final_graph_nodes_on_level = nodes_iterator_to_array(graph.get_nodes_on_level(level)?);
    let initializer_graph_nodes_on_level = map_array_and_sort(
      &nodes_iterator_to_array(initializer.get_nodes_on_level(level)?),
      new_ordinals,
    );
    let overlap = compute_overlap(
      &final_graph_nodes_on_level,
      &initializer_graph_nodes_on_level,
    );
    assert_eq!(initializer_graph_nodes_on_level.len(), overlap);
  }
  Ok(())
}

pub fn assert_graph_initialized_from_graph<G, H>(
  graph: &mut G,
  initializer: &mut H,
  new_ordinals: &[usize],
) -> Result<()>
where
  G: HnswGraph,
  H: HnswGraph,
{
  assert_eq!(
    initializer.num_levels()?,
    graph.num_levels()?,
    "the number of levels in the graphs are different!"
  );
  assert_eq!(
    initializer.size(),
    graph.size(),
    "the number of nodes in the graphs are different!"
  );

  for level in 0..graph.num_levels()? {
    let nodes_on_level = nodes_iterator_to_array(initializer.get_nodes_on_level(level)?);
    for node in nodes_on_level {
      graph.seek(level, new_ordinals[node])?;
      initializer.seek(level, node)?;
      let expected_neighbors: HashSet<usize> = get_neighbor_nodes(initializer)?
        .into_iter()
        .map(|neighbor| new_ordinals[neighbor])
        .collect();
      assert_eq!(
        get_neighbor_nodes(graph)?,
        expected_neighbors,
        "arcs differ for node {node}"
      );
    }
  }

  Ok(())
}

pub fn assert_level0_neighbors(
  graph: &OnHeapHnswGraph,
  node: usize,
  expected: &[usize],
) -> Result<()> {
  let mut expected = expected.to_vec();
  expected.sort_unstable();

  let nn = graph.get_neighbors(0, node)?;
  let mut actual = nn.nodes()[..nn.size()].to_vec();
  actual.sort_unstable();

  assert_eq!(
    expected, actual,
    "expected: {:?} actual: {:?}",
    expected, actual
  );
  Ok(())
}

pub fn nodes_iterator_to_array<I>(mut nodes_iterator: I) -> Vec<usize>
where
  I: NodesIterator,
{
  let mut arr = Vec::with_capacity(nodes_iterator.size());
  while nodes_iterator.has_next() {
    arr.push(nodes_iterator.next().unwrap());
  }
  arr
}

pub fn map_array_and_sort(arr: &[usize], offset: &[usize]) -> Vec<usize> {
  let mut mapped = arr.iter().map(|value| offset[*value]).collect::<Vec<_>>();
  mapped.sort_unstable();
  mapped
}

pub fn create_offset_ordinal_map<T>(
  doc_id_size: usize,
  total_vector_values: &mut T,
  doc_id_offset: i32,
) -> Result<Vec<usize>>
where
  T: KnnVectorValues,
{
  let mut ordinal_offset = 0usize;
  let mut iterator = total_vector_values.iterator()?;
  while iterator.next_doc()? < doc_id_offset {
    ordinal_offset += 1;
  }

  let mut offset_ordinal_map = vec![0; doc_id_size];
  let upper_doc = doc_id_offset + doc_id_size as i32;
  let mut curr = 0usize;
  while iterator.doc_id() < upper_doc {
    offset_ordinal_map[curr] = ordinal_offset + curr;
    curr += 1;
    let _ = iterator.next_doc()?;
  }

  Ok(offset_ordinal_map)
}

fn compute_overlap(left: &[usize], right: &[usize]) -> usize {
  let mut left = left.to_vec();
  let mut right = right.to_vec();
  left.sort_unstable();
  right.sort_unstable();

  let mut overlap = 0usize;
  let mut i = 0usize;
  let mut j = 0usize;
  while i < left.len() && j < right.len() {
    if left[i] == right[j] {
      overlap += 1;
      i += 1;
      j += 1;
    } else if left[i] > right[j] {
      j += 1;
    } else {
      i += 1;
    }
  }

  overlap
}

pub fn get_neighbor_nodes<G>(graph: &mut G) -> Result<HashSet<usize>>
where
  G: HnswGraph,
{
  let mut neighbors = HashSet::new();
  loop {
    let neighbor = graph.next_neighbor()?;
    if neighbor == NO_MORE_DOCS as usize {
      break;
    }
    neighbors.insert(neighbor);
  }
  Ok(neighbors)
}

pub fn assert_byte_vectors_equal<U, V>(u: &U, v: &V) -> Result<()>
where
  U: ByteVectorValues,
  V: ByteVectorValues,
{
  assert_eq!(u.size(), v.size());
  for ord in 0..u.size() {
    let u_doc = u.ord_to_doc(ord)?;
    let v_doc = v.ord_to_doc(ord)?;
    assert_eq!(u_doc, v_doc);
    assert_ne!(NO_MORE_DOCS, u_doc as i32);

    let u_vec = u.vector_value(ord)?;
    let v_vec = v.vector_value(ord)?;
    let u_bytes = u_vec.as_ref().as_bytes()?;
    let v_bytes = v_vec.as_ref().as_bytes()?;
    assert_eq!(u_bytes, v_bytes, "vectors do not match for doc={u_doc}");
  }
  Ok(())
}

pub fn assert_float_vectors_equal<U, V>(u: &U, v: &V) -> Result<()>
where
  U: FloatVectorValues,
  V: FloatVectorValues,
{
  assert_eq!(u.size(), v.size());
  for ord in 0..u.size() {
    let u_doc = u.ord_to_doc(ord)?;
    let v_doc = v.ord_to_doc(ord)?;
    assert_eq!(u_doc, v_doc);
    assert_ne!(NO_MORE_DOCS, u_doc as i32);

    let u_vec = u.vector_value(ord)?;
    let v_vec = v.vector_value(ord)?;
    let u_floats = u_vec.as_ref().as_floats()?;
    let v_floats = v_vec.as_ref().as_floats()?;
    assert_eq!(
      u_floats.len(),
      v_floats.len(),
      "vectors do not match for doc={u_doc}"
    );
    for (lhs, rhs) in u_floats.iter().zip(v_floats.iter()) {
      assert!(
        (lhs - rhs).abs() <= 1e-4,
        "vectors do not match for doc={u_doc}: left={u_floats:?}, right={v_floats:?}"
      );
    }
  }
  Ok(())
}

pub fn create_random_float_vectors<R>(
  size: usize,
  dimension: usize,
  random: &mut R,
) -> Vec<Vec<f32>>
where
  R: Rng + ?Sized,
{
  (0..size)
    .map(|_| random_vector(random, dimension))
    .collect()
}

pub fn create_random_byte_vectors<R>(size: usize, dimension: usize, random: &mut R) -> Vec<Vec<u8>>
where
  R: Rng + ?Sized,
{
  (0..size)
    .map(|_| random_vector8(random, dimension))
    .collect()
}

pub fn create_random_accept_ords<R>(
  start_index: usize,
  length: usize,
  random: &mut R,
) -> FixedBitSet
where
  R: Rng + ?Sized,
{
  let mut bits = FixedBitSet::new(length);
  for i in 0..start_index.min(length) {
    bits.set(i);
  }

  for i in start_index.min(length)..length {
    if random.random::<f32>() < 0.667 {
      bits.set(i);
    }
  }

  bits
}

pub fn random_vector<R>(random: &mut R, dim: usize) -> Vec<f32>
where
  R: Rng + ?Sized,
{
  let mut vec = vec![0.0; dim];
  for value in &mut vec {
    *value = random.random::<f32>();
    if random.random_bool(0.5) {
      *value = -*value;
    }
  }
  VectorUtil::l2normalize(&mut vec).expect("random_vector should not generate a zero vector");
  vec
}

pub fn random_vector8<R>(random: &mut R, dim: usize) -> Vec<u8>
where
  R: Rng + ?Sized,
{
  random_vector(random, dim)
    .into_iter()
    .map(|component| (component * 127.0) as i8 as u8)
    .collect()
}

pub type TestsCircularKnnVectorValues =
  KnnVectorValuesEnm2<CircularByteVectorValues, CircularFloatVectorValues>;
impl Clone for TestsCircularKnnVectorValues {
  fn clone(&self) -> Self {
    match self {
      KnnVectorValuesEnm2::A(v) => KnnVectorValuesEnm2::A(v.clone()),
      KnnVectorValuesEnm2::B(v) => KnnVectorValuesEnm2::B(v.clone()),
    }
  }
}
pub type TestsKnnVectorValues = KnnVectorValuesEnm2<MockByteVectorValues, MockVectorValues>;
impl Clone for TestsKnnVectorValues {
  fn clone(&self) -> Self {
    match self {
      TestsKnnVectorValues::A(v) => TestsKnnVectorValues::A(v.clone()),
      TestsKnnVectorValues::B(v) => TestsKnnVectorValues::B(v.clone()),
    }
  }
}
impl TestsKnnVectorValues {
  fn copy_(&self) -> Result<TestsKnnVectorValues> {
    match self {
      TestsKnnVectorValues::A(v) => Ok(TestsKnnVectorValues::A(v.byte_copy()?.unwrap())),
      TestsKnnVectorValues::B(v) => Ok(TestsKnnVectorValues::B(v.float_copy()?.unwrap())),
    }
  }
}

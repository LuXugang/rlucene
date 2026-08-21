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
use crate::core::codecs::hnsw::hnsw_graph_provider::HnswGraphProvider;
use crate::core::codecs::lucene99::lucene99_hnsw_scalar_quantized_vectors_format::Lucene99HnswScalarQuantizedVectorsFormat;
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_format::{
  DEFAULT_BEAM_WIDTH, DEFAULT_MAX_CONN, Lucene99HnswVectorsFormat,
};
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::knn_float_vector_field::KnnFloatVectorField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::BytesRef;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::directory_reader;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::knn_vector_values::{DocIndexIterator, KnnVectorValues};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::knn_float_vector_query::KnnFloatVectorQuery;
use crate::core::search::searcher_factory::SearcherFactory;
use crate::core::search::searcher_manager::SearcherManager;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::TryIntoInt;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::hnsw_graph::{HnswGraph, NodesIterator};
use crate::core::util::hnsw::hnsw_graph_builder::TestRandSeedGuard;
use crate::core::util::vector_util::VectorUtil;
use crate::test_framework::core::codecs::asserting_codec::AssertingCodec;
use crate::test_framework::core::util::lucene_test_case::{
  at_least_usize, new_directory_shared, new_index_writer_config, new_searcher_with_reader, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::prelude::StdRng;
use rand::{Rng, RngExt};
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Barrier};
use std::thread;

const KNN_GRAPH_FIELD: &str = "vector";

/// Tests indexing of a knn-graph.
struct TestKnnGraph {
  _rand_seed_guard: TestRandSeedGuard,
  m: usize,
  codec: AssertingCodec,
  float32_codec: AssertingCodec,
  vector_encoding: VectorEncoding,
  similarity_function: VectorSimilarityFunction,
}

impl TestKnnGraph {
  fn new<R>(random: &mut R) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    let rand_seed_guard = TestRandSeedGuard::new(random.random::<u64>());
    let m = if random.random_bool(0.5) {
      random.random_range(3..259)
    } else {
      DEFAULT_MAX_CONN
    };
    let similarity_functions = [
      VectorSimilarityFunction::DotProduct,
      VectorSimilarityFunction::Cosine,
      VectorSimilarityFunction::MaximumInnerProduct,
    ];
    let similarity_function =
      similarity_functions[random.random_range(0..similarity_functions.len())];
    let vector_encoding = if random.random_bool(0.5) {
      VectorEncoding::BYTE(1)
    } else {
      VectorEncoding::FLOAT32(4)
    };
    let codec = if random.random_bool(0.5) {
      TestUtil::always_knn_vectors_format(
        Lucene99HnswScalarQuantizedVectorsFormat::with_graph_para(m, DEFAULT_BEAM_WIDTH)?,
      )
    } else {
      TestUtil::always_knn_vectors_format(Lucene99HnswVectorsFormat::with_graph_para(
        m,
        DEFAULT_BEAM_WIDTH,
      )?)
    };
    let float32_codec = TestUtil::always_knn_vectors_format(
      Lucene99HnswVectorsFormat::with_graph_para(m, DEFAULT_BEAM_WIDTH)?,
    );
    Ok(Self {
      _rand_seed_guard: rand_seed_guard,
      m,
      codec,
      float32_codec,
      vector_encoding,
      similarity_function,
    })
  }

  fn random_vector<R>(&self, random: &mut R, dimension: usize) -> Result<Vec<f32>>
  where
    R: Rng + ?Sized,
  {
    let mut value = (0..dimension)
      .map(|_| random.random::<f32>())
      .collect::<Vec<_>>();
    VectorUtil::l2normalize(&mut value)?;
    if matches!(self.vector_encoding, VectorEncoding::BYTE(_)) {
      for v in &mut value {
        *v = (*v * 127.0).trunc();
      }
    }
    Ok(value)
  }

  fn random_vectors<R>(
    &self,
    random: &mut R,
    num_doc: usize,
    dimension: usize,
  ) -> Result<Vec<Option<Vec<f32>>>>
  where
    R: Rng + ?Sized,
  {
    (0..num_doc)
      .map(|_| {
        if random.random_bool(0.5) {
          self.random_vector(random, dimension).map(Some)
        } else {
          Ok(None)
        }
      })
      .collect()
  }

  fn add(
    &self,
    writer: &Arc<IndexWriter<crate::core::store::directory::DirEnum>>,
    id: usize,
    vector: Option<&[f32]>,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<()> {
    let mut doc = Document::new();
    if let Some(vector) = vector {
      doc.add(KnnFloatVectorField::with_similarity_function(
        KNN_GRAPH_FIELD,
        vector.to_vec(),
        similarity_function,
      )?);
    }
    let id_string = id.to_string();
    doc.add(StringField::from_string(
      "id",
      id_string.clone(),
      Store::Yes,
    )?);
    doc.add(SortedDocValuesField::new(
      "id",
      BytesRef::from_string(&id_string),
    ));
    writer.update_document_with_term(Term::from_text("id", &id_string), doc)?;
    Ok(())
  }

  fn assert_consistent_graph(
    &self,
    writer: &Arc<IndexWriter<crate::core::store::directory::DirEnum>>,
    values: &[Option<Vec<f32>>],
    vector_field: &str,
  ) -> Result<()> {
    let reader = directory_reader::open_from_writer(writer)?;
    let mut num_docs_with_vectors = 0;
    for context in (&reader).get_context()?.leaves()? {
      let leaf = context.reader();
      let Some(vector_values) = LeafReader::get_float_vector_values(&leaf, vector_field)? else {
        continue;
      };
      let mut stored_fields = IndexReader::stored_fields(&leaf)?;
      let mut iterator = vector_values.iterator()?;
      let mut doc = iterator.next_doc()?;
      while doc != crate::core::search::doc_id_set_iterator::NO_MORE_DOCS {
        let id = stored_fields
          .document(doc)?
          .get("id")?
          .expect("stored id")
          .as_ref()
          .parse::<usize>()
          .expect("numeric stored id");
        let expected = values[id].as_ref().expect("document should have a vector");
        let actual = vector_values.vector_value(iterator.index()?.try_convert()?)?;
        assert_eq!(expected.as_slice(), actual.as_ref().as_floats()?);
        num_docs_with_vectors += 1;
        doc = iterator.next_doc()?;
      }

      let vector_reader = leaf
        .get_vector_reader()?
        .expect("vector reader should exist");
      let mut graph = vector_reader.get_graph(vector_field)?;
      for level in 0..graph.num_levels()? {
        let max_conn_on_level = if level == 0 { self.m * 2 } else { self.m };
        let mut graph_on_level = vec![None; graph.size()];
        let nodes = graph.get_nodes_on_level(level)?;
        let expected_count = nodes.size();
        let mut count_on_level = 0;
        let mut found_orphan = false;
        for node in nodes {
          graph.seek(level, node)?;
          let mut friends = Vec::new();
          loop {
            let arc = graph.next_neighbor()?;
            if arc == crate::core::search::doc_id_set_iterator::NO_MORE_DOCS as usize {
              break;
            }
            friends.push(arc);
          }
          if friends.is_empty() {
            found_orphan = true;
          } else {
            graph_on_level[node] = Some(friends);
          }
          count_on_level += 1;
        }
        assert_eq!(expected_count, count_on_level);
        assert_ne!(0, count_on_level, "No nodes on level [{level}]");
        if count_on_level == 1 {
          assert!(found_orphan);
        } else {
          assert!(!found_orphan, "Graph has orphan nodes on level [{level}]");
          if max_conn_on_level > count_on_level {
            Self::assert_connected(&graph_on_level);
          } else {
            Self::assert_max_conn(&graph_on_level, max_conn_on_level);
          }
        }
      }
    }
    let expected_num_docs_with_vectors = values.iter().filter(|value| value.is_some()).count();
    assert_eq!(expected_num_docs_with_vectors, num_docs_with_vectors);
    reader.close()
  }

  fn assert_max_conn(graph: &[Option<Vec<usize>>], max_conn: usize) {
    for friends in graph.iter().flatten() {
      assert!(friends.len() <= max_conn);
      for &neighbor in friends {
        assert!(graph[neighbor].is_some());
      }
    }
  }

  /// Assert that every node is reachable from some other node.
  fn assert_connected(graph: &[Option<Vec<usize>>]) {
    let nodes = graph
      .iter()
      .enumerate()
      .filter_map(|(node, friends)| friends.as_ref().map(|_| node))
      .collect::<Vec<_>>();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::from([nodes[0]]);
    while let Some(node) = queue.pop_front() {
      if !visited.insert(node) {
        continue;
      }
      for &neighbor in graph[node].as_ref().expect("expected neighbors") {
        if !visited.contains(&neighbor) {
          queue.push_back(neighbor);
        }
      }
    }
    for node in nodes {
      assert!(visited.contains(&node));
    }
  }

  fn index_data<R>(
    &self,
    writer: &Arc<IndexWriter<crate::core::store::directory::DirEnum>>,
    random: &mut R,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let n = 5;
    let step_size = 17;
    let mut values = vec![None; n * n];
    let mut index = 0;
    for (i, value) in values.iter_mut().enumerate() {
      let x = index % n;
      let y = index / n;
      *value = Some(vec![x as f32, y as f32]);
      index = (index + step_size) % (n * n);
      self.add(writer, i, value.as_deref(), self.similarity_function)?;
      if i == 13 {
        writer.commit()?;
      }
    }
    if random.random_bool(0.5) {
      writer.force_merge(1)?;
    }
    self.assert_consistent_graph(writer, &values, KNN_GRAPH_FIELD)
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&mut TestKnnGraph, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let mut case = TestKnnGraph::new(&mut random)?;
  f(&mut case, &mut random)
}

/// Basic test of creating documents in a graph.
#[test]
fn test_basic() -> Result<()> {
  run_case(|case, random| {
    let dir = new_directory_shared(random)?;
    let mut config = new_index_writer_config(random)?;
    config.set_codec(case.codec.clone());
    let writer = IndexWriter::new(dir.clone(), config)?;
    let num_doc = at_least_usize(random, 10);
    let dimension = at_least_usize(random, 3);
    let mut values = vec![None; num_doc];
    for (i, value) in values.iter_mut().enumerate() {
      if random.random_bool(0.5) {
        *value = Some(case.random_vector(random, dimension)?);
      }
      case.add(&writer, i, value.as_deref(), case.similarity_function)?;
    }
    case.assert_consistent_graph(&writer, &values, KNN_GRAPH_FIELD)?;
    writer.close()?;
    dir.close()
  })
}

#[test]
fn test_single_document() -> Result<()> {
  run_case(|case, random| {
    let dir = new_directory_shared(random)?;
    let mut config = new_index_writer_config(random)?;
    config.set_codec(case.codec.clone());
    let writer = IndexWriter::new(dir.clone(), config)?;
    let mut value = vec![0.0, 1.0, 2.0];
    if case.similarity_function == VectorSimilarityFunction::DotProduct {
      VectorUtil::l2normalize(&mut value)?;
    }
    if matches!(case.vector_encoding, VectorEncoding::BYTE(_)) {
      for v in &mut value {
        *v = (*v * 127.0).floor();
      }
    }
    let values = vec![Some(value)];
    case.add(&writer, 0, values[0].as_deref(), case.similarity_function)?;
    case.assert_consistent_graph(&writer, &values, KNN_GRAPH_FIELD)?;
    writer.commit()?;
    case.assert_consistent_graph(&writer, &values, KNN_GRAPH_FIELD)?;
    writer.close()?;
    dir.close()
  })
}

/// Verify that the graph properties are preserved when merging.
#[test]
fn test_merge() -> Result<()> {
  run_case(|case, random| {
    let dir = new_directory_shared(random)?;
    let mut config = new_index_writer_config(random)?;
    config.set_codec(case.codec.clone());
    let writer = IndexWriter::new(dir.clone(), config)?;
    let num_doc = at_least_usize(random, 100);
    let dimension = at_least_usize(random, 10);
    let mut values = case.random_vectors(random, num_doc, dimension)?;
    for (i, value) in values.iter_mut().enumerate() {
      if random.random_bool(0.5) {
        *value = Some(case.random_vector(random, dimension)?);
      }
      case.add(&writer, i, value.as_deref(), case.similarity_function)?;
      if random.random_range(0..10) == 3 {
        writer.commit()?;
      }
    }
    if random.random_bool(0.5) {
      writer.force_merge(1)?;
    }
    case.assert_consistent_graph(&writer, &values, KNN_GRAPH_FIELD)?;
    writer.close()?;
    dir.close()
  })
}

/// Test writing and reading of multiple vector fields.
#[test]
fn test_multiple_vector_fields() -> Result<()> {
  run_case(|case, random| {
    let num_vector_fields = random.random_range(2..=5);
    let num_doc = at_least_usize(random, 100);
    let mut values = Vec::with_capacity(num_vector_fields);
    for _ in 0..num_vector_fields {
      let dimension = at_least_usize(random, 3);
      values.push(case.random_vectors(random, num_doc, dimension)?);
    }
    let dir = new_directory_shared(random)?;
    let mut config = new_index_writer_config(random)?;
    config.set_codec(case.codec.clone());
    let writer = IndexWriter::new(dir.clone(), config)?;
    for doc_id in 0..num_doc {
      let mut doc = Document::new();
      for (field, field_values) in values.iter().enumerate() {
        if let Some(vector) = &field_values[doc_id] {
          doc.add(KnnFloatVectorField::with_similarity_function(
            &format!("{KNN_GRAPH_FIELD}{field}"),
            vector.clone(),
            case.similarity_function,
          )?);
        }
      }
      doc.add(StringField::from_string(
        "id",
        doc_id.to_string(),
        Store::Yes,
      )?);
      writer.add_document(doc)?;
    }
    for (field, field_values) in values.iter().enumerate() {
      case.assert_consistent_graph(&writer, field_values, &format!("{KNN_GRAPH_FIELD}{field}"))?;
    }
    writer.close()?;
    dir.close()
  })
}

/// Verify that searching does something reasonable.
#[test]
fn test_search() -> Result<()> {
  run_case(|case, random| {
    // We can't use dot product here since the vectors are laid out on a grid, not a sphere.
    case.similarity_function = VectorSimilarityFunction::Euclidean;
    let dir = new_directory_shared(random)?;
    let mut config = new_index_writer_config(random)?;
    config.set_codec(case.float32_codec.clone());
    let writer = IndexWriter::new(dir.clone(), config)?;
    case.index_data(&writer, random)?;
    let reader = directory_reader::open_from_writer(&writer)?;
    let searcher = new_searcher_with_reader(reader)?;
    for (expected, target) in [
      (vec![0, 15, 3, 18, 5], vec![0.0, 0.1]),
      (vec![15, 18, 0, 3, 5], vec![0.3, 0.8]),
    ] {
      let mut top_docs =
        searcher.search(KnnFloatVectorQuery::new(KNN_GRAPH_FIELD, target, 5)?, 5)?;
      let mut stored_fields = searcher.stored_fields()?;
      for score_doc in &mut top_docs.score_docs {
        score_doc.doc = stored_fields
          .document(score_doc.doc)?
          .get("id")?
          .expect("stored id")
          .as_ref()
          .parse::<i32>()
          .expect("numeric stored id");
      }
      assert_eq!(expected.len(), top_docs.score_docs.len());
      for (expected, actual) in expected.iter().zip(&top_docs.score_docs) {
        assert_eq!(*expected, actual.doc);
      }
    }
    searcher.get_index_reader().close()?;
    writer.close()?;
    dir.close()
  })
}

#[test]
fn test_multi_threaded_search() -> Result<()> {
  run_case(|case, random| {
    case.similarity_function = VectorSimilarityFunction::Euclidean;
    let dir = new_directory_shared(random)?;
    let mut config = new_index_writer_config(random)?;
    config.set_codec(case.float32_codec.clone());
    let writer = IndexWriter::new(dir.clone(), config)?;
    case.index_data(&writer, random)?;

    let manager = Arc::new(SearcherManager::from_writer(
      &writer,
      Some(SearcherFactory::new()),
    )?);
    let thread_count = random.random_range(2..=5);
    let barrier = Arc::new(Barrier::new(thread_count + 1));
    let mut threads = Vec::with_capacity(thread_count);
    for _ in 0..thread_count {
      let manager = Arc::clone(&manager);
      let barrier = Arc::clone(&barrier);
      threads.push(thread::spawn(move || -> Result<()> {
        barrier.wait();
        let searcher = manager.acquire()?;
        let mut results = searcher.search(
          KnnFloatVectorQuery::new(KNN_GRAPH_FIELD, vec![0.0, 0.1], 5)?,
          5,
        )?;
        let mut stored_fields = searcher.stored_fields()?;
        for score_doc in &mut results.score_docs {
          score_doc.doc = stored_fields
            .document(score_doc.doc)?
            .get("id")?
            .expect("stored id")
            .as_ref()
            .parse::<i32>()
            .expect("numeric stored id");
        }
        let expected = [0, 15, 3, 18, 5];
        assert_eq!(expected.len(), results.score_docs.len());
        for (expected, actual) in expected.iter().zip(&results.score_docs) {
          assert_eq!(*expected, actual.doc);
        }
        manager.release(searcher)
      }));
    }
    barrier.wait();
    for thread in threads {
      thread.join().expect("search thread panicked")?;
    }
    manager.close()?;
    writer.close()?;
    dir.close()
  })
}

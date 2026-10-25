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
use rlucene::core::{
  codecs::hnsw::hnsw_graph_provider::HnswGraphProvider,
  index::{
    codec_reader::CodecReader, directory_reader, float_vector_values::FloatVectorValues,
    index_reader::IndexReader, index_reader_context::IndexReaderContext,
    indexable_field::IndexableField, knn_vector_values::KnnVectorValues, leaf_reader::LeafReader,
    stored_fields::StoredFields,
  },
  store::mmap_directory::MMapDirectory,
  util::{close::CloseableRef, hnsw::hnsw_graph::HnswGraph},
};
use std::{
  fs::File,
  io::{BufWriter, Write},
  path::Path,
  sync::Arc,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn main() -> Result<()> {
  let args: Vec<_> = std::env::args().collect();
  let dim: usize = std::env::var("RLUCENE_BENCH_DIM")?.parse()?;
  let ndoc: usize = std::env::var("RLUCENE_BENCH_NDOC")?.parse()?;
  let dir = Arc::new(MMapDirectory::new(Path::new(&args[1]))?);
  let reader = Arc::new(directory_reader::open(dir.clone())?);
  let context = reader.clone().get_context()?;
  let leaves = context.leaves()?;
  assert_eq!(reader.num_docs()? as usize, ndoc);
  assert_eq!(leaves.len(), 1);
  assert!(!reader.has_deletions()?);
  let leaf = leaves[0].reader();
  let field_infos = leaf.get_field_infos()?;
  let field = field_infos
    .field_info_by_name("knn")?
    .ok_or("missing field")?;
  let expected_similarity = match std::env::var("RLUCENE_BENCH_SIMILARITY")?.as_str() {
    "EUCLIDEAN" => {
      rlucene::core::index::vector_similarity_function::VectorSimilarityFunction::Euclidean
    },
    "DOT_PRODUCT" => {
      rlucene::core::index::vector_similarity_function::VectorSimilarityFunction::DotProduct
    },
    _ => return Err("unsupported similarity".into()),
  };
  if field.get_vector_similarity_function() != &expected_similarity {
    return Err("wrong similarity".into());
  }
  let values = LeafReader::get_float_vector_values(&leaf, "knn")?.ok_or("missing vectors")?;
  assert_eq!(values.dimension(), dim);
  assert_eq!(values.size(), ndoc);
  let mut stored = reader.stored_fields()?;
  let raw = std::fs::read(&args[2])?;
  assert_eq!(raw.len(), ndoc * dim * 4);
  let mut seen = vec![false; ndoc];
  let mut permutation_count = 0usize;
  let mut mapping = BufWriter::new(File::create(format!("{}.map.bin", args[3]))?);
  for ord in 0..values.size() {
    assert_eq!(values.ord_to_doc(ord)?, ord);
    let doc = stored.document(ord as i32)?;
    let id = doc
      .get_field("id")
      .ok_or("missing id")?
      .numeric_value()?
      .ok_or("missing number")?
      .to_i32()
      .ok_or("id type")?;
    assert!(id >= 0 && (id as usize) < ndoc);
    let id = id as usize;
    assert!(!seen[id]);
    seen[id] = true;
    permutation_count += usize::from(id != ord);
    mapping.write_all(&(id as i32).to_be_bytes())?;
    let bytes = &raw[id * dim * 4..(id + 1) * dim * 4];
    let actual = values.vector_value(ord)?;
    for j in 0..dim {
      assert_eq!(
        actual[j].to_bits(),
        u32::from_le_bytes(bytes[j * 4..j * 4 + 4].try_into()?)
      );
    }
  }
  assert!(seen.into_iter().all(|v| v));
  mapping.flush()?;
  println!("MAPPING\t{permutation_count}");
  let vector_reader = leaf.get_vector_reader()?.ok_or("missing vector reader")?;
  let mut graph = vector_reader.get_graph("knn")?;
  let mut out = BufWriter::new(File::create(&args[3])?);
  for value in [
    graph.size(),
    graph.num_levels()?,
    graph.entry_node()?.ok_or("missing entry")?,
  ] {
    out.write_all(&(value as i32).to_be_bytes())?;
  }
  let mut nodes = 0u64;
  let mut edges = 0u64;
  for level in 0..graph.num_levels()? {
    out.write_all(&(level as i32).to_be_bytes())?;
    let level_nodes: Vec<_> = graph.get_nodes_on_level(level)?.collect();
    for node in level_nodes {
      out.write_all(&(node as i32).to_be_bytes())?;
      nodes += 1;
      graph.seek(level, node)?;
      loop {
        let n = graph.next_neighbor()?;
        if n == i32::MAX as usize {
          break;
        }
        out.write_all(&(n as i32).to_be_bytes())?;
        edges += 1;
      }
      out.write_all(&(-1i32).to_be_bytes())?;
    }
    out.write_all(&(-2i32).to_be_bytes())?;
  }
  out.flush()?;
  leaf.check_integrity()?;
  println!(
    "VERIFIED\t{ndoc}\t{dim}\t{}\t{}\t{nodes}\t{edges}",
    graph.num_levels()?,
    graph.entry_node()?.ok_or("missing entry")?
  );
  reader.close()?;
  dir.close()?;
  Ok(())
}

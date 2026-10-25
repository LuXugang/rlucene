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
  analysis::standard::standard_analyzer::StandardAnalyzer,
  codecs::{
    codec::Codec, lucene99::lucene99_hnsw_vectors_format::Lucene99HnswVectorsFormat,
    lucene101_codec::Lucene101Codec,
  },
  document::{
    document::Document, knn_float_vector_field::KnnFloatVectorField, stored_field::StoredField,
  },
  index::{
    directory_reader,
    index_reader::IndexReader,
    index_reader_context::IndexReaderContext,
    index_writer::IndexWriter,
    index_writer_config::{IndexWriterConfig, OpenMode},
    live_index_writer_config::LiveIndexWriterConfig,
    merge_scheduler::MergeSchedulerEnum,
    two_phase_commit::TwoPhaseCommit,
    vector_similarity_function::VectorSimilarityFunction,
  },
  store::mmap_directory::MMapDirectory,
  util::close::CloseableRef,
};
use std::{fs::File, io::Read, path::Path, sync::Arc, time::Instant};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn configured_codec() -> Result<Lucene101Codec> {
  Ok(
    Lucene101Codec::new()?.with_knn_vectors_format(
      Lucene99HnswVectorsFormat::with_graph_para(
        std::env::var("RLUCENE_BENCH_M")?.parse()?,
        std::env::var("RLUCENE_BENCH_BEAM")?.parse()?,
      )?
      .into(),
    ),
  )
}
fn main() -> Result<()> {
  let a: Vec<_> = std::env::args().collect();
  let dim: usize = std::env::var("RLUCENE_BENCH_DIM")?.parse()?;
  let ndoc: i32 = std::env::var("RLUCENE_BENCH_NDOC")?.parse()?;
  let similarity = match std::env::var("RLUCENE_BENCH_SIMILARITY")?.as_str() {
    "EUCLIDEAN" => VectorSimilarityFunction::Euclidean,
    "DOT_PRODUCT" => VectorSimilarityFunction::DotProduct,
    _ => return Err("unsupported similarity".into()),
  };
  let total_start = Instant::now();
  let mut cfg = IndexWriterConfig::with_analyzer(StandardAnalyzer::new())?;
  let codec = configured_codec()?;
  let actual_format = codec
    .knn_vectors_format()?
    .get_knn_vectors_format_for_field("knn")?
    .to_string();
  cfg.set_open_mode(OpenMode::Create).set_codec(codec);
  cfg
    .set_use_compound_file(false)
    .set_ram_buffer_size_mb(std::env::var("RLUCENE_BENCH_RAM_MIB")?.parse()?);
  cfg.set_max_full_flush_merge_wait_millis(0);
  println!(
    "CONFIG\tcodec={}\tRAM_MiB={}\tcompound={}\tvector_format={}\tseed={}",
    cfg.get_codec(),
    cfg.get_ram_buffer_size_mb(),
    cfg.get_use_compound_file(),
    actual_format,
    rlucene::core::util::hnsw::hnsw_graph_builder::RAND_SEED
  );
  let index_start = Instant::now();
  let dir = Arc::new(MMapDirectory::new(Path::new(&a[1]))?);
  let mut source = File::open(&a[2])?;
  let mut bytes = vec![0u8; dim * 4];
  let mut values = vec![0f32; dim];
  let writer = IndexWriter::new(dir.clone(), cfg)?;
  let start = Instant::now();
  for i in 0..ndoc {
    source.read_exact(&mut bytes)?;
    for (j, v) in values.iter_mut().enumerate() {
      *v = f32::from_le_bytes(bytes[j * 4..j * 4 + 4].try_into()?);
    }
    let mut doc = Document::new();
    doc.add(KnnFloatVectorField::with_similarity_function(
      "knn",
      values.clone(),
      similarity,
    )?);
    doc.add(StoredField::from_i32("id", i)?);
    writer.add_document(doc)?;
    if (i + 1) % 100000 == 0 {
      println!("INDEXED\t{}", i + 1);
    }
  }
  let loop_ns = start.elapsed().as_nanos();
  println!(
    "FIELD_CONFIG\tdimension={dim}\tsimilarity={}\tdocs={ndoc}",
    std::env::var("RLUCENE_BENCH_SIMILARITY")?
  );
  let commit_start = Instant::now();
  writer.commit()?;
  let commit = commit_start.elapsed().as_nanos();
  let indexed = loop_ns + commit;
  let upstream_index = index_start.elapsed().as_nanos();
  let start = Instant::now();
  match writer.get_config()?.get_merge_scheduler() {
    MergeSchedulerEnum::Concurrent(c) => c.sync()?,
    _ => return Err("default scheduler not concurrent".into()),
  };
  let wait = start.elapsed().as_nanos();
  let start = Instant::now();
  writer.close()?;
  let close = start.elapsed().as_nanos();
  drop(writer);
  let mut append = IndexWriterConfig::with_analyzer(StandardAnalyzer::new())?;
  append
    .set_open_mode(OpenMode::Append)
    .set_codec(configured_codec()?);
  append
    .set_use_compound_file(false)
    .set_ram_buffer_size_mb(std::env::var("RLUCENE_BENCH_RAM_MIB")?.parse()?);
  append.set_max_full_flush_merge_wait_millis(0);
  let start = Instant::now();
  let merged = IndexWriter::new(dir.clone(), append)?;
  merged.force_merge_with_wait(1, false)?;
  match merged.get_config()?.get_merge_scheduler() {
    MergeSchedulerEnum::Concurrent(c) => c.sync()?,
    _ => return Err("scheduler".into()),
  };
  merged.close()?;
  let force = start.elapsed().as_nanos();
  dir.close()?;
  let total = total_start.elapsed().as_nanos();
  let d = Arc::new(MMapDirectory::new(Path::new(&a[1]))?);
  let r = Arc::new(directory_reader::open(d.clone())?);
  let segments = r.clone().get_context()?.leaves()?.len();
  let docs = r.num_docs()?;
  println!(
    "RESULT\t{loop_ns}\t{indexed}\t{wait}\t{force}\t{close}\t{total}\t{segments}\t{docs}\t{commit}\t{upstream_index}"
  );
  r.close()?;
  d.close()?;
  Ok(())
}

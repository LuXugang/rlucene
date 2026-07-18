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
#[allow(dead_code)] // for quick search
struct TestManyKnnDocs;

#[cfg(feature = "monster")]
mod monster {
  use crate::core::document::document::Document;
  use crate::core::document::knn_float_vector_field::KnnFloatVectorField;
  use crate::core::index::directory_reader;
  use crate::core::index::index_writer::IndexWriter;
  use crate::core::index::index_writer_config::IndexWriterConfig;
  use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
  use crate::core::index::tiered_merge_policy::TieredMergePolicy;
  use crate::core::index::two_phase_commit::TwoPhaseCommit;
  use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
  use crate::core::search::knn_float_vector_query::KnnFloatVectorQuery;
  use crate::core::search::top_docs::TopDocsLike;
  use crate::core::util::error::lucene_error::Result;
  use crate::test_framework::core::util::lucene_test_case::{
    create_temp_dir_with_prefix, new_fs_directory, new_searcher_with_reader, random,
  };

  #[test]
  #[ignore = "monster"]
  fn test_large_segment() -> Result<()> {
    let mut random = random();
    let mut iwc = IndexWriterConfig::new()?;
    // TODO: setCodec 未实现
    // ConfigurableMCodec(128) to make sure to use the ConfigurableMCodec instead
    // of a random one.
    iwc.set_ram_buffer_size_mb(64.0); // Use a 64MB buffer to create larger initial segments.
    let mut mp = TieredMergePolicy::new();
    mp.set_max_merge_at_once(256)?; // Avoid intermediate merges (waste of time with HNSW?).
    mp.set_segments_per_tier(256.0)?; // Only merge once at the end when we ask.
    iwc.set_merge_policy(mp);
    let field_name = "field";
    let similarity_function = VectorSimilarityFunction::DotProduct;

    let temp_dir = create_temp_dir_with_prefix("ManyKnnVectorDocs")?;
    let dir = new_fs_directory(&mut random, temp_dir)?;
    let iw = IndexWriter::new(dir.clone(), iwc)?;

    let num_vectors = 2_088_992;
    let mut vector = vec![0.0f32; 1];
    let mut field = KnnFloatVectorField::with_similarity_function(
      field_name,
      vector.clone(),
      similarity_function,
    )?;
    for i in 0..num_vectors {
      vector[0] = (i % 256) as f32;
      field.set_vector_value(vector.clone())?;
      let mut doc = Document::new();
      doc.add(field.clone());
      iw.add_document(doc)?;
    }

    // Merge to single segment and then verify.
    iw.force_merge(1)?;
    iw.commit()?;
    let searcher = new_searcher_with_reader(directory_reader::open(dir.clone())?)?;
    let docs = searcher.search(KnnFloatVectorQuery::new("field", vec![120.0], 10)?, 5)?;
    assert_eq!(5, docs.score_docs().len());

    iw.close()?;
    Ok(())
  }
}

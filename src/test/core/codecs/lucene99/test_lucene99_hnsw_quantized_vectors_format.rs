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
use crate::core::codecs::lucene99::lucene99_hnsw_scalar_quantized_vectors_format::Lucene99HnswScalarQuantizedVectorsFormat;
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_format::{
  MAXIMUM_BEAM_WIDTH, MAXIMUM_MAX_CONN,
};
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_reader::SIMILARITY_FUNCTIONS;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::error::lucene_error::{LuceneError, Result};

#[allow(dead_code)] // for quick search
struct TestLucene99HnswQuantizedVectorsFormat;

// Verifies it's fine to change your mind on the number of bits quantization you want for the same
// field in the same index by changing up the Codec. This is allowed because at merge time we
// requantize the vectors.
#[test]
fn test_mixed_quantized_bits() -> Result<()> {
  // TODO: IndexWriterConfig does not yet support injecting the per-test KnnVectorsFormat needed to
  // write one field first with 4-bit and then with 7-bit quantization.
  Ok(())
}

// Verifies you can change your mind and enable quantization on a previously indexed vector field
// without quantization.
#[test]
fn test_mixed_quantized_un_quantized() -> Result<()> {
  // TODO: IndexWriterConfig does not yet support switching the KnnVectorsFormat for an existing
  // index from unquantized HNSW vectors to scalar-quantized HNSW vectors.
  Ok(())
}

#[test]
fn test_quantization_scoring_edge_case() -> Result<()> {
  // TODO: The custom codec injection and nearest-vector search path required by this Java test have
  // not been migrated.
  Ok(())
}

#[test]
fn test_quantized_vectors_write_and_read() -> Result<()> {
  // TODO: The per-field reader inspection API used to obtain QuantizedByteVectorValues from a
  // Lucene99HnswVectorsReader has not been migrated.
  Ok(())
}

#[test]
fn test_to_string() -> Result<()> {
  let format = Lucene99HnswScalarQuantizedVectorsFormat::
    with_graph_para_with_threads_bits_compress_confidence_interval(
      10,
      20,
      1,
      4,
      false,
      Some(0.9),
    )?;
  let expected = "Lucene99HnswScalarQuantizedVectorsFormat(name=Lucene99HnswScalarQuantizedVectorsFormat, maxConn=10, beamWidth=20, flatVectorFormat=Lucene99ScalarQuantizedVectorsFormat(name=Lucene99ScalarQuantizedVectorsFormat, confidenceInterval=0.9, bits=4, compress=false, flatVectorScorer=ScalarQuantizedVectorScorer(nonQuantizedDelegate=DefaultFlatVectorScorer()), rawVectorFormat=Lucene99FlatVectorsFormat(vectorsScorer=DefaultFlatVectorScorer())))";
  assert_eq!(expected, format.to_string());
  Ok(())
}

#[test]
fn test_limits() -> Result<()> {
  // TODO: The Java -1 maxConn and beamWidth cases cannot be represented by Rust's usize
  // constructor parameters.
  assert!(matches!(
    Lucene99HnswScalarQuantizedVectorsFormat::with_graph_para(0, 20),
    Err(LuceneError::IllegalArgument(_))
  ));
  assert!(matches!(
    Lucene99HnswScalarQuantizedVectorsFormat::with_graph_para(20, 0),
    Err(LuceneError::IllegalArgument(_))
  ));
  assert!(matches!(
    Lucene99HnswScalarQuantizedVectorsFormat::with_graph_para(MAXIMUM_MAX_CONN + 1, 20),
    Err(LuceneError::IllegalArgument(_))
  ));
  assert!(matches!(
    Lucene99HnswScalarQuantizedVectorsFormat::with_graph_para(20, MAXIMUM_BEAM_WIDTH + 1),
    Err(LuceneError::IllegalArgument(_))
  ));
  assert!(matches!(
    Lucene99HnswScalarQuantizedVectorsFormat::
      with_graph_para_with_threads_bits_compress_confidence_interval(
        20,
        100,
        0,
        7,
        false,
        Some(1.1),
      ),
    Err(LuceneError::IllegalArgument(_))
  ));
  assert!(matches!(
    Lucene99HnswScalarQuantizedVectorsFormat::
      with_graph_para_with_threads_bits_compress_confidence_interval(
        20, 100, 0, -1, false, None,
      ),
    Err(LuceneError::IllegalArgument(_))
  ));
  assert!(matches!(
    Lucene99HnswScalarQuantizedVectorsFormat::
      with_graph_para_with_threads_bits_compress_confidence_interval(
        20, 100, 0, 5, false, None,
      ),
    Err(LuceneError::IllegalArgument(_))
  ));

  assert!(matches!(
    Lucene99HnswScalarQuantizedVectorsFormat::
      with_graph_para_with_threads_bits_compress_confidence_interval(
        20, 100, 0, 9, false, None,
      ),
    Err(LuceneError::IllegalArgument(_))
  ));
  assert!(matches!(
    Lucene99HnswScalarQuantizedVectorsFormat::
      with_graph_para_with_threads_bits_compress_confidence_interval(
        20,
        100,
        0,
        7,
        false,
        Some(0.8),
      ),
    Err(LuceneError::IllegalArgument(_))
  ));
  // TODO: The Rust constructor has no executor argument corresponding to Java's
  // SameThreadExecutorService rejection case.
  Ok(())
}

// Ensures that all expected vector similarity functions are translatable in the format.
#[test]
fn test_vector_similarity_funcs() {
  // This does not necessarily have to be all similarity functions, but differences should be
  // considered carefully.
  let expected_values = [
    VectorSimilarityFunction::Euclidean,
    VectorSimilarityFunction::DotProduct,
    VectorSimilarityFunction::Cosine,
    VectorSimilarityFunction::MaximumInnerProduct,
  ];
  assert_eq!(SIMILARITY_FUNCTIONS, expected_values);
}

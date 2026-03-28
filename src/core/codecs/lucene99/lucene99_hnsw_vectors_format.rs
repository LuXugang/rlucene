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
use crate::core::codecs::hnsw::default_flat_vector_scorer::DefaultFlatVectorScorer;
use crate::core::codecs::hnsw::flat_vector_scorer_util::LUCENE99_FLAT_VECTORS_SCORER;
use crate::core::codecs::lucene99::lucene99_flat_vectors_format::Lucene99FlatVectorsFormat;
use crate::core::util::hnsw::hnsw_graph_builder::DEFAULT_MAX_CONN as OtherDEFAULT_MAX_CONN;
use once_cell::sync::Lazy;
pub(crate) const META_CODEC_NAME: &str = "Lucene99HnswVectorsFormatMeta";
pub(crate) const VECTOR_INDEX_CODEC_NAME: &str = "Lucene99HnswVectorsFormatIndex";
pub(crate) const META_EXTENSION: &str = "vem";
pub(crate) const VECTOR_INDEX_EXTENSION: &str = "vex";

pub const VERSION_START: i32 = 0;
pub const VERSION_CURRENT: i32 = VERSION_START;

/// A maximum configurable maximum max conn.
///
/// NOTE: We eagerly populate `float[MAX_CONN*2]` and `int[MAX_CONN*2]`, so exceptionally large
/// numbers here will use an inordinate amount of heap
pub const MAXIMUM_MAX_CONN: i32 = 512;

/// Default number of maximum connections per node
pub const DEFAULT_MAX_CONN: usize = OtherDEFAULT_MAX_CONN;

/// The maximum size of the queue to maintain while searching during graph construction. This
/// maximum value preserves the ratio of the `DEFAULT_BEAM_WIDTH`/`DEFAULT_MAX_CONN` (i.e. `6.25 * 16 = 3200`).
pub const MAXIMUM_BEAM_WIDTH: i32 = 3200;

/// Default number of the size of the queue maintained while searching during a graph construction.
pub const DEFAULT_BEAM_WIDTH: usize = DEFAULT_MAX_CONN;

/// Default to use single thread merge
pub const DEFAULT_NUM_MERGE_WORKER: i32 = 1;

pub(crate) const DIRECT_MONOTONIC_BLOCK_SHIFT: i32 = 16;
pub struct Lucene99HnswVectorsFormat {
  max_conn: usize,
  beam_width: usize,
  num_merge_workers: usize,
}

impl Lucene99HnswVectorsFormat {}
pub static FLAT_VECTORS_FORMAT: Lazy<Lucene99FlatVectorsFormat<DefaultFlatVectorScorer, u8>> =
  Lazy::new(|| {
    let scorer = LUCENE99_FLAT_VECTORS_SCORER.clone();
    Lucene99FlatVectorsFormat::new(scorer)
  });

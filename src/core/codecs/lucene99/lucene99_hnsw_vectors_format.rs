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
use crate::core::util::hnsw::hnsw_graph_builder::DEFAULT_MAX_CONN;

pub struct Lucene99HnswVectorsFormat;

impl Lucene99HnswVectorsFormat {
  pub const META_CODEC_NAME: &'static str = "Lucene99HnswVectorsFormatMeta";
  pub const VECTOR_INDEX_CODEC_NAME: &'static str = "Lucene99HnswVectorsFormatIndex";
  pub const META_EXTENSION: &'static str = "vem";
  pub const VECTOR_INDEX_EXTENSION: &'static str = "vex";

  pub const VERSION_START: i32 = 0;
  pub const VERSION_CURRENT: i32 = Self::VERSION_START;

  /// A maximum configurable maximum max conn.
  ///
  /// NOTE: We eagerly populate `float[MAX_CONN*2]` and `int[MAX_CONN*2]`, so exceptionally large
  /// numbers here will use an inordinate amount of heap
  pub const MAXIMUM_MAX_CONN: i32 = 512;

  /// Default number of maximum connections per node
  pub const DEFAULT_MAX_CONN: usize = DEFAULT_MAX_CONN;

  /// The maximum size of the queue to maintain while searching during graph construction. This
  /// maximum value preserves the ratio of the `DEFAULT_BEAM_WIDTH`/`DEFAULT_MAX_CONN` (i.e. `6.25 * 16 = 3200`).
  pub const MAXIMUM_BEAM_WIDTH: i32 = 3200;

  /// Default number of the size of the queue maintained while searching during a graph construction.
  pub const DEFAULT_BEAM_WIDTH: usize = DEFAULT_MAX_CONN;

  /// Default to use single thread merge
  pub const DEFAULT_NUM_MERGE_WORKER: i32 = 1;

  pub const DIRECT_MONOTONIC_BLOCK_SHIFT: i32 = 16;
}

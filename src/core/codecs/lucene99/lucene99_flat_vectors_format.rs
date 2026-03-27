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
use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
pub(crate) const NAME: &str = "Lucene99FlatVectorsFormat";
pub(crate) const META_CODEC_NAME: &str = "Lucene99FlatVectorsFormatMeta";
pub(crate) const VECTOR_DATA_CODEC_NAME: &str = "Lucene99FlatVectorsFormatData";
pub(crate) const META_EXTENSION: &str = "vemf";
pub(crate) const VECTOR_DATA_EXTENSION: &str = "vec";

pub(crate) const VERSION_START: i32 = 0;
pub(crate) const VERSION_CURRENT: i32 = VERSION_START;

pub(crate) const DIRECT_MONOTONIC_BLOCK_SHIFT: i32 = 16;
pub struct Lucene99FlatVectorsFormat<T>
where
  T: FlatVectorsScorer,
{
  vectors_scorer: T,
}
impl<T: FlatVectorsScorer> Lucene99FlatVectorsFormat<T> {
  pub fn new(vectors_scorer: T) -> Self {
    Self { vectors_scorer }
  }
}

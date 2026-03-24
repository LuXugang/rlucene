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
use crate::core::codecs::knn_field_vectors_writer::KnnFieldVectorsWriter;
use crate::core::index::docs_with_field_set::DocsWithFieldSet;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

/// Vectors' writer for a field
pub trait FlatFieldVectorsWriter: KnnFieldVectorsWriter {
  /// Returns a list of vectors to be written.
  fn get_vectors(&self) -> Arc<Vec<Self::V>>;

  /// Returns the DocsWithFieldSet for the field writer.
  fn get_docs_with_field_set(&self) -> &DocsWithFieldSet;

  /// Indicates that this writer is done and no new vectors are allowed to be added.
  fn finish(&mut self) -> Result<()>;

  /// Returns true if the writer is done and no new vectors are allowed to be added.
  fn is_finished(&self) -> bool;
}

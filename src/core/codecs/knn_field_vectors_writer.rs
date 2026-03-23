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
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::Result;
/// Vectors’ writer for a field.
///
/// # Parameters
///
/// - `T`: an array type; the type of vectors to be written.
pub trait KnnFieldVectorsWriter: Accountable {
  type V;
  /// Adds a new doc ID with its vector value to the given field for indexing.
  /// Doc IDs must be added in increasing order.
  fn add_value(&mut self, doc_id: i32, vector_value: Self::V) -> Result<()>;
  /// Used to copy values being indexed to internal storage.
  ///
  /// # Arguments
  ///
  /// - `vector_value`: an array containing the vector value to add.
  ///
  /// # Returns
  ///
  /// A copy of the value; a new array.
  fn copy_value(&self, vector_value: Self::V) -> Result<Self::V>;
}

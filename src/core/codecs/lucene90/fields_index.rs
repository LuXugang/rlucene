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
use crate::core::util::clone::TryClone;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
pub(crate) trait FieldsIndex: TryClone + CloseableRef {
  /// Get the ID of the block that contains the given docID.
  fn get_block_id(&mut self, doc_id: i32) -> Result<i64>;

  /// Get the start pointer of the block with the given ID.
  fn get_block_start_pointer(&mut self, block_id: i64) -> Result<usize>;

  /// Get the number of bytes of the block with the given ID.
  fn get_block_length(&mut self, block_id: i64) -> Result<usize>;

  /// Get the start pointer of the block that contains the given docID.
  /// This is a final method in the original struct, so it's implemented
  /// directly here.
  fn get_start_pointer(&mut self, doc_id: i32) -> Result<usize> {
    let block_id = self.get_block_id(doc_id)?;
    self.get_block_start_pointer(block_id)
  }

  /// Check the integrity of the index.
  fn check_integrity(&self) -> Result<()>;
}

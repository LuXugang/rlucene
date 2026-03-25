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
use strum_macros::{Display, EnumCount, FromRepr};

/// The numeric datatype of the vector values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr, EnumCount, Display)]
#[repr(u8)]
pub enum VectorEncoding {
  /**
   * Encodes vector using 8 bits of precision per sample. Values provided
   * with higher precision (e.g., queries provided as float) *must*
   * be in the range [-128, 127]. NOTE: this can enable significant
   * storage savings and faster searches, at the cost of some possible
   * loss of precision.
   */
  BYTE(usize),

  /// Encodes vector using 32 bits of precision per sample in IEEE floating
  /// point format.
  FLOAT32(usize),
}

impl VectorEncoding {
  /// The number of bytes required to encode a scalar in this format.
  /// A vector will nominally require dimension * byteSize bytes of storage.
  pub fn byte_size(&self) -> usize {
    match self {
      VectorEncoding::BYTE(size) => *size,
      VectorEncoding::FLOAT32(size) => *size,
    }
  }
  pub fn ordinal(&self) -> i32 {
    match self {
      VectorEncoding::BYTE(_) => 0,
      VectorEncoding::FLOAT32(_) => 1,
    }
  }
  pub const fn values() -> &'static [VectorEncoding] {
    &[VectorEncoding::BYTE(1), VectorEncoding::FLOAT32(4)]
  }
}

impl Default for VectorEncoding {
  fn default() -> Self {
    VectorEncoding::BYTE(1)
  }
}

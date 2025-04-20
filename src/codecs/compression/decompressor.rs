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
use crate::index::BytesRef;
use crate::store::DataInput;
use crate::util::error::lucene_error::Result;

/// A decompressor.
pub trait Decompressor: crate::util::clone::TryClone {
    /// Decompress bytes that were stored between offsets `offset` and `offset + length`
    /// in the original stream from the compressed stream `in` to `bytes`.
    /// After returning, the length of `bytes` must be equal to `length`. Implementations of this
    /// method are free to resize `bytes` depending on their needs.
    ///
    /// # Parameters
    /// - `in`: The input that stores the compressed stream.
    /// - `original_length`: The length of the original data (before compression).
    /// - `offset`: Bytes before this offset do not need to be decompressed.
    /// - `length`: Bytes after `offset + length` do not need to be decompressed.
    /// - `bytes`: A reference to a `BytesRef` where to store the decompressed data.
    fn decompress(
        &mut self,
        input: &mut impl DataInput,
        original_length: i32,
        offset: i32,
        length: i32,
        bytes: &mut BytesRef<Vec<u8>>,
    ) -> Result<()>;
}

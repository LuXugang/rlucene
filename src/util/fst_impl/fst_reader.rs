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
use crate::store::DataOutput;
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::fst::BytesReader;

/// Abstraction for reading bytes necessary for FST.
pub trait FstReader: Accountable {
    type FstBytesReader: BytesReader;
    /// Get the reverse `BytesReader` for this FST.
    ///
    /// # Returns
    /// The reverse `BytesReader`.
    fn get_reverse_bytes_reader(&self) -> Result<Self::FstBytesReader>;

    /// Write this FST to another `DataOutput`.
    ///
    /// # Parameters
    /// - `out`: The `DataOutput` to write to.
    ///
    /// # Errors
    /// Returns an error if an exception occurred during writing.
    fn write_to(&self, out: &mut impl DataOutput) -> Result<()>;
}

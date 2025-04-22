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
use crate::util::access::AccessVec;
use crate::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;

pub trait BytesRefIterator<AV>
where
    AV: AccessVec<u8>,
{
    /// The returned `BytesRef` may be re-used across calls to `next`. After this method returns `None`,
    /// do not call it again as the results are undefined.
    ///
    /// # Returns
    /// The next [`BytesRef`] in the iterator or `None` if the end of the iterator is reached.
    ///
    /// # Note
    /// In some scenarios, we need to return a reference to the `BytesRef` to avoid frequent copying operations.
    /// Like in [`TermsDict`](crate::codecs::lucene90::lucene90_doc_values_producer::TermsDict), this method can be used
    /// when reusing internal buffers to reduce allocations and improve performance.
    ///
    /// To simplify the interface and allow for both owned and borrowed variants in a unified way,
    /// it is recommended to use [`Cow<BytesRef>`](std::borrow::Cow). This enables returning either:
    ///
    /// - `Cow::Borrowed(&BytesRef)` when the data is internally reusable, avoiding clone costs
    /// - `Cow::Owned(BytesRef)` when a fresh copy is required
    ///
    /// This approach provides flexibility to the implementor and clarity to the caller,
    /// while preserving performance by avoiding unnecessary allocations.
    /// # Errors
    /// Returns an `std::io::Error` if there is a low-level I/O error.
    fn next(&mut self) -> Result<Option<Cow<BytesRef<AV>>>> {
        Err(LuceneError::need_implemented("this method need implement"))
    }
}

pub struct EmptyBytesRefIterator;

impl<AV> BytesRefIterator<AV> for EmptyBytesRefIterator
where
    AV: AccessVec<u8>,
{
    fn next(&mut self) -> Result<Option<Cow<BytesRef<AV>>>> {
        Ok(None)
    }
}

impl EmptyBytesRefIterator {
    #[allow(unused)]
    pub const EMPTY: Self = EmptyBytesRefIterator;
}

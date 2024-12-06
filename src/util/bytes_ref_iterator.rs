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
use crate::util::error::runtime_error::RuntimeError;

pub trait BytesRefIterator {
    /// Increments the iteration to the next [`BytesRef`](BytesRef) in the iterator.
    /// Returns the resulting [`BytesRef`](BytesRef) or `None` if the end of the iterator is reached.
    /// The returned `BytesRef` may be re-used across calls to `next`. After this method returns `None`,
    /// do not call it again as the results are undefined.
    ///
    /// # Returns
    /// The next [`BytesRef`](BytesRef) in the iterator or `None` if the end of the iterator is reached.
    ///
    /// # Errors
    /// Returns an `std::io::Error` if there is a low-level I/O error.
    #[allow(dead_code)]
    fn next(&self) -> Result<Option<BytesRef>, RuntimeError>;
}

pub struct EmptyBytesRefIterator;

impl BytesRefIterator for EmptyBytesRefIterator {
    fn next(&self) -> Result<Option<BytesRef>, RuntimeError> {
        Ok(None)
    }
}

impl EmptyBytesRefIterator {
    #[allow(dead_code)]
    pub const EMPTY: Self = EmptyBytesRefIterator;
}

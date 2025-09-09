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
use strum_macros::{EnumCount, FromRepr};

/// Describes how an
/// [`IndexableField`](crate::core::index::indexable_field::IndexableField) should be
/// inverted for indexing terms and postings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, FromRepr, Hash, EnumCount)]
#[repr(u8)]
pub enum InvertableType {
    /// The field should be treated as a single value whose binary content is
    /// returned by
    /// [`IndexableField::binary_value()`](crate::core::index::indexable_field::IndexableField::binary_value).
    /// The term frequency is assumed to be one. If you need to index
    /// multiple values, you should pass multiple
    /// [`IndexableField`](crate::core::index::indexable_field::IndexableField)
    /// instances to the
    /// [`IndexWriter`](crate::core::index::index_writer::IndexWriter). If the same
    /// value is provided multiple times, the term frequency will be equal
    /// to the number of times that this value occurred in the same document.
    BINARY,

    /// The field should be inverted through its
    /// [`IndexableField::token_stream()`](crate::core::index::indexable_field::IndexableField::token_stream).
    TokenStream,
}

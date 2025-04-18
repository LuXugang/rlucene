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

/// DocValues types. Note that DocValues is strongly typed, so a field cannot have different types
/// across different documents.
#[derive(Debug, PartialEq, Eq, Clone, Copy, FromRepr, Hash, EnumCount)]
#[repr(u8)]
pub enum DocValuesType {
    /// No doc values for this field.
    None,
    /// A per-document Number.
    Numeric,
    /// A per-document byte[].
    /// Values may be larger than 32,766 bytes, but different codecs may enforce
    /// their own limits.
    Binary,
    /// A pre-sorted byte[]. Fields with this type only store distinct byte values and store an
    /// additional offset pointer per document to dereference the shared byte[]. The stored byte[] is
    /// presorted and allows access via document id, ordinal, and by-value. Values must be <= 32,766 bytes.
    Sorted,
    /// A pre-sorted Number[]. Fields with this type store numeric values in sorted order according to
    /// `i64::cmp`.
    SortedNumeric,
    /// A pre-sorted Set of byte[]. Fields with this type only store distinct byte values and store
    /// additional offset pointers per document to dereference the shared byte[]. The stored byte[] is
    /// presorted and allows access via document id, ordinal, and by-value. Values must be <= 32,766 bytes.
    SortedSet,
}
/// Use Default for padding
impl Default for DocValuesType {
    fn default() -> Self {
        DocValuesType::None
    }
}

/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/
use strum_macros::{Display, EnumCount, FromRepr};

/// DocValues types. Note that DocValues is strongly typed, so a field cannot
/// have different types across different documents.
#[derive(Debug, PartialEq, Eq, Clone, Copy, FromRepr, Hash, EnumCount, Display)]
#[repr(u8)]
pub enum DocValuesType {
    /// No doc values for this field.
    None,
    /// A per-document Number.
    Numeric,
    /// A per-document byte[].
    /// Values may be larger than 32,766 bytes, but different codecs may
    /// enforce their own limits.
    Binary,
    /// A pre-sorted byte[]. Fields with this type only store distinct byte
    /// values and store an additional offset pointer per document to
    /// dereference the shared byte[]. The stored byte[] is presorted and
    /// allows access via document id, ordinal, and by-value. Values must be <=
    /// 32,766 bytes.
    Sorted,
    /// A pre-sorted Number[]. Fields with this type store numeric values in
    /// sorted order according to `i64::cmp`.
    SortedNumeric,
    /// A pre-sorted Set of byte[]. Fields with this type only store distinct
    /// byte values and store additional offset pointers per document to
    /// dereference the shared byte[]. The stored byte[] is presorted and
    /// allows access via document id, ordinal, and by-value. Values must be <=
    /// 32,766 bytes.
    SortedSet,
}
/// Use Default for padding
impl Default for DocValuesType {
    fn default() -> Self {
        DocValuesType::None
    }
}

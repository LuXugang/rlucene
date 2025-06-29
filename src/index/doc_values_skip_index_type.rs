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
use strum_macros::{Display, FromRepr};

use crate::index::doc_values_type::DocValuesType;

/// Options for skip indexes on doc values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr, Hash, Display)]
#[repr(u8)]
pub enum DocValuesSkipIndexType {
    /// No skip index should be created.
    None,
    /// Record range of values. Suitable for:
    /// - `Numeric`
    /// - `SortedNumeric`
    /// - `Sorted`
    /// - `SortedSet`
    ///
    /// Records min/max values per range of doc IDs.
    Range,
}

impl DocValuesSkipIndexType {
    /// Checks compatibility with a specific doc values type
    pub fn is_compatible_with(&self, dv_type: DocValuesType) -> bool {
        match self {
            Self::None => true,
            Self::Range => matches!(
                dv_type,
                DocValuesType::Numeric
                    | DocValuesType::SortedNumeric
                    | DocValuesType::Sorted
                    | DocValuesType::SortedSet
            ),
        }
    }
}
/// Use Default for padding
impl Default for DocValuesSkipIndexType {
    fn default() -> Self {
        Self::None
    }
}

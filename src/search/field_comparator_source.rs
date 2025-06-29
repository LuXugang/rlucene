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
use std::fmt::{Display, Formatter};
use std::hash::Hash;

use crate::search::field_comparator::FieldComparator;
use crate::search::pruning::Pruning;
use crate::util::error::lucene_error::Result;

/// Provides a [`FieldComparator`]
/// for custom field sorting.
///
/// # Lucene Experimental
/// This API is experimental and may change in future versions.
pub trait FieldComparatorSource: Display + Clone {
    /// Creates a comparator for the field in the given index.
    ///
    /// # Arguments
    /// - `field_name`: The name of the field to create a comparator for.
    /// - `num_hits`: The number of hits.
    /// - `pruning`: The pruning strategy to use.
    /// - `reversed`: Whether the sorting should be reversed.
    ///
    /// # Returns
    /// A new [`FieldComparator`] instance.
    ///
    /// # Errors
    /// Returns an error if the comparator could not be created due to I/O
    /// issues or invalid parameters.
    fn new_comparator<F: FieldComparator>(
        &self,
        field_name: &str,
        num_hits: usize,
        pruning: Pruning,
        reversed: bool,
    ) -> Result<F>;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum FieldComparatorSourceEnum {}
impl Display for FieldComparatorSourceEnum {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

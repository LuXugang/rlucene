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
use std::fmt;
use std::fmt::Display;

use crate::search::sort_field::{SortField, SortFiledBase};
use crate::search::sort_field_enum::SortFieldEnum;
use crate::util::error::lucene_error::{LuceneError, Result};

#[derive(Clone)]
pub struct Sort {
    pub(crate) fields: Vec<SortFieldEnum>,
}

impl Sort {
    /// Replace Java's `Sort.INDEXORDER` with this method.
    pub fn get_index_order() -> Result<Self> {
        let sort_field = SortFieldEnum::Sorter(SortField::get_field_doc()?);
        Self::with_fields(vec![sort_field])
    }
    /// Replace Java's `Sort.RELEVANCE` with this method.
    pub fn get_relevance() -> Result<Self> {
        Self::new()
    }
    /// Returns true if the relevance score is needed to sort documents.
    pub fn needs_scores(&self) -> bool {
        for sort_field in &self.fields {
            if sort_field.needs_scores() {
                return true;
            }
        }
        false
    }
}

impl Sort {
    /// Sorts by computed relevance.
    ///
    /// This is the same sort criteria as calling `IndexSearcher::search`
    /// without a sort criteria, only with slightly more overhead.
    pub fn new() -> Result<Self> {
        let sort_field = SortFieldEnum::Sorter(SortField::get_field_score()?);
        Self::with_fields(vec![sort_field])
    }

    /// Sets the sort to the given criteria in succession.
    ///
    /// The first `SortField` is checked first, but if it produces a tie, then
    /// the second `SortField` is used to break the tie, and so on. Finally,
    /// if there is still a tie after all `SortField`s are checked, the
    /// internal Lucene doc ID is used to break it.
    ///
    /// # Arguments
    /// - `fields`: A vector of `SortField` to define the sorting order.
    ///
    /// # Errors
    /// Returns an error if the provided `fields` vector is empty.
    /// # Note
    /// You could use
    /// [`push_sort_fields`](crate::search::sort_field_enum::SortFieldVecExt::push_sort_fields)
    /// to init SortFieldEnum vector. # Example
    /// ```rust
    /// use rlucene::index::sort::Sort;
    /// use rlucene::search::sort_field::{SortField, SortFieldType};
    /// use rlucene::search::sort_field_enum::SortFieldVecExt;
    /// use rlucene::search::sorted_numeric_sort_field::SortedNumericSortField;
    /// use rlucene::search::sorted_set_sort_field::SortedSetSortField;
    /// let sort_field1 = SortField::new(Some("field1".to_string()), SortFieldType::Custom).unwrap();
    /// let sort_field2 = SortedSetSortField::new("field2".to_string(), false).unwrap();
    /// let mut fileds = Vec::new();
    /// fileds.push_sort_fields(sort_field1);
    /// fileds.push_sort_fields(sort_field2);
    /// let sort = Sort::with_fields(fileds);
    /// assert!(sort.is_ok());
    /// ```
    pub fn with_fields(fields: Vec<SortFieldEnum>) -> Result<Self> {
        if fields.is_empty() {
            Err(LuceneError::illegal_argument(
                "There must be at least 1 sort field".to_string(),
            ))
        } else {
            Ok(Self { fields })
        }
    }

    /// Representation of the sort criteria.
    ///
    /// # Returns
    /// Array (Vec) of `SortField` objects used in this sort criteria.
    pub fn get_sort(&self) -> &[SortFieldEnum] {
        &self.fields
    }
}

impl Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fields_string = self
            .fields
            .iter()
            .map(|field| field.to_string())
            .collect::<Vec<_>>()
            .join(",");
        write!(f, "{fields_string}")
    }
}
impl PartialEq for Sort {
    fn eq(&self, other: &Self) -> bool {
        self.fields == other.fields
    }
}
impl Eq for Sort {}

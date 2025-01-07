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
use crate::search::sort_field::{SortField, Type};
use crate::util::error::lucene_error::LuceneError;
use std::fmt::Display;

pub struct Sort {
    fields: Vec<SortField>,
}
impl Sort {
    /// Sorts by computed relevance.
    ///
    /// This is the same sort criteria as calling `IndexSearcher::search` without a sort criteria,
    /// only with slightly more overhead.
    pub fn new() -> Result<Self, LuceneError> {
        let sort_field = SortField::new(None, Type::Score)?;
        Self::new_with_fields(vec![sort_field])
    }

    /// Sets the sort to the given criteria in succession.
    ///
    /// The first `SortField` is checked first, but if it produces a tie, then the second `SortField`
    /// is used to break the tie, and so on. Finally, if there is still a tie after all `SortField`s
    /// are checked, the internal Lucene doc ID is used to break it.
    ///
    /// # Arguments
    /// - `fields`: A vector of `SortField` to define the sorting order.
    ///
    /// # Errors
    /// Returns an error if the provided `fields` vector is empty.
    pub fn new_with_fields(fields: Vec<SortField>) -> Result<Self, LuceneError> {
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
    pub fn get_sort(&self) -> &[SortField] {
        &self.fields
    }
}

impl Display for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sort")
    }
}

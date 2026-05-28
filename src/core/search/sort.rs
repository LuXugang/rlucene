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
use crate::core::search::sort_field::{SortField, SortFiledBase};
use crate::core::search::sort_field_enum::SortFieldEnum;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt;
use std::fmt::Display;
use std::hash::Hash;
/// Encapsulates sort criteria for returned hits.
///
/// A `Sort` can be created with an empty constructor, yielding an object
/// that instructs searches to return hits sorted by relevance; or it can be
/// created with one or more [`SortField`]s.
///
/// See also: [`SortField`].
#[derive(Clone)]
pub struct Sort {
  pub(crate) fields: Vec<SortFieldEnum>,
}

impl Sort {
  /// Represents sorting by index order.
  pub fn get_index_order() -> Result<Self> {
    let sort_field = SortFieldEnum::Sorter(SortField::get_field_doc()?);
    Self::with_fields(vec![sort_field])
  }
  /// Represents sorting by computed relevance. Using this sort criteria returns the same results as
  /// calling [`IndexSearcher::search(Query, i32)`](crate::core::search::index_searcher::IndexSearcher::search) without a sort criteria,
  /// only with slightly more overhead.
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
  pub fn with_fields<T>(fields: Vec<T>) -> Result<Self>
  where
    T: Into<SortFieldEnum>,
  {
    let fields: Vec<SortFieldEnum> = fields.into_iter().map(Into::into).collect();
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
  pub fn take_sort(&mut self) -> Vec<SortFieldEnum> {
    std::mem::take(&mut self.fields)
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

impl Hash for Sort {
  fn hash<H>(&self, state: &mut H)
  where
    H: std::hash::Hasher,
  {
    self.fields.hash(state);
  }
}

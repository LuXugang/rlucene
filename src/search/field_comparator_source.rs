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
use crate::search::field_comparator::FieldComparator;
use crate::search::pruning::Pruning;
use crate::util::error::lucene_error::LuceneError;

/// Provides a [`FieldComparator`]
/// for custom field sorting.
///
/// # Lucene Experimental
/// This API is experimental and may change in future versions.
pub trait FieldComparatorSource {
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
    /// Returns an error if the comparator could not be created due to I/O issues or invalid parameters.
    fn new_comparator<F: FieldComparator>(
        &self,
        field_name: &str,
        num_hits: usize,
        pruning: Pruning,
        reversed: bool,
    ) -> Result<F, LuceneError>;
}

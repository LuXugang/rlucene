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
use crate::core::search::pruning::Pruning;

pub(crate) const MIN_SKIP_INTERVAL: i32 = 32;
pub(crate) const MAX_SKIP_INTERVAL: i32 = 8192;
/// Base numeric comparator for comparing numeric values.
/// This comparator provides a skipping functionality – an iterator that can skip over
/// non-competitive documents.
///
/// The parameter `field` provided in the constructor is used as a field name in the default
/// implementations of the methods `get_numeric_doc_values` and `get_point_values` to retrieve
/// doc values and points.
///
/// You can pass a dummy value for a field name (e.g. when sorting by script),
/// but in this case you must override both of these methods.
pub struct NumericComparator<T> {
    pub(crate) field: String,
    pub(crate) missing_value: T,
    missing_value_as_long: i64,
    pub(crate) reverse: bool,
    bytes_count: i32, // how many bytes are used to encode this number

    pub(crate) top_value_set: bool,
    pub(crate) single_sort: bool, // true if sort is based on a single sort field
    pub(crate) hits_threshold_reached: bool,
    pub(crate) queue_full: bool,
    pub(crate) pruning: Pruning,
}

impl<T> NumericComparator<T> {
    pub fn new(
        field: String,
        missing_value: T,
        reverse: bool,
        pruning: Pruning,
        bytes_count: i32,
        missing_value_as_long: i64,
    ) -> Self {
        Self {
            field,
            missing_value,
            missing_value_as_long,
            reverse,
            bytes_count,
            top_value_set: false,
            single_sort: false,
            hits_threshold_reached: false,
            queue_full: false,
            pruning,
        }
    }

    pub fn set_top_value(&mut self) {
        self.top_value_set = true;
    }

    pub fn set_single_sort(&mut self) {
        self.single_sort = true;
    }

    pub fn disable_skipping(&mut self) {
        self.pruning = Pruning::None;
    }
}

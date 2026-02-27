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
use std::sync::Arc;

pub(crate) trait IntSet {
    /// Returns a slice (`&[i32]`) representation of this int set's values.
    /// Values are valid for indices `[0, size()]`.
    /// If this is a mutable int set, then changes to the set are not guaranteed
    /// to be visible in this slice.
    ///
    /// Returns:
    /// - A slice containing the values for this set, guaranteed to have at
    ///   least [`size()`](Self::size) elements.
    fn get_array(&mut self) -> &Arc<Vec<i32>>;

    /// Returns the number of values in this set.
    /// Guaranteed to be less than or equal to the length of the slice returned
    /// by [`get_array`](Self::get_array).
    ///
    /// Returns:
    /// - The number of values in this set.
    fn size(&self) -> usize;

    /// Computes a long (i64) hash code for this set.
    fn long_hash_code(&mut self) -> i64;
}

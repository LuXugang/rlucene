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
pub trait MSBRadixSorterBase {
    /// Returns the k-th byte of the entry at the given index `i`, or `-1` if its length is less than
    /// or equal to `k`.
    ///
    /// # Parameters
    /// - `i`: The index of the entry, which must be between `0` (inclusive) and `max_length` (exclusive).
    /// - `k`: The position of the byte to retrieve within the entry.
    ///
    /// # Returns
    /// The k-th byte of the entry at index `i` as an `i32`, or `-1` if the entry's length is less than or equal to `k`.
    ///
    /// # Note
    /// In Rust, this method might return a signed integer (`i32`) to accommodate the `-1` case, which differs from Java's default integer handling.
    fn byte_at(&self, i: i32, k: i32) -> i32;
}

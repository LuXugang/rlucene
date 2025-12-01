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
use crate::core::index::BytesRef;
use crate::core::util::BytesRefComparator;
use crate::core::util::error::lucene_error::Result;

pub trait SortableBytesRefArray<'a> {
    /// Append a new value
    fn append(&mut self, bytes: &BytesRef<Vec<u8>>) -> Result<i32>;
    /// Clear all previously stored values
    fn clear(&mut self);
    /// Returns the number of values appended so far
    fn size(&self) -> i32;
    /// Sort all values by the provided comparator and return an iterator over
    /// the sorted values  */
    type Iter;
    fn iterator(&'a self, comp: impl BytesRefComparator) -> Result<Self::Iter>;
}

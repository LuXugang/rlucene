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
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::util::error::lucene_error::Result;
/// A list of per-document numeric values, sorted according to i64's cmp.
pub trait SortedNumericDocValues: DocValuesIterator {
    /// Iterates to the next value in the current document. Do not call this more than
    /// [`doc_value_count`](SortedNumericDocValues::doc_value_count) times for the document.
    fn next_value(&mut self) -> Result<i64>;

    /// Retrieves the number of values for the current document. This must always be greater than zero.
    /// It is illegal to call this method after [`advance_exact(int)`](DocValuesIterator::advance_exact) returned `false`.
    fn doc_value_count(&mut self) -> Result<i32>;
}

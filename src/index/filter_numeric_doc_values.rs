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
use crate::index::numeric_doc_values::NumericDocValues;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::Result;
/// Delegates all methods to a wrapped [`NumericDocValues`].
pub struct FilterNumericDocValues<N> {
    inner: N,
}
impl<N> FilterNumericDocValues<N>
where
    N: NumericDocValues,
{
    pub fn new(inner: N) -> Self {
        FilterNumericDocValues { inner }
    }
}

impl<N> DocValuesIterator for FilterNumericDocValues<N>
where
    N: NumericDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.inner.advance_exact(target)
    }
}

impl<N> DocIdSetIterator for FilterNumericDocValues<N>
where
    N: NumericDocValues,
{
    fn doc_id(&self) -> i32 {
        self.inner.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.inner.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.inner.advance(target)
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        self.inner.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.inner.cost()
    }
}

impl<N> NumericDocValues for FilterNumericDocValues<N>
where
    N: NumericDocValues,
{
    fn long_value(&mut self) -> Result<i64> {
        self.inner.long_value()
    }
}

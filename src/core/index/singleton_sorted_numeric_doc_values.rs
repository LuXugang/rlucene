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
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;

/// Exposes a multi-valued view over a single-valued instance.
///
/// This can be used if you want to have one multi-valued implementation that
/// works for both single-valued and multi-valued types.
pub struct SingletonSortedNumericDocValues<N>
where
    N: NumericDocValues,
{
    inner: Option<N>,
}

impl<N> SingletonSortedNumericDocValues<N>
where
    N: NumericDocValues,
{
    pub fn new(inner: N) -> Result<Self> {
        if inner.doc_id() != -1 {
            return Err(LuceneError::illegal_state(format!(
                "iterator has already been used: docID={}",
                inner.doc_id()
            )));
        }
        Ok(Self { inner: Some(inner) })
    }
}

impl<N> DocIdSetIterator for SingletonSortedNumericDocValues<N>
where
    N: NumericDocValues,
{
    fn doc_id(&self) -> i32 {
        self.inner.as_ref().unwrap().doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.inner.as_mut().unwrap().next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.inner.as_mut().unwrap().advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.inner.as_ref().unwrap().cost()
    }
}

impl<N> DocValuesIterator for SingletonSortedNumericDocValues<N>
where
    N: NumericDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.inner.as_mut().unwrap().advance_exact(target)
    }
}

impl<N> SortedNumericDocValues for SingletonSortedNumericDocValues<N>
where
    N: NumericDocValues,
{
    fn next_value(&mut self) -> Result<i64> {
        self.inner.as_mut().unwrap().long_value()
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        Ok(1)
    }

    fn is_single_valued(&self) -> bool {
        true
    }

    type NumericDocValues = N;

    fn get_numeric_doc_values(&mut self) -> Result<Option<Self::NumericDocValues>> {
        if self.inner.as_ref().unwrap().doc_id() != -1 {
            return Err(LuceneError::illegal_state(format!(
                "iterator has already been used: docID={}",
                self.inner.as_ref().unwrap().doc_id()
            )));
        }
        Ok(self.inner.take())
    }
}

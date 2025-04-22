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
use crate::codecs::doc_values_enum::doc_values::NumericDocValuesEnum;
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::IndexInput;
use crate::util::access::AccessVec;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use std::cell::RefCell;
use std::rc::Rc;

/// Exposes a multi-valued view over a single-valued instance.
///
/// This can be used if you want to have one multi-valued implementation that works for both
/// single-valued and multi-valued types.
pub struct SingletonSortedNumericDocValues<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    inner: Rc<RefCell<NumericDocValuesEnum<I, AV>>>,
}

impl<I, AV> SingletonSortedNumericDocValues<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    pub fn new(inner: NumericDocValuesEnum<I, AV>) -> Result<Self> {
        if inner.doc_id() != -1 {
            return Err(LuceneError::illegal_state(format!(
                "iterator has already been used: docID={}",
                inner.doc_id()
            )));
        }
        Ok(Self {
            inner: Rc::new(RefCell::new(inner)),
        })
    }

    pub fn get_numeric_doc_values(&self) -> Result<Rc<RefCell<NumericDocValuesEnum<I, AV>>>> {
        if self.inner.borrow().doc_id() != -1 {
            return Err(LuceneError::illegal_state(format!(
                "iterator has already been used: docID={}",
                self.inner.borrow().doc_id()
            )));
        }
        Ok(self.inner.clone())
    }
}

impl<I, AV> DocIdSetIterator for SingletonSortedNumericDocValues<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    fn doc_id(&self) -> i32 {
        self.inner.borrow().doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.inner.borrow_mut().next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.inner.borrow_mut().advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.inner.borrow().cost()
    }
}

impl<I, AV> DocValuesIterator for SingletonSortedNumericDocValues<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.inner.borrow_mut().advance_exact(target)
    }
}

impl<I, AV> SortedNumericDocValues for SingletonSortedNumericDocValues<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    fn next_value(&mut self) -> Result<i64> {
        self.inner.borrow_mut().long_value()
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        Ok(1)
    }
}

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
use crate::codecs::doc_values_enum::doc_values::{
    NumericDocValuesEnum, SortedDocValuesEnum, SortedNumericDocValuesEnum, SortedSetDocValuesEnum,
};
use crate::index::binary_doc_values::BinaryDocValues;
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
use crate::index::singleton_sorted_set_doc_values::SingletonSortedSetDocValues;
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::search::doc_id_set_iterator::doc_id_set_iterator_static::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::IndexInput;
use crate::util::error::lucene_error::Result;
use std::cell::RefCell;
use std::rc::Rc;

pub struct DocValues;
impl DocValues {
    /// Returns a multi-valued view over the provided NumericDocValues.
    pub fn singleton_numeric<I>(
        dv: NumericDocValuesEnum<I>,
    ) -> Result<SortedNumericDocValuesEnum<I>>
    where
        I: IndexInput,
    {
        Ok(SortedNumericDocValuesEnum::Singleton(
            SingletonSortedNumericDocValues::new(dv)?,
        ))
    }
    /// Returns a multi-valued view over the provided SortedDocValues.
    pub fn singleton_sorted<I>(dv: SortedDocValuesEnum<I>) -> Result<SortedSetDocValuesEnum<I>>
    where
        I: IndexInput,
    {
        Ok(SortedSetDocValuesEnum::Singleton(
            SingletonSortedSetDocValues::new(dv)?,
        ))
    }

    pub fn unwrap_singleton_sorted_set_doc_values<I>(
        dv: impl SortedSetDocValues<I>,
    ) -> Result<Rc<RefCell<SortedDocValuesEnum<I>>>>
    where
        I: IndexInput,
    {
        dv.unwrap_singleton()
    }
}

pub struct EmptyBinary {
    doc: i32,
}
impl Default for EmptyBinary {
    fn default() -> Self {
        Self::new()
    }
}
impl EmptyBinary {
    pub fn new() -> Self {
        Self { doc: -1 }
    }
}

impl DocIdSetIterator for EmptyBinary {
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.doc = NO_MORE_DOCS;
        Ok(self.doc)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        self.doc = NO_MORE_DOCS;
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(0)
    }
}

impl DocValuesIterator for EmptyBinary {
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.doc = target;
        Ok(false)
    }
}
impl BinaryDocValues for EmptyBinary {}

pub struct EmptyNumeric {
    doc: i32,
}
impl Default for EmptyNumeric {
    fn default() -> Self {
        Self::new()
    }
}

impl EmptyNumeric {
    pub fn new() -> Self {
        Self { doc: -1 }
    }
}

impl DocValuesIterator for EmptyNumeric {}

impl DocIdSetIterator for EmptyNumeric {
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.doc = NO_MORE_DOCS;
        Ok(self.doc)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.doc = target;
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(0)
    }
}

impl NumericDocValues for EmptyNumeric {
    fn long_value(&mut self) -> Result<i64> {
        debug_assert!(false);
        Ok(0)
    }
}

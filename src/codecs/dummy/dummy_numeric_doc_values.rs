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
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;

pub struct DummyNumericDocValues;

impl DocValuesIterator for DummyNumericDocValues {
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        Err(LuceneError::not_implemented(
            "this method should never be called",
        ))
    }
}

impl DocIdSetIterator for DummyNumericDocValues {
    fn doc_id(&self) -> i32 {
        -1
    }

    fn next_doc(&mut self) -> Result<i32> {
        Err(LuceneError::not_implemented(
            "this method should never be called",
        ))
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::not_implemented(
            "this method should never be called",
        ))
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        Err(LuceneError::not_implemented(
            "this method should never be called",
        ))
    }

    fn cost(&self) -> Result<i64> {
        Err(LuceneError::not_implemented(
            "this method should never be called",
        ))
    }
}

impl NumericDocValues for DummyNumericDocValues {
    fn long_value(&mut self) -> Result<i64> {
        Err(LuceneError::not_implemented(
            "this method should never be called",
        ))
    }
}

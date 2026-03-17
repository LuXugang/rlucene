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
use crate::core::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error::Result;
pub struct DummySortedNumericDocValues;

impl DocValuesIterator for DummySortedNumericDocValues {
  fn advance_exact(&mut self, _target: i32) -> Result<bool> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }
}

impl DocIdSetIterator for DummySortedNumericDocValues {
  fn doc_id(&self) -> i32 {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  fn next_doc(&mut self) -> Result<i32> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  fn slow_advance(&mut self, _target: i32) -> Result<i32> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  fn cost(&self) -> Result<i64> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }
}

impl SortedNumericDocValues for DummySortedNumericDocValues {
  fn next_value(&mut self) -> Result<i64> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  fn doc_value_count(&mut self) -> Result<i32> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  fn is_single_valued(&self) -> bool {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  type NumericDocValues = DummyNumericDocValues;

  fn get_numeric_doc_values(&mut self) -> Result<Self::NumericDocValues> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }
}

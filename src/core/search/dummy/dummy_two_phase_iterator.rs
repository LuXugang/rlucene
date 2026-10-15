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
use crate::core::search::approximation_disi::ApproximationBitSetAccess;
use crate::core::search::approximation_disi::ApproximationDISI;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::Result;

pub struct DummyTwoPhaseIterator;

impl ApproximationDISI for DummyTwoPhaseIterator {
  fn approx_doc_id(&self) -> i32 {
    dummy_unreachable!()
  }

  fn approx_next_doc(&mut self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn approx_advance(&mut self, _target: i32) -> Result<i32> {
    dummy_unreachable!()
  }

  fn approx_slow_advance(&mut self, _target: i32) -> Result<i32> {
    dummy_unreachable!()
  }

  fn approx_cost(&self) -> Result<i64> {
    dummy_unreachable!()
  }
}

impl ApproximationBitSetAccess for DummyTwoPhaseIterator {
  fn approx_is_bit_iter(&self) -> bool {
    dummy_unreachable!()
  }

  fn approx_get(&self, _index: usize) -> Result<bool> {
    dummy_unreachable!()
  }

  fn approx_set_doc_id(&mut self, _doc: i32) -> Result<()> {
    dummy_unreachable!()
  }

  fn approx_bit_set_length(&self) -> Result<usize> {
    dummy_unreachable!()
  }
}

impl TwoPhaseIterator for DummyTwoPhaseIterator {
  fn matches(&mut self) -> Result<bool> {
    dummy_unreachable!()
  }

  fn match_cost(&self) -> f32 {
    dummy_unreachable!()
  }
}

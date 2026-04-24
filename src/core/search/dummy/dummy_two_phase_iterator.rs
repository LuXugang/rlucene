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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::Result;

pub struct DummyTwoPhaseIterator;

impl TwoPhaseIterator for DummyTwoPhaseIterator {
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    dummy_unreachable!()
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    dummy_unreachable!()
  }

  fn matches(&mut self) -> Result<bool> {
    dummy_unreachable!()
  }

  fn match_cost(&self) -> f32 {
    dummy_unreachable!()
  }
}

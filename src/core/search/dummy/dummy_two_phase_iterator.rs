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
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::Result;

pub struct DummyTwoPhaseIterator;

impl TwoPhaseIterator for DummyTwoPhaseIterator {
    type DocIdSetIterator = DummyDISI;

    type DocIdSetIteratorRef<'a>
        = &'a DummyDISI
    where
        Self: 'a;

    type DocIdSetIteratorMut<'a>
        = &'a mut DummyDISI
    where
        Self: 'a;

    fn approximation_mut(&mut self) -> Result<Self::DocIdSetIteratorMut<'_>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn approximation(&self) -> Result<Self::DocIdSetIteratorRef<'_>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn set_empty(&mut self) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn matches(&mut self) -> Result<bool> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn match_cost(&self) -> f32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

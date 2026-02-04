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
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

pub struct DummyLeafCollector;

impl Display for DummyLeafCollector {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl LeafCollector for DummyLeafCollector {
    fn set_scorer(&mut self, _scorer: &mut dyn Scorable) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn collect(&mut self, _doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type DocIdSetIteratorRef<'a>
        = DummyDISI
    where
        Self: 'a;

    fn competitive_iterator(&mut self) -> Result<Option<Self::DocIdSetIteratorRef<'_>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn finish(&mut self) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

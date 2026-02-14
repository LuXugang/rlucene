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
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::util::error::lucene_error::Result;

pub struct DummyScorable;

impl Scorable for DummyScorable {
    fn score(&mut self) -> Result<f32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn smoothing_score(&mut self, _doc_id: i32) -> Result<f32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn set_min_competitive_score(&mut self, _min_score: f32) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_children(&self) -> Result<Vec<ChildScorable<Box<dyn Scorable>>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn cost(&self) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

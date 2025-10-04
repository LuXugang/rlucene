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
use crate::core::search::score_doc::ScoreDocLike;
use std::fmt::{Display, Formatter};

#[derive(Clone, Default)]
pub struct DummyScoreDocLike;

impl Display for DummyScoreDocLike {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl ScoreDocLike for DummyScoreDocLike {
    fn doc(&self) -> i32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn score(&self) -> f32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn shard_index(&self) -> i32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
